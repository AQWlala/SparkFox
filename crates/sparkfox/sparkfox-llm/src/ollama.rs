//! Ollama Provider — 本地离线 LLM 实现（Sub-Step 10.1.4）
//!
//! 通过 `reqwest` 调用 Ollama `/api/chat` 接口，作为 RISK-v1.1-01 的离线兜底方案。
//! 实现 [`LlmProvider`] trait 的三个方法：
//! - `complete`: 非流式 completion，解析 `message.content`
//! - `stream_complete`: NDJSON 流式 completion，逐 `message.content` 增量通过 mpsc 推送
//! - `structured_complete`: 通过 `format: <schema>` 字段约束 JSON 输出（Ollama 原生 JSON schema）
//!
//! # 错误处理
//! - 连接拒绝 → `Error::llm("Ollama 服务未启动...")`
//! - HTTP 500 → `Error::llm("Ollama server error...")`
//! - 其他非 2xx → `Error::llm(format!("Ollama API 错误: HTTP {status}"))`
//!
//! # 用法
//! ```no_run
//! # use sparkfox_llm::{LlmProvider, OllamaProvider};
//! # async fn demo() -> sparkfox_core::Result<()> {
//! // 默认 base_url = "http://localhost:11434"
//! let provider = OllamaProvider::new("llama3");
//! let text = provider.complete("你好").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 离线兜底定位（RISK-v1.1-01）
//! Ollama 作为本地推理后端，在云端 LLM 不可用（网络故障 / API key 失效 / 限流）时
//! 提供降级能力。本地模型 token 不计费、数据不外泄，适合处理隐私敏感文档。
//!
//! # 测试方式
//! 集成测试见 `tests/ollama_provider_test.rs`，使用 `wiremock` 起本地 mock 服务，
//! 模拟 Ollama API 响应验证三个方法。

#![forbid(unsafe_code)]

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;

use sparkfox_core::{Error, Result};

use crate::provider::{LlmProvider, LlmStream};

/// Ollama 默认 base URL（本地 11434 端口）
const DEFAULT_BASE_URL: &str = "http://localhost:11434";

/// Ollama Provider — 通过 HTTP 调用本地 Ollama `/api/chat` API
///
/// # 字段
/// - `model`: 模型名（如 `llama3` / `qwen2.5` / `mistral`）
/// - `base_url`: Ollama 服务地址（默认 `http://localhost:11434`，可指向远程实例）
///
/// # 兼容性
/// - `new(model)` 使用默认 base_url，最常见的本地部署形态
/// - `with_base_url(model, base_url)` 用于测试或自定义部署
///
/// # 离线兜底（RISK-v1.1-01）
/// 作为云端 LLM 不可用时的降级方案，本地推理无 token 计费 / 数据外泄风险。
pub struct OllamaProvider {
    /// 模型名（如 "llama3" / "qwen2.5"）
    pub model: String,
    /// Ollama 服务基础 URL（不含 `/api` 后缀）
    base_url: String,
}

impl OllamaProvider {
    /// 创建 Ollama Provider（使用默认 base_url = `http://localhost:11434`）
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 创建 Ollama Provider，指定 base_url（用于测试或自定义部署）
    ///
    /// # 参数
    /// - `model`: 模型名（如 "llama3"）
    /// - `base_url`: Ollama 服务地址（不含 `/api` 后缀，如 `http://127.0.0.1:11434`）
    pub fn with_base_url(model: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            base_url: base_url.into(),
        }
    }

    /// 返回 base_url（用于测试断言默认值）
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// 构造 `/api/chat` 完整 URL
    fn chat_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{base}/api/chat")
    }

    /// 构造 Ollama `/api/chat` 请求体（非流式 / 流式通用）
    ///
    /// # 参数
    /// - `prompt`: 用户输入
    /// - `stream`: 是否启用流式（true → NDJSON，false → 单 JSON）
    fn build_chat_body(prompt: &str, model: &str, stream: bool) -> serde_json::Value {
        serde_json::json!({
            "model": model,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "stream": stream
        })
    }

    /// 构造 Ollama structured 请求体（含 `format` 字段约束 JSON schema）
    fn build_structured_body(
        prompt: &str,
        model: &str,
        schema: &serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "model": model,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "stream": false,
            "format": schema
        })
    }
}

// ============================================================================
// 响应数据结构（仅解析必要字段，其余字段忽略）
// ============================================================================

/// Ollama 非流式 `/api/chat` 响应
#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: Option<ChatMessage>,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

/// Ollama 流式 `/api/chat` NDJSON 单行响应
#[derive(Debug, Deserialize)]
struct StreamChunk {
    message: Option<ChatMessage>,
    #[allow(dead_code)]
    done: Option<bool>,
}

// ============================================================================
// LlmProvider 实现
// ============================================================================

#[async_trait]
impl LlmProvider for OllamaProvider {
    /// 普通 completion — POST `/api/chat`，解析 `message.content`
    async fn complete(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let body = Self::build_chat_body(prompt, &self.model, false);

        let resp = client
            .post(self.chat_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| map_request_error(&e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_http_error(status).await);
        }

        let parsed: ChatResponse = resp
            .json()
            .await
            .map_err(|e| Error::llm(format!("Ollama 响应 JSON 解析失败: {e}")))?;

        let content = parsed
            .message
            .and_then(|m| m.content)
            .ok_or_else(|| Error::llm("Ollama 响应缺少 message.content"))?;

        Ok(content)
    }

    /// 流式 completion — POST with `stream: true`，逐 `message.content` 增量通过 mpsc 推送
    ///
    /// Ollama 流式响应为 NDJSON 格式（每行一个 JSON 对象），
    /// 最后一行通常为 `{"done":true}`（无 `message` 字段或 `content` 为空）。
    async fn stream_complete(&self, prompt: &str) -> Result<LlmStream> {
        let client = reqwest::Client::new();
        let body = Self::build_chat_body(prompt, &self.model, true);

        let resp = client
            .post(self.chat_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| map_request_error(&e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_http_error(status).await);
        }

        // 创建 mpsc channel，后台 task 消费 NDJSON 流逐 content 增量推送
        let (tx, stream) = LlmStream::channel(32);

        tokio::spawn(async move {
            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        // 按行处理 NDJSON（每行一个独立 JSON 对象）
                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].trim().to_string();
                            buffer = buffer[newline_pos + 1..].to_string();

                            // 跳过空行（NDJSON 末尾可能含空行）
                            if line.is_empty() {
                                continue;
                            }

                            // 解析 JSON chunk（解析失败的行直接跳过，避免格式噪声中断流）
                            let Ok(chunk) =
                                serde_json::from_str::<StreamChunk>(&line)
                            else {
                                continue;
                            };

                            // 提取 message.content，跳过空内容（done 行通常无 content）
                            if let Some(content) = chunk
                                .message
                                .and_then(|m| m.content)
                            {
                                if !content.is_empty() {
                                    // 发送失败说明消费方已 drop，提前结束
                                    if tx.send(Ok(content)).await.is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(Error::llm(format!(
                                "Ollama 流式读取失败: {e}"
                            ))))
                            .await;
                        return;
                    }
                }
            }
            // NDJSON 流自然结束（tx drop 后消费方 next() 返回 None）
        });

        Ok(stream)
    }

    /// 结构化输出 — 通过 `format: <schema>` 字段约束 JSON（Ollama 原生 JSON schema）
    ///
    /// 重写默认实现，直接走 Ollama 原生 structured output 通道，避免文本解析不稳定。
    /// Ollama 将 JSON 作为字符串放进 `message.content` 返回。
    async fn structured_complete(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let body = Self::build_structured_body(prompt, &self.model, schema);

        let resp = client
            .post(self.chat_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| map_request_error(&e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_http_error(status).await);
        }

        let parsed: ChatResponse = resp
            .json()
            .await
            .map_err(|e| Error::llm(format!("Ollama 响应 JSON 解析失败: {e}")))?;

        let content = parsed
            .message
            .and_then(|m| m.content)
            .ok_or_else(|| Error::llm("Ollama structured 响应缺少 message.content"))?;

        // 内容应为符合 schema 的 JSON 字符串
        serde_json::from_str::<serde_json::Value>(&content)
            .map_err(|e| Error::llm(format!("Ollama structured 响应内容不是合法 JSON: {e}")))
    }
}

// ============================================================================
// 辅助函数 — 错误映射
// ============================================================================

/// 将 HTTP 请求错误映射为 `Error::Llm`，特别识别 connection refused
fn map_request_error(e: &reqwest::Error) -> Error {
    // 连接拒绝 → 提示用户启动 Ollama 服务
    if e.is_connect() {
        return Error::llm(format!(
            "Ollama 服务未启动 (connection refused) — 请确认 Ollama 已在 http://localhost:11434 启动: {e}"
        ));
    }
    Error::llm(format!("Ollama HTTP 请求失败: {e}"))
}

/// 将 HTTP 错误状态码映射为 `Error::Llm`，覆盖 500 等常见错误
async fn map_http_error(status: reqwest::StatusCode) -> Error {
    let code = status.as_u16();
    match code {
        500..=599 => Error::llm(format!(
            "Ollama server error / 服务器错误 (HTTP {code})"
        )),
        _ => Error::llm(format!("Ollama API 错误: HTTP {code}")),
    }
}

// ============================================================================
// 单元测试 — 与 provider.rs 集成测试互补，覆盖纯逻辑分支
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 `new()` 构造器使用默认 base_url
    #[test]
    fn test_new_uses_default_base_url() {
        let p = OllamaProvider::new("llama3");
        assert_eq!(p.model, "llama3");
        assert_eq!(p.base_url(), DEFAULT_BASE_URL);
    }

    /// 验证 `with_base_url()` 构造器
    #[test]
    fn test_with_base_url() {
        let p = OllamaProvider::with_base_url("llama3", "http://localhost:11434");
        assert_eq!(p.base_url(), "http://localhost:11434");
    }

    /// 验证 URL 拼接逻辑（含 trailing slash 处理）
    #[test]
    fn test_chat_url() {
        let p1 = OllamaProvider::new("llama3");
        assert_eq!(p1.chat_url(), "http://localhost:11434/api/chat");

        let p2 = OllamaProvider::with_base_url("llama3", "http://localhost:11434/");
        assert_eq!(p2.chat_url(), "http://localhost:11434/api/chat");
    }

    /// 验证 500 错误映射
    #[tokio::test]
    async fn test_map_http_error_500() {
        let err = map_http_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR).await;
        let msg = format!("{err}");
        assert!(msg.contains("server error") || msg.contains("500") || msg.contains("服务器"));
    }
}
