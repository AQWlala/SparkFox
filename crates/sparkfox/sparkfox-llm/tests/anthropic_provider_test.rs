//! Sub-Step 10.1.3 — AnthropicProvider 真实实现集成测试
//!
//! 使用 wiremock 起本地 mock 服务，模拟 Anthropic `/v1/messages` 接口，
//! 验证 [`sparkfox_llm::AnthropicProvider`] 的 `complete` / `stream_complete` /
//! `structured_complete` 三个方法。
//!
//! # 测试用例
//! 1. `test_anthropic_provider_implements_trait` — 静态断言 `Arc<AnthropicProvider>` 可作 `Box<dyn LlmProvider>`
//! 2. `test_anthropic_complete_mocks_messages_api` — 验证 complete 解析 content[0].text
//! 3. `test_anthropic_stream_handles_event_stream` — 验证 stream_complete 解析 SSE content_block_delta
//! 4. `test_anthropic_structured_complete_uses_tool_use` — 验证 structured_complete 通过 tool_use 返回合法 JSON
//!
//! # Anthropic Messages API 与 OpenAI 的差异
//! - 鉴权：`x-api-key` header（非 Bearer）+ `anthropic-version: 2023-06-01`
//! - 端点：`POST /v1/messages`（非 `/v1/chat/completions`）
//! - 响应结构：`{content: [{type: "text", text: "..."}]}`（非 `choices[0].message.content`）
//! - 流式：SSE `event: <type>\ndata: <json>\n\n` 格式（含 event 行，OpenAI 仅有 data 行）
//! - 结构化输出：通过 tool_use 强制 JSON（OpenAI 用 response_format: json_schema）

#![forbid(unsafe_code)]

use std::sync::Arc;

use sparkfox_llm::{AnthropicProvider, LlmProvider};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 构造一段标准 Anthropic 非流式 messages 响应 JSON
///
/// Anthropic 响应结构：
/// ```json
/// {
///   "id": "msg_xxx",
///   "type": "message",
///   "role": "assistant",
///   "model": "claude-3-5-sonnet-20240620",
///   "content": [{"type": "text", "text": "..."}],
///   "stop_reason": "end_turn",
///   "usage": {"input_tokens": 10, "output_tokens": 5}
/// }
/// ```
fn make_messages_response(text: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "msg_test-001",
        "type": "message",
        "role": "assistant",
        "model": "claude-3-5-sonnet-20240620",
        "content": [
            { "type": "text", "text": text }
        ],
        "stop_reason": "end_turn",
        "usage": { "input_tokens": 10, "output_tokens": 5 }
    })
}

// ---------------------------------------------------------------------------
// 测试 1: AnthropicProvider 实现 LlmProvider trait（编译期静态断言）
// ---------------------------------------------------------------------------

/// 静态断言 `Arc<AnthropicProvider>` 可作为 `Box<dyn LlmProvider>` 使用。
///
/// 该测试不需要运行时网络 — 仅在编译期通过 trait bound 验证 trait 实现，
/// 同时验证 Anthropic 实例可以包在 `Arc` 中并以 trait object 形式持有
/// （这是 AuditedProvider 装饰器模式的典型用法）。
#[test]
fn test_anthropic_provider_implements_trait() {
    let provider = AnthropicProvider::new("sk-ant-test-key", "claude-3-5-sonnet-20240620");
    // Arc<AnthropicProvider> → Box<dyn LlmProvider> 转换必须可行
    let _trait_obj: Box<dyn LlmProvider> = Box::new(Arc::new(provider));
    // 同时验证基础字段保持向后兼容（占位 API：api_key + model）
    let provider2 = AnthropicProvider::new("sk-ant-test-key", "claude-3-5-sonnet-20240620");
    assert_eq!(provider2.api_key, "sk-ant-test-key");
    assert_eq!(provider2.model, "claude-3-5-sonnet-20240620");
}

// ---------------------------------------------------------------------------
// 测试 2: complete 解析 content[0].text
// ---------------------------------------------------------------------------

/// 使用 wiremock 模拟 Anthropic `/v1/messages` 返回，
/// 验证 [`AnthropicProvider::complete`] 正确解析 `content[0].text`。
///
/// 同时验证请求 header 含 `x-api-key` 和 `anthropic-version: 2023-06-01`。
#[tokio::test]
async fn test_anthropic_complete_mocks_messages_api() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "sk-ant-test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(make_messages_response("Hello from mock Claude")),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::with_base_url(
        "sk-ant-test-key",
        "claude-3-5-sonnet-20240620",
        server.uri(),
    );

    let text = provider
        .complete("ping")
        .await
        .expect("complete 应成功解析 mock 响应");
    assert_eq!(text, "Hello from mock Claude");
}

// ---------------------------------------------------------------------------
// 测试 3: stream_complete 处理 SSE event-stream
// ---------------------------------------------------------------------------

/// 模拟 Anthropic SSE event-stream（含 `event:` 行），验证
/// [`AnthropicProvider::stream_complete`] 解析 `content_block_delta` 事件并推送 `delta.text`。
///
/// Anthropic SSE 格式与 OpenAI 不同：
/// ```text
/// event: content_block_delta
/// data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"world"}}
///
/// ```
#[tokio::test]
async fn test_anthropic_stream_handles_event_stream() {
    let server = MockServer::start().await;

    // 构造 Anthropic SSE event-stream body
    // 任务 spec 要求事件格式：`event: content_block_delta\ndata: {"delta":{"text":"world"}}\n\n`
    let sse_body = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_stream_001\"}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0}\n\n",
        "event: content_block_delta\n",
        "data: {\"delta\":{\"text\":\"world\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "sk-ant-stream-key"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(sse_body.to_string(), "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::with_base_url(
        "sk-ant-stream-key",
        "claude-3-5-sonnet-20240620",
        server.uri(),
    );

    let mut stream = provider
        .stream_complete("ping")
        .await
        .expect("stream_complete 应成功启动");

    let mut tokens = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("流式 chunk 不应为 Err");
        tokens.push(chunk);
    }

    assert_eq!(
        tokens,
        vec!["world"],
        "应仅从 content_block_delta 推送 'world'，实际: {:?}",
        tokens
    );
}

// ---------------------------------------------------------------------------
// 测试 4: structured_complete 通过 tool_use 强制 JSON schema
// ---------------------------------------------------------------------------

/// 自定义 Respond — 捕获请求体，验证 `tools` + `tool_choice` 字段，
/// 然后返回含 `tool_use` content block 的响应（input 为符合 schema 的 JSON）。
#[derive(Clone)]
struct ToolUseCaptureResponder;

impl wiremock::Respond for ToolUseCaptureResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: serde_json::Value = serde_json::from_slice(&request.body)
            .expect("请求体应为合法 JSON");
        // 验证 tools 字段
        let tools = body
            .get("tools")
            .expect("请求体应含 tools 字段（Anthropic tool_use 强制 JSON）");
        assert!(
            tools.is_array() && !tools.as_array().unwrap().is_empty(),
            "tools 应为非空数组"
        );
        // 验证 tool_choice 字段
        let tool_choice = body
            .get("tool_choice")
            .expect("请求体应含 tool_choice 字段");
        assert_eq!(
            tool_choice["type"], "tool",
            "tool_choice.type 应为 'tool'，实际: {tool_choice}"
        );
        assert_eq!(
            tool_choice["name"], "extract",
            "tool_choice.name 应为 'extract'，实际: {tool_choice}"
        );
        // 验证 tools[0].input_schema 即为传入 schema
        let input_schema = &tools[0]["input_schema"];
        assert!(
            input_schema.get("required").is_some(),
            "tools[0].input_schema 应含 required 字段"
        );

        // 返回含 tool_use content block 的响应
        let response = serde_json::json!({
            "id": "msg_structured_001",
            "type": "message",
            "role": "assistant",
            "model": "claude-3-5-sonnet-20240620",
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_test_001",
                    "name": "extract",
                    "input": { "name": "SparkFox", "version": "1.1.0" }
                }
            ],
            "stop_reason": "tool_use",
            "usage": { "input_tokens": 12, "output_tokens": 8 }
        });
        ResponseTemplate::new(200).set_body_json(response)
    }
}

/// 验证 [`AnthropicProvider::structured_complete`] 在请求体中带 `tools` + `tool_choice`，
/// 并能正确解析返回的 `tool_use` content block 中的 `input` JSON。
#[tokio::test]
async fn test_anthropic_structured_complete_uses_tool_use() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "sk-ant-schema-key"))
        .respond_with(ToolUseCaptureResponder)
        .mount(&server)
        .await;

    let provider = AnthropicProvider::with_base_url(
        "sk-ant-schema-key",
        "claude-3-5-sonnet-20240620",
        server.uri(),
    );

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "version": { "type": "string" }
        },
        "required": ["name", "version"]
    });

    let value = provider
        .structured_complete("提取实体", &schema)
        .await
        .expect("structured_complete 应成功返回 JSON Value");

    assert_eq!(value["name"], "SparkFox");
    assert_eq!(value["version"], "1.1.0");

    // 验证请求体确实被服务端收到并校验（MockServer 默认会记录所有请求）
    let received = server
        .received_requests()
        .await
        .expect("应能获取收到请求列表");
    assert_eq!(received.len(), 1, "应仅收到 1 次请求");
}
