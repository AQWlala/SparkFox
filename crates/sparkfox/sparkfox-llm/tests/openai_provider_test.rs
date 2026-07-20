//! Sub-Step 10.1.2 — OpenAIProvider 真实实现集成测试
//!
//! 使用 wiremock 起本地 mock 服务，模拟 OpenAI `/v1/chat/completions` 接口，
//! 验证 [`sparkfox_llm::OpenAIProvider`] 的 `complete` / `stream_complete` /
//! `structured_complete` 三个方法以及错误处理。
//!
//! # 测试用例
//! 1. `test_openai_provider_implements_trait` — 静态断言 OpenAIProvider 实现 LlmProvider
//! 2. `test_openai_complete_mocks_api_response` — 验证 complete 解析 choices[0].message.content
//! 3. `test_openai_stream_handles_sse_chunks` — 验证 stream_complete 逐 token 产出
//! 4. `test_openai_structured_complete_sends_json_schema` — 验证请求体含 response_format: json_schema
//! 5. `test_openai_handles_api_error` — 验证 401/429/500 三种错误码映射

#![forbid(unsafe_code)]

use sparkfox_llm::{LlmProvider, OpenAIProvider};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 构造一段标准 OpenAI 非流式 chat completion 响应 JSON
fn make_chat_completion_response(content: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test-001",
        "object": "chat.completion",
        "created": 1718000000_u64,
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": content },
            "finish_reason": "stop"
        }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 }
    })
}

/// 构造一段 SSE 流式响应 body（多个 delta chunk + 末尾 [DONE]）
fn make_sse_stream_body(chunks: &[serde_json::Value]) -> String {
    let mut body = String::new();
    for chunk in chunks {
        body.push_str("data: ");
        body.push_str(&chunk.to_string());
        body.push_str("\n\n");
    }
    body.push_str("data: [DONE]\n\n");
    body
}

// ---------------------------------------------------------------------------
// 测试 1: OpenAIProvider 实现 LlmProvider trait（编译期静态断言）
// ---------------------------------------------------------------------------

/// 静态断言 `OpenAIProvider` 实现了 `LlmProvider` trait。
///
/// 该测试不需要运行时网络 — 仅在编译期通过 trait bound 验证 trait 实现。
/// 若 OpenAIProvider 没有实现 LlmProvider，本测试将无法编译。
#[test]
fn test_openai_provider_implements_trait() {
    let provider = OpenAIProvider::new("sk-test-key", "gpt-4");
    // 借用 provider 以 `&dyn LlmProvider` 形式确认 trait 实现
    let _trait_obj: &dyn LlmProvider = &provider;
    // 同时验证基础字段保持向后兼容（占位 API：api_key + model）
    assert_eq!(provider.api_key, "sk-test-key");
    assert_eq!(provider.model, "gpt-4");
}

// ---------------------------------------------------------------------------
// 测试 2: complete 解析 choices[0].message.content
// ---------------------------------------------------------------------------

/// 使用 wiremock 模拟 OpenAI `/v1/chat/completions` 返回，
/// 验证 [`OpenAIProvider::complete`] 正确解析 `choices[0].message.content`。
#[tokio::test]
async fn test_openai_complete_mocks_api_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer sk-test-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(make_chat_completion_response("Hello from mock GPT-4")),
        )
        .mount(&server)
        .await;

    let provider = OpenAIProvider::with_base_url(
        "sk-test-key",
        "gpt-4",
        server.uri(),
    );

    let text = provider
        .complete("ping")
        .await
        .expect("complete 应成功解析 mock 响应");
    assert_eq!(text, "Hello from mock GPT-4");
}

// ---------------------------------------------------------------------------
// 测试 3: stream_complete 逐 token 产出 SSE chunks
// ---------------------------------------------------------------------------

/// 模拟 OpenAI SSE 流，验证 [`OpenAIProvider::stream_complete`] 按 token 增量产出。
#[tokio::test]
async fn test_openai_stream_handles_sse_chunks() {
    let server = MockServer::start().await;

    let chunk1 = serde_json::json!({
        "id": "chatcmpl-stream-001",
        "object": "chat.completion.chunk",
        "choices": [{
            "index": 0,
            "delta": { "role": "assistant", "content": "Hello" },
            "finish_reason": null
        }]
    });
    let chunk2 = serde_json::json!({
        "id": "chatcmpl-stream-001",
        "object": "chat.completion.chunk",
        "choices": [{
            "index": 0,
            "delta": { "content": ", " },
            "finish_reason": null
        }]
    });
    let chunk3 = serde_json::json!({
        "id": "chatcmpl-stream-001",
        "object": "chat.completion.chunk",
        "choices": [{
            "index": 0,
            "delta": { "content": "world!" },
            "finish_reason": null
        }]
    });
    let chunk4 = serde_json::json!({
        "id": "chatcmpl-stream-001",
        "object": "chat.completion.chunk",
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": "stop"
        }]
    });

    let sse_body = make_sse_stream_body(&[chunk1, chunk2, chunk3, chunk4]);

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer sk-stream-key"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAIProvider::with_base_url(
        "sk-stream-key",
        "gpt-4",
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
        vec!["Hello", ", ", "world!"],
        "应按 SSE 顺序产出 3 段非空 token，实际: {:?}",
        tokens
    );
}

// ---------------------------------------------------------------------------
// 测试 4: structured_complete 请求体含 response_format: json_schema
// ---------------------------------------------------------------------------

/// 自定义 Respond — 捕获请求体，验证 `response_format: { type: "json_schema", json_schema: {...} }`，
/// 然后返回符合 schema 的 JSON 响应。
#[derive(Clone)]
struct JsonSchemaCaptureResponder;

impl wiremock::Respond for JsonSchemaCaptureResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: serde_json::Value = serde_json::from_slice(&request.body)
            .expect("请求体应为合法 JSON");
        // 验证 response_format 字段
        let response_format = body
            .get("response_format")
            .expect("请求体应含 response_format 字段");
        assert_eq!(
            response_format["type"],
            "json_schema",
            "response_format.type 应为 'json_schema'，实际: {response_format}"
        );
        assert!(
            response_format.get("json_schema").is_some(),
            "response_format 应含 json_schema 子对象"
        );
        // 返回符合 schema 的 JSON 内容
        let content = r#"{"name":"SparkFox","version":"1.1.0"}"#;
        ResponseTemplate::new(200).set_body_json(make_chat_completion_response(content))
    }
}

/// 验证 [`OpenAIProvider::structured_complete`] 在请求体中带 `response_format: json_schema`，
/// 并能正确解析返回的 JSON。
#[tokio::test]
async fn test_openai_structured_complete_sends_json_schema() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer sk-schema-key"))
        .respond_with(JsonSchemaCaptureResponder)
        .mount(&server)
        .await;

    let provider = OpenAIProvider::with_base_url(
        "sk-schema-key",
        "gpt-4",
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

// ---------------------------------------------------------------------------
// 测试 5: 错误处理 — 401 / 429 / 500 三种错误码
// ---------------------------------------------------------------------------

/// 验证 401 错误映射到 `Error::Llm("...API key 无效...")`。
#[tokio::test]
async fn test_openai_handles_401_invalid_api_key() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(401).set_body_string(
                r#"{"error":{"message":"Invalid API key","type":"invalid_request_error"}}"#,
            ),
        )
        .mount(&server)
        .await;

    let provider = OpenAIProvider::with_base_url("sk-bad", "gpt-4", server.uri());
    let err = provider
        .complete("any")
        .await
        .expect_err("401 应返回 Err");
    let msg = format!("{err}");
    assert!(
        msg.contains("API key") || msg.contains("401"),
        "错误信息应提及 API key 无效或 401，实际: {msg}"
    );
}

/// 验证 429 错误映射到 rate limit 相关错误。
#[tokio::test]
async fn test_openai_handles_429_rate_limit() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429).set_body_string(
                r#"{"error":{"message":"Rate limit exceeded","type":"rate_limit_exceeded"}}"#,
            ),
        )
        .mount(&server)
        .await;

    let provider = OpenAIProvider::with_base_url("sk-rate", "gpt-4", server.uri());
    let err = provider
        .complete("any")
        .await
        .expect_err("429 应返回 Err");
    let msg = format!("{err}");
    assert!(
        msg.contains("rate limit") || msg.contains("429") || msg.contains("限流"),
        "错误信息应提及 rate limit / 限流 / 429，实际: {msg}"
    );
}

/// 验证 500 错误映射到 server error 相关错误。
#[tokio::test]
async fn test_openai_handles_500_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_string(r#"{"error":{"message":"Internal server error"}}"#),
        )
        .mount(&server)
        .await;

    let provider = OpenAIProvider::with_base_url("sk-500", "gpt-4", server.uri());
    let err = provider
        .complete("any")
        .await
        .expect_err("500 应返回 Err");
    let msg = format!("{err}");
    assert!(
        msg.contains("server error") || msg.contains("500") || msg.contains("服务器"),
        "错误信息应提及 server error / 500 / 服务器，实际: {msg}"
    );
}
