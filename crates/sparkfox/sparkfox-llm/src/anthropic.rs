//! Anthropic Provider — 真实 HTTP 实现（Sub-Step 10.1.3）
//!
//! 通过 `reqwest` 调用 Anthropic `/v1/messages` 接口（Claude 系列模型），
//! 实现 [`LlmProvider`] trait 的三个方法：
//! - `complete`: 普通 completion，解析 `content[0].text`
//! - `stream_complete`: SSE 流式 completion，解析 `content_block_delta` 事件的 `delta.text`
//! - `structured_complete`: 通过 `tool_use` 强制 JSON schema（重写默认实现）
//!
//! # Anthropic 与 OpenAI 的关键差异
//! | 维度 | OpenAI | Anthropic |
//! |------|--------|-----------|
//! | 鉴权 | `Authorization: Bearer <key>` | `x-api-key: <key>` + `anthropic-version: 2023-06-01` |
//! | 端点 | `POST /v1/chat/completions` | `POST /v1/messages` |
//! | 非流式响应 | `choices[0].message.content` (string) | `content[0].text` (数组中 block 的 text 字段) |
//! | 流式格式 | `data: {json}\n\n` | `event: <type>\ndata: {json}\n\n` (含 event 行) |
//! | 结构化输出 | `response_format: {type: "json_schema", json_schema: {...}}` | `tools: [{name, input_schema}]` + `tool_choice: {type: "tool", name}` |
//!
//! # tool_use 强制 JSON 机制
//! Anthropic 不支持 OpenAI 风格的 `response_format: json_schema`，但可通过 tool_use 实现
//! 等价效果：
//! 1. 在请求中定义一个名为 `extract` 的 tool，其 `input_schema` 即为目标 JSON Schema
//! 2. 设置 `tool_choice: {type: "tool", name: "extract"}` 强制模型必须调用此 tool
//! 3. 模型返回的 `content[*]` 中将含 `type: "tool_use"` block，其 `input` 字段即为符合
//!    schema 的 JSON Value
//!
//! # 错误处理
//! - 401 → `Error::llm("Anthropic API key 无效 (HTTP 401)")`
//! - 429 → `Error::llm("Anthropic rate limit / 限流 (HTTP 429)")`
//! - 500 → `Error::llm("Anthropic server error / 服务器错误 (HTTP 5xx)")`
//! - 其他非 2xx → `Error::llm(format!("Anthropic API 错误: HTTP {status}"))`
//!
//! # 用法
//! ```no_run
//! # use sparkfox_llm::{AnthropicProvider, LlmProvider};
//! # async fn demo() -> sparkfox_core::Result<()> {
//! // 默认 base_url = "https://api.anthropic.com"
//! let provider = AnthropicProvider::new("sk-ant-xxx", "claude-3-5-sonnet-20240620");
//! let text = provider.complete("你好").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 复用 LlmAuditLogger（S-01 P0 修复）
//! AnthropicProvider 本身不直接记录审计日志 — 遵循 v1.0.0 装饰器模式：
//! 业务层用 [`AuditedProvider`](crate::AuditedProvider) 包装 AnthropicProvider，
//! 由包装层统一记录每次调用的审计日志。
//!
//! # 测试方式
//! 集成测试见 `tests/anthropic_provider_test.rs`，使用 `wiremock` 起本地 mock 服务，
//! 模拟 Anthropic API 响应验证三个方法。

#![forbid(unsafe_code)]

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use std::sync::Arc;

use sparkfox_core::{Error, Result};

use crate::provider::{LlmProvider, LlmStream};

/// Anthropic 默认 API base URL（不含 `/v1` 后缀，由 [`AnthropicProvider::messages_url`] 拼接）
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// Anthropic API 版本（固定为 `2023-06-01`，与官方 SDK 一致）
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// structured_complete 强制使用的 tool 名称（与 `tool_choice.name` 配对）
const STRUCTURED_TOOL_NAME: &str = "extract";

/// Anthropic 默认 max_tokens（Anthropic API 要求显式指定，与 OpenAI 不同）
const DEFAULT_MAX_TOKENS: u32 = 1024;

/// Anthropic Provider — 通过 HTTP 调用 Anthropic messages API
///
/// # 字段
/// - `api_key`: Anthropic API Key（以 `sk-ant-` 开头）
/// - `model`: 模型名（如 `claude-3-5-sonnet-20240620` / `claude-3-opus-20240229`）
/// - `base_url`: API 基础 URL（默认 `https://api.anthropic.com`，测试时可指向 mock 服务）
///
/// # 兼容性
/// - `new(api_key, model)` 保持与 v1.0.0 占位 API 兼容（使用默认 base_url）
/// - `with_base_url(api_key, model, base_url)` 用于测试或自定义部署
pub struct AnthropicProvider {
    /// API Key（从环境变量或配置文件加载）
    pub api_key: String,
    /// 模型名（如 "claude-3-5-sonnet-20240620" / "claude-3-opus-20240229"）
    pub model: String,
    /// API 基础 URL（不含 `/v1` 后缀）
    base_url: String,
}

impl AnthropicProvider {
    /// 创建 Anthropic Provider（使用默认 base_url = `https://api.anthropic.com`）
    ///
    /// 保持与 v1.0.0 占位 API 兼容。
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 创建 Anthropic Provider，指定 base_url（用于测试或自定义部署）
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

    /// 构造 `/v1/messages` 完整 URL
    fn messages_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{base}/v1/messages")
    }

    /// 构造带 `x-api-key` / `anthropic-version` headers 的 POST 请求 builder
    fn request_builder(
        &self,
        client: &reqwest::Client,
        body: serde_json::Value,
    ) -> reqwest::RequestBuilder {
        client
            .post(self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
    }
}

// ============================================================================
// 响应数据结构（仅解析必要字段，其余字段忽略）
// ============================================================================

/// Anthropic 非流式 messages 响应
///
/// 实际响应含 `id` / `type` / `role` / `model` / `stop_reason` / `usage` 等字段，
/// 此处仅解析 `content` 数组。
#[derive(Debug, Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

/// Anthropic content block — `text` 或 `tool_use` 两种类型
///
/// - `text` block: `{type: "text", text: "..."}`
/// - `tool_use` block: `{type: "tool_use", id: "...", name: "...", input: {...}}`
///
/// 用 `#[serde(tag = "type")]` 区分，但为简化解析逻辑，
/// 这里直接把所有可能字段都设为 `Option`，按 `type` 判定后取用相应字段。
#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(default)]
    r#type: String,
    /// text block 的文本内容
    #[serde(default)]
    text: Option<String>,
    /// tool_use block 的 input（符合 tool input_schema 的 JSON Value）
    #[serde(default)]
    input: Option<serde_json::Value>,
}

// ============================================================================
// LlmProvider 实现
// ============================================================================

#[async_trait]
impl LlmProvider for AnthropicProvider {
    /// 普通 completion — POST `/v1/messages`，解析 `content[0].text`
    async fn complete(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let body = message_builder(&self.model, prompt, false, None);

        let resp = self
            .request_builder(&client, body)
            .send()
            .await
            .map_err(|e| Error::llm(format!("Anthropic HTTP 请求失败: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_http_error(status).await);
        }

        let parsed: MessagesResponse = resp
            .json()
            .await
            .map_err(|e| Error::llm(format!("Anthropic 响应 JSON 解析失败: {e}")))?;

        // 取第一个 text 类型的 content block
        let text = parsed
            .content
            .into_iter()
            .find(|c| c.r#type == "text")
            .and_then(|c| c.text)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::llm("Anthropic 响应缺少 content[].text"))?;

        Ok(text)
    }

    /// 流式 completion — POST with `stream: true`，解析 SSE `content_block_delta` 事件
    ///
    /// Anthropic SSE 格式与 OpenAI 不同，含 `event:` 行：
    /// ```text
    /// event: content_block_delta
    /// data: {"delta":{"text":"world"}}
    ///
    /// ```
    async fn stream_complete(&self, prompt: &str) -> Result<LlmStream> {
        let client = reqwest::Client::new();
        let body = message_builder(&self.model, prompt, true, None);

        let resp = self
            .request_builder(&client, body)
            .send()
            .await
            .map_err(|e| Error::llm(format!("Anthropic HTTP 请求失败: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_http_error(status).await);
        }

        // 创建 mpsc channel，后台 task 消费 SSE 流逐 token 推送
        let (tx, stream) = LlmStream::channel(32);

        tokio::spawn(async move {
            let mut byte_stream = resp.bytes_stream();
            // SSE 事件缓冲：按 `\n\n` 分割事件，每个事件内含 `event:` 和 `data:` 行
            let mut buffer = String::new();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // 按空行（`\n\n`）分割 SSE 事件
                        while let Some(separator_pos) = buffer.find("\n\n") {
                            let event_block = buffer[..separator_pos].to_string();
                            buffer = buffer[separator_pos + 2..].to_string();

                            // 解析事件块：提取 event 类型和 data payload
                            let mut event_type: Option<String> = None;
                            let mut data_payload: Option<String> = None;

                            for line in event_block.lines() {
                                let line = line.trim();
                                if let Some(evt) = line.strip_prefix("event:") {
                                    event_type = Some(evt.trim().to_string());
                                } else if let Some(data) = line.strip_prefix("data:") {
                                    data_payload = Some(data.trim().to_string());
                                }
                            }

                            // 仅处理 content_block_delta 事件
                            if event_type.as_deref() != Some("content_block_delta") {
                                continue;
                            }

                            let Some(payload) = data_payload else {
                                continue;
                            };

                            // 解析 JSON，提取 delta.text
                            let Ok(value) =
                                serde_json::from_str::<serde_json::Value>(&payload)
                            else {
                                continue;
                            };

                            if let Some(text) = value
                                .get("delta")
                                .and_then(|d| d.get("text"))
                                .and_then(|t| t.as_str())
                            {
                                if !text.is_empty() {
                                    if tx.send(Ok(text.to_string())).await.is_err() {
                                        return; // 消费方已 drop
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(Error::llm(format!(
                                "Anthropic 流式读取失败: {e}"
                            ))))
                            .await;
                        return;
                    }
                }
            }
            // 流自然结束 — tx drop 后消费方 next() 返回 None
        });

        Ok(stream)
    }

    /// 结构化输出 — 通过 `tool_use` 强制 JSON schema（重写默认实现）
    ///
    /// Anthropic 不支持 OpenAI 风格的 `response_format: json_schema`，但可通过 tool_use
    /// 实现等价效果：
    /// 1. 在请求中添加 `tools: [{name: "extract", input_schema: schema}]`
    /// 2. 设置 `tool_choice: {type: "tool", name: "extract"}` 强制模型调用此 tool
    /// 3. 从响应 `content[*]` 中提取 `tool_use` block，返回其 `input` 作为 JSON Value
    async fn structured_complete(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let body = message_builder(&self.model, prompt, false, Some(schema));

        let resp = self
            .request_builder(&client, body)
            .send()
            .await
            .map_err(|e| Error::llm(format!("Anthropic HTTP 请求失败: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(map_http_error(status).await);
        }

        let parsed: MessagesResponse = resp
            .json()
            .await
            .map_err(|e| Error::llm(format!("Anthropic 响应 JSON 解析失败: {e}")))?;

        // 从 content[*] 中提取 tool_use block 的 input 字段
        for block in parsed.content {
            if block.r#type == "tool_use" {
                if let Some(input) = block.input {
                    return Ok(input);
                }
            }
        }

        Err(Error::llm(
            "Anthropic structured 响应缺少 tool_use content block",
        ))
    }
}

// ============================================================================
// Arc<AnthropicProvider> 委托实现 — 支持 Arc 包装后作 Box<dyn LlmProvider>
// ============================================================================

/// 为 `Arc<AnthropicProvider>` 委托实现 [`LlmProvider`]。
///
/// 业务层常需用 `Arc` 共享 provider 实例（如多个 Agent 共用一个 Claude 连接），
/// 此 impl 允许 `Arc<AnthropicProvider>` 直接作为 `Box<dyn LlmProvider>` / `&dyn LlmProvider`
/// 使用，无需手动解引用。
///
/// 所有方法委托给内部 `AnthropicProvider` 的对应实现。
#[async_trait]
impl LlmProvider for Arc<AnthropicProvider> {
    async fn complete(&self, prompt: &str) -> Result<String> {
        self.as_ref().complete(prompt).await
    }

    async fn stream_complete(&self, prompt: &str) -> Result<LlmStream> {
        self.as_ref().stream_complete(prompt).await
    }

    async fn structured_complete(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.as_ref().structured_complete(prompt, schema).await
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 构造 Anthropic `/v1/messages` 请求体
///
/// 抽取出公共请求体构造逻辑，供 `complete` / `stream_complete` / `structured_complete` 复用。
///
/// # 参数
/// - `model`: 模型名（如 `claude-3-5-sonnet-20240620`）
/// - `prompt`: 用户 prompt（作为单条 user message）
/// - `stream`: 是否启用流式（`true` 在请求中添加 `"stream": true`）
/// - `schema`: 结构化输出的 JSON Schema（`Some` 时添加 `tools` + `tool_choice`）
///
/// # Anthropic 请求体结构
/// ```json
/// {
///   "model": "claude-3-5-sonnet-20240620",
///   "max_tokens": 1024,
///   "messages": [{"role": "user", "content": "..."}]
/// }
/// ```
///
/// structured_complete 模式下额外添加：
/// ```json
/// {
///   "tools": [{"name": "extract", "input_schema": {...}}],
///   "tool_choice": {"type": "tool", "name": "extract"}
/// }
/// ```
fn message_builder(
    model: &str,
    prompt: &str,
    stream: bool,
    schema: Option<&serde_json::Value>,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "max_tokens": DEFAULT_MAX_TOKENS,
        "messages": [
            { "role": "user", "content": prompt }
        ]
    });

    if stream {
        body["stream"] = serde_json::json!(true);
    }

    if let Some(schema) = schema {
        // 通过 tool_use 强制 JSON：定义 extract tool，其 input_schema 即为目标 schema
        body["tools"] = serde_json::json!([
            {
                "name": STRUCTURED_TOOL_NAME,
                "description": "Extract structured data per the given JSON schema",
                "input_schema": schema
            }
        ]);
        // 强制模型必须调用此 tool（不能跳过 / 不能调其他 tool）
        body["tool_choice"] = serde_json::json!({
            "type": "tool",
            "name": STRUCTURED_TOOL_NAME
        });
    }

    body
}

/// 将 HTTP 错误状态码映射为 `Error::Llm`，覆盖 401 / 429 / 500 等常见错误
async fn map_http_error(status: reqwest::StatusCode) -> Error {
    let code = status.as_u16();
    match code {
        401 => Error::llm(format!(
            "Anthropic API key 无效 (HTTP {code}) — 请检查 API Key 配置"
        )),
        429 => Error::llm(format!(
            "Anthropic rate limit / 限流 (HTTP {code}) — 请稍后重试"
        )),
        500..=599 => Error::llm(format!(
            "Anthropic server error / 服务器错误 (HTTP {code})"
        )),
        _ => Error::llm(format!("Anthropic API 错误: HTTP {code}")),
    }
}

// ============================================================================
// 单元测试 — 与集成测试互补，覆盖纯逻辑分支
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 `new()` 构造器使用默认 base_url
    #[test]
    fn test_new_uses_default_base_url() {
        let p = AnthropicProvider::new("sk-ant-test", "claude-3-5-sonnet-20240620");
        assert_eq!(p.api_key, "sk-ant-test");
        assert_eq!(p.model, "claude-3-5-sonnet-20240620");
        assert_eq!(p.base_url, DEFAULT_BASE_URL);
    }

    /// 验证 `with_base_url()` 构造器
    #[test]
    fn test_with_base_url() {
        let p = AnthropicProvider::with_base_url(
            "sk-ant-test",
            "claude-3-5-sonnet-20240620",
            "http://localhost:8080",
        );
        assert_eq!(p.base_url, "http://localhost:8080");
    }

    /// 验证 URL 拼接逻辑（含 trailing slash 处理）
    #[test]
    fn test_messages_url() {
        let p1 = AnthropicProvider::new("sk-ant", "claude-3-5-sonnet-20240620");
        assert_eq!(
            p1.messages_url(),
            "https://api.anthropic.com/v1/messages"
        );

        let p2 = AnthropicProvider::with_base_url(
            "sk-ant",
            "claude-3-5-sonnet-20240620",
            "http://localhost:8080/",
        );
        assert_eq!(p2.messages_url(), "http://localhost:8080/v1/messages");
    }

    /// 验证 message_builder 基础（非流式 / 非结构化）
    #[test]
    fn test_message_builder_basic() {
        let body = message_builder("claude-3-5-sonnet-20240620", "hello", false, None);
        assert_eq!(body["model"], "claude-3-5-sonnet-20240620");
        assert_eq!(body["max_tokens"], DEFAULT_MAX_TOKENS);
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "hello");
        assert!(body.get("stream").is_none());
        assert!(body.get("tools").is_none());
        assert!(body.get("tool_choice").is_none());
    }

    /// 验证 message_builder 流式标记
    #[test]
    fn test_message_builder_stream() {
        let body = message_builder("claude-3-5-sonnet-20240620", "hello", true, None);
        assert_eq!(body["stream"], true);
    }

    /// 验证 message_builder structured 模式（含 tools + tool_choice）
    #[test]
    fn test_message_builder_structured() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": { "name": { "type": "string" } }
        });
        let body = message_builder(
            "claude-3-5-sonnet-20240620",
            "extract",
            false,
            Some(&schema),
        );
        assert_eq!(body["tools"][0]["name"], STRUCTURED_TOOL_NAME);
        assert_eq!(body["tools"][0]["input_schema"]["required"][0], "name");
        assert_eq!(body["tool_choice"]["type"], "tool");
        assert_eq!(body["tool_choice"]["name"], STRUCTURED_TOOL_NAME);
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
        assert!(
            msg.contains("server error") || msg.contains("500") || msg.contains("服务器")
        );
    }
}
