//! OpenAI Provider — 真实 HTTP 实现（Sub-Step 10.1.2）
//!
//! 通过 `reqwest` 调用 OpenAI `/v1/chat/completions` 接口，
//! 实现 [`LlmProvider`] trait 的三个方法：
//! - `complete`: 普通 completion，解析 `choices[0].message.content`
//! - `stream_complete`: SSE 流式 completion，逐 token 通过 mpsc channel 推送
//! - `structured_complete`: 原生 `response_format: json_schema` 调用（优于默认 JSON repair）
//!
//! # 错误处理
//! - 401 → `Error::llm("OpenAI API key 无效 (HTTP 401)")`
//! - 429 → `Error::llm("OpenAI rate limit / 限流 (HTTP 429)")`
//! - 500 → `Error::llm("OpenAI server error / 服务器错误 (HTTP 500)")`
//! - 其他非 2xx → `Error::llm(format!("OpenAI API 错误: HTTP {status}"))`
//!
//! # 用法
//! ```no_run
//! # use sparkfox_llm::{LlmProvider, OpenAIProvider};
//! # async fn demo() -> sparkfox_core::Result<()> {
//! // 默认 base_url = "https://api.openai.com"
//! let provider = OpenAIProvider::new("sk-xxx", "gpt-4");
//! let text = provider.complete("你好").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 复用 LlmAuditLogger（S-01 P0 修复）
//! OpenAIProvider 本身不直接记录审计日志 — 这遵循 v1.0.0 装饰器模式：
//! 业务层用 [`AuditedProvider`](crate::AuditedProvider) 包装 OpenAIProvider，
//! 由包装层统一记录每次调用的审计日志（含 model / token_count / status）。
//!
//! ```no_run
//! # use std::sync::Arc;
//! # use sparkfox_llm::{AuditedProvider, LlmProvider, OpenAIProvider};
//! # use sparkfox_security::LlmAuditLogger;
//! # async fn demo() -> sparkfox_core::Result<()> {
//! # let logger = Arc::new(LlmAuditLogger::from_conn(rusqlite::Connection::open_in_memory()?).await?);
//! let inner = OpenAIProvider::new("sk-xxx", "gpt-4");
//! let provider = AuditedProvider::new(inner, logger, "openai".into(), "gpt-4".into());
//! let _text = provider.complete("hello").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 测试方式
//! 集成测试见 `tests/openai_provider_test.rs`，使用 `wiremock` 起本地 mock 服务，
//! 模拟 OpenAI API 响应验证三个方法及错误处理。

#![forbid(unsafe_code)]

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;

use sparkfox_core::{Error, Result};

use crate::provider::{LlmProvider, LlmStream};

/// OpenAI 默认 API base URL（不含 `/v1` 后缀，由 [`OpenAIProvider::chat_completions_url`] 拼接）
const DEFAULT_BASE_URL: &str = "https://api.openai.com";

/// OpenAI Provider — 通过 HTTP 调用 OpenAI chat completions API
///
/// # 字段
/// - `api_key`: OpenAI API Key（以 `sk-` 开头）
/// - `model`: 模型名（如 `gpt-4` / `gpt-4-turbo` / `gpt-3.5-turbo`）
/// - `base_url`: API 基础 URL（默认 `https://api.openai.com`，测试时可指向 mock 服务）
///
/// # 兼容性
/// - `new(api_key, model)` 保持与 v1.0.0 占位 API 兼容（使用默认 base_url）
/// - `with_base_url(api_key, model, base_url)` 用于测试或自定义部署（如 Azure OpenAI）
pub struct OpenAIProvider {
    /// API Key（从环境变量或配置文件加载）
    pub api_key: String,
    /// 模型名（如 "gpt-4" / "gpt-4-turbo"）
    pub model: String,
    /// API 基础 URL（不含 `/v1` 后缀）
    base_url: String,
}

impl OpenAIProvider {
    /// 创建 OpenAI Provider（使用默认 base_url = `https://api.openai.com`）
    ///
    /// 保持与 v1.0.0 占位 API 兼容。
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 创建 OpenAI Provider，指定 base_url（用于测试或自定义部署）
    ///
    /// # 参数
    /// - `api_key`: API Key
    /// - `model`: 模型名
    /// - `base_url`: API 基础 URL（不含 `/v1` 后缀，如 `http://127.0.0.1:8080`）
    pub fn with_base_url(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            base_url: base_url.into(),
        }
    }

    /// 构造 `/v1/chat/completions` 完整 URL
    fn chat_completions_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{base}/v1/chat/completions")
    }

    /// 构造带 Bearer token 的请求 builder（GET 类基础请求）
    fn request_builder(
        &self,
        client: &reqwest::Client,
        body: serde_json::Value,
    ) -> reqwest::RequestBuilder {
        client
            .post(self.chat_completions_url())
            .bearer_auth(&self.api_key)
            .json(&body)
    }
}

// ============================================================================
// 响应数据结构（仅解析必要字段，其余字段忽略）
// ============================================================================

/// OpenAI 非流式 chat completion 响应
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

/// OpenAI 流式 chat completion chunk
#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChatChunkChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChunkChoice {
    delta: ChatDelta,
}

#[derive(Debug, Deserialize)]
struct ChatDelta {
    content: Option<String>,
}

// ============================================================================
// LlmProvider 实现
// ============================================================================

#[async_trait]
impl LlmProvider for OpenAIProvider {
    /// 普通 completion — POST `/v1/chat/completions`，解析 `choices[0].message.content`
    async fn complete(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        let resp = self.request_builder(&client, body).send().await
            .map_err(|e| Error::llm(format!("OpenAI HTTP 请求失败: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_http_error(status).await);
        }

        let parsed: ChatCompletionResponse = resp
            .json()
            .await
            .map_err(|e| Error::llm(format!("OpenAI 响应 JSON 解析失败: {e}")))?;

        let content = parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| Error::llm("OpenAI 响应缺少 choices[0].message.content"))?;

        Ok(content)
    }

    /// 流式 completion — POST with `stream: true`，逐 token 通过 mpsc channel 推送
    async fn stream_complete(&self, prompt: &str) -> Result<LlmStream> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "stream": true
        });

        let resp = self.request_builder(&client, body).send().await
            .map_err(|e| Error::llm(format!("OpenAI HTTP 请求失败: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_http_error(status).await);
        }

        // 创建 mpsc channel，后台 task 消费 SSE 流逐 token 推送
        // LlmStream::channel 返回 (Sender, LlmStream)，LlmStream 已封装 rx
        let (tx, stream) = LlmStream::channel(32);

        tokio::spawn(async move {
            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        // 按行处理 SSE 事件
                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].trim().to_string();
                            buffer = buffer[newline_pos + 1..].to_string();

                            // 只处理 `data: ` 前缀的行
                            let Some(payload) = line.strip_prefix("data: ") else {
                                continue;
                            };

                            // [DONE] 标记流结束
                            if payload == "[DONE]" {
                                return; // drop tx → 流结束
                            }

                            // 解析 JSON chunk
                            let Ok(chunk) =
                                serde_json::from_str::<ChatCompletionChunk>(payload)
                            else {
                                continue; // 跳过无法解析的 chunk
                            };

                            // 提取 delta.content，跳过空内容
                            if let Some(content) = chunk
                                .choices
                                .into_iter()
                                .next()
                                .and_then(|c| c.delta.content)
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
                                "OpenAI 流式读取失败: {e}"
                            ))))
                            .await;
                        return;
                    }
                }
            }
            // stream 自然结束（无 [DONE] 标记也视为结束）
            // tx drop 后消费方 next() 返回 None
        });

        Ok(stream)
    }

    /// 结构化输出 — 原生 `response_format: json_schema` 调用（优于默认 JSON repair 实现）
    ///
    /// 重写默认实现，直接走 OpenAI 原生 structured output 通道，避免文本解析不稳定。
    async fn structured_complete(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "sparkfox_structured_output",
                    "schema": schema,
                    "strict": false
                }
            }
        });

        let resp = self.request_builder(&client, body).send().await
            .map_err(|e| Error::llm(format!("OpenAI HTTP 请求失败: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_http_error(status).await);
        }

        let parsed: ChatCompletionResponse = resp
            .json()
            .await
            .map_err(|e| Error::llm(format!("OpenAI 响应 JSON 解析失败: {e}")))?;

        let content = parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| Error::llm("OpenAI structured 响应缺少 choices[0].message.content"))?;

        // 内容应为符合 schema 的 JSON 字符串
        serde_json::from_str::<serde_json::Value>(&content)
            .map_err(|e| Error::llm(format!("OpenAI structured 响应内容不是合法 JSON: {e}")))
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 将 HTTP 错误状态码映射为 `Error::Llm`，覆盖 401 / 429 / 500 等常见错误
async fn map_http_error(status: reqwest::StatusCode) -> Error {
    let code = status.as_u16();
    match code {
        401 => Error::llm(format!(
            "OpenAI API key 无效 (HTTP {code}) — 请检查 API Key 配置"
        )),
        429 => Error::llm(format!(
            "OpenAI rate limit / 限流 (HTTP {code}) — 请稍后重试"
        )),
        500..=599 => Error::llm(format!(
            "OpenAI server error / 服务器错误 (HTTP {code})"
        )),
        _ => Error::llm(format!("OpenAI API 错误: HTTP {code}")),
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
        let p = OpenAIProvider::new("sk-test", "gpt-4");
        assert_eq!(p.api_key, "sk-test");
        assert_eq!(p.model, "gpt-4");
        assert_eq!(p.base_url, DEFAULT_BASE_URL);
    }

    /// 验证 `with_base_url()` 构造器
    #[test]
    fn test_with_base_url() {
        let p = OpenAIProvider::with_base_url("sk-test", "gpt-4", "http://localhost:8080");
        assert_eq!(p.base_url, "http://localhost:8080");
    }

    /// 验证 URL 拼接逻辑（含 trailing slash 处理）
    #[test]
    fn test_chat_completions_url() {
        let p1 = OpenAIProvider::new("sk", "gpt-4");
        assert_eq!(p1.chat_completions_url(), "https://api.openai.com/v1/chat/completions");

        let p2 = OpenAIProvider::with_base_url("sk", "gpt-4", "http://localhost:8080/");
        assert_eq!(p2.chat_completions_url(), "http://localhost:8080/v1/chat/completions");
    }

    /// 验证 401 错误映射
    #[tokio::test]
    async fn test_map_http_error_401() {
        let err = map_http_error(reqwest::StatusCode::UNAUTHORIZED).await;
        let msg = format!("{err}");
        assert!(msg.contains("API key") || msg.contains("401"));
    }

    /// 验证 429 错误映射
    #[tokio::test]
    async fn test_map_http_error_429() {
        let err = map_http_error(reqwest::StatusCode::TOO_MANY_REQUESTS).await;
        let msg = format!("{err}");
        assert!(msg.contains("rate limit") || msg.contains("429") || msg.contains("限流"));
    }

    /// 验证 500 错误映射
    #[tokio::test]
    async fn test_map_http_error_500() {
        let err = map_http_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR).await;
        let msg = format!("{err}");
        assert!(msg.contains("server error") || msg.contains("500") || msg.contains("服务器"));
    }
}
