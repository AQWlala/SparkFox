//! Sub-Step 10.1.4 — OllamaProvider 集成测试（RED → GREEN → REFACTOR）
//!
//! 使用 wiremock 起本地 mock 服务，模拟 Ollama `/api/chat` 接口，
//! 验证 [`sparkfox_llm::OllamaProvider`] 的 `complete` / `stream_complete` /
//! `structured_complete` 三个方法。
//!
//! # 测试用例
//! 1. `test_ollama_provider_implements_trait` — 静态断言 `Arc<OllamaProvider>` 可作 `Box<dyn LlmProvider>`
//! 2. `test_ollama_complete_mocks_local_api` — 验证 complete 解析 `message.content`
//! 3. `test_ollama_stream_handles_ndjson` — 验证 stream_complete 消费 NDJSON 流
//! 4. `test_ollama_structured_complete_uses_format_field` — 验证 structured_complete 通过 `format: <schema>` 约束 JSON
//!
//! # Ollama API 形态
//! - 非流式：POST /api/chat，body `{model, messages, stream:false}`，
//!   响应 `{"message":{"role":"assistant","content":"..."},"done":true}`
//! - 流式：POST /api/chat，body `{...,stream:true}`，
//!   响应为 NDJSON（每行一个 JSON 对象），最后一行 `{"done":true}`
//! - 结构化：在 body 中追加 `format: <json_schema>` 字段（Ollama 原生 JSON schema 约束）

#![forbid(unsafe_code)]

use std::sync::Arc;

use sparkfox_llm::{LlmProvider, OllamaProvider};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 构造 Ollama 非流式 `/api/chat` 响应 JSON
fn make_ollama_chat_response(content: &str) -> serde_json::Value {
    serde_json::json!({
        "model": "llama3",
        "created_at": "2026-07-20T00:00:00Z",
        "message": { "role": "assistant", "content": content },
        "done": true,
        "total_duration": 1_000_000_000_u64,
        "load_duration": 500_000_000_u64,
        "prompt_eval_count": 10,
        "prompt_eval_duration": 200_000_000_u64,
        "eval_count": 5,
        "eval_duration": 300_000_000_u64
    })
}

/// 构造 Ollama 流式 NDJSON body（每行一个 JSON 对象，末尾 `{"done":true}`）
fn make_ndjson_stream_body(chunks: &[serde_json::Value]) -> String {
    let mut body = String::new();
    for chunk in chunks {
        body.push_str(&chunk.to_string());
        body.push('\n');
    }
    body
}

// ---------------------------------------------------------------------------
// 测试 1: OllamaProvider 实现 LlmProvider trait（编译期静态断言）
// ---------------------------------------------------------------------------

/// 验证 `Arc<OllamaProvider>` 可作 `Box<dyn LlmProvider>` — 即：
/// - OllamaProvider 实现了 LlmProvider trait
/// - OllamaProvider 满足 Send + Sync（Arc 共享前置条件）
///
/// 该测试不需要运行时网络 — 仅在编译期通过 trait bound 验证 trait 实现。
#[test]
fn test_ollama_provider_implements_trait() {
    let provider = Arc::new(OllamaProvider::new("llama3"));
    // 验证基础字段
    assert_eq!(provider.model, "llama3");
    // 验证默认 base_url 指向本地 Ollama 服务
    assert_eq!(provider.base_url(), "http://localhost:11434");

    // 静态断言：Arc<OllamaProvider> 可作为 Box<dyn LlmProvider>
    // （Arc 用于跨任务共享；Box 用于 trait object 持有 — 两者都需要 Send + Sync）
    let boxed: Box<dyn LlmProvider> = Box::new(OllamaProvider::new("llama3"));
    let _arc: Arc<dyn LlmProvider> = Arc::new(OllamaProvider::new("llama3"));
    // 引用 boxed 避免未使用变量 warning
    let _ = &boxed;
}

// ---------------------------------------------------------------------------
// 测试 2: complete 解析 message.content
// ---------------------------------------------------------------------------

/// 使用 wiremock 模拟 Ollama `/api/chat` 返回，
/// 验证 [`OllamaProvider::complete`] 正确解析 `message.content`。
#[tokio::test]
async fn test_ollama_complete_mocks_local_api() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(make_ollama_chat_response("hello")),
        )
        .mount(&server)
        .await;

    let provider = OllamaProvider::with_base_url("llama3", server.uri());

    let text = provider
        .complete("ping")
        .await
        .expect("complete 应成功解析 mock 响应");
    assert_eq!(text, "hello");
}

// ---------------------------------------------------------------------------
// 测试 3: stream_complete 消费 NDJSON 流
// ---------------------------------------------------------------------------

/// 模拟 Ollama NDJSON 流（每行一个 JSON 对象），验证
/// [`OllamaProvider::stream_complete`] 按 `message.content` 增量推送。
#[tokio::test]
async fn test_ollama_stream_handles_ndjson() {
    let server = MockServer::start().await;

    let chunk1 = serde_json::json!({
        "model": "llama3",
        "created_at": "2026-07-20T00:00:00Z",
        "message": { "role": "assistant", "content": "world" },
        "done": false
    });
    let chunk2 = serde_json::json!({
        "model": "llama3",
        "created_at": "2026-07-20T00:00:01Z",
        "message": { "role": "assistant", "content": "world" },
        "done": false
    });
    let chunk_end = serde_json::json!({
        "model": "llama3",
        "created_at": "2026-07-20T00:00:02Z",
        "message": { "role": "assistant", "content": "" },
        "done": true
    });

    let ndjson_body = make_ndjson_stream_body(&[chunk1, chunk2, chunk_end]);

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(ndjson_body, "application/x-ndjson"),
        )
        .mount(&server)
        .await;

    let provider = OllamaProvider::with_base_url("llama3", server.uri());

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
        vec!["world", "world"],
        "应按 NDJSON 顺序产出 2 段非空 content，实际: {:?}",
        tokens
    );
}

// ---------------------------------------------------------------------------
// 测试 4: structured_complete 通过 format 字段约束 JSON
// ---------------------------------------------------------------------------

/// 自定义 Respond — 捕获请求体，验证含 `format` 字段（Ollama 原生 JSON schema 约束），
/// 然后返回符合 schema 的 JSON 内容（嵌在 `message.content` 字符串中）。
#[derive(Clone)]
struct FormatFieldCaptureResponder;

impl wiremock::Respond for FormatFieldCaptureResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: serde_json::Value = serde_json::from_slice(&request.body)
            .expect("请求体应为合法 JSON");
        // 验证 format 字段存在（Ollama 用 format 字段约束输出 JSON schema）
        let format = body
            .get("format")
            .expect("请求体应含 format 字段（Ollama 原生 JSON schema 约束）");
        // format 应为 object（JSON schema 本身）或 string（部分 Ollama 版本支持 "json"）
        assert!(
            format.is_object() || format.is_string(),
            "format 应为 JSON schema object 或字符串 'json'，实际: {format}"
        );
        // 返回符合 schema 的 JSON 内容（Ollama 将 JSON 作为字符串放进 message.content）
        let content = r#"{"key":"value"}"#;
        ResponseTemplate::new(200).set_body_json(make_ollama_chat_response(content))
    }
}

/// 验证 [`OllamaProvider::structured_complete`] 在请求体中带 `format: <schema>` 字段，
/// 并能正确解析返回的 JSON。
#[tokio::test]
async fn test_ollama_structured_complete_uses_format_field() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(FormatFieldCaptureResponder)
        .mount(&server)
        .await;

    let provider = OllamaProvider::with_base_url("llama3", server.uri());

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "key": { "type": "string" }
        },
        "required": ["key"]
    });

    let value = provider
        .structured_complete("输出 JSON", &schema)
        .await
        .expect("structured_complete 应成功返回 JSON Value");

    assert_eq!(value["key"], "value");

    // 验证请求体确实被服务端收到并校验
    let received = server
        .received_requests()
        .await
        .expect("应能获取收到请求列表");
    assert_eq!(received.len(), 1, "应仅收到 1 次请求");
}
