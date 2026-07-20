//! Sub-Step 10.1.5 — JSON repair 重试集成测试（RED → GREEN → REFACTOR）
//!
//! 验证 [`sparkfox_llm`] 的 JSON repair 能力与 `structured_complete` 重试逻辑，
//! 缓解 RISK-SAG-04（国产模型 structured output 不稳定）。
//!
//! # 测试用例
//! 1. `test_repair_trailing_comma` — `repair_json` 修复尾随逗号
//! 2. `test_repair_unquoted_key` — `repair_json` 修复未引号键
//! 3. `test_repair_markdown_code_fence` — `repair_json` 从 ```json ... ``` 中提取 JSON
//! 4. `test_structured_complete_retries_3_times_on_invalid_json` — 3 次无效 JSON 后返回 Err（重试上限 ≤ 3）
//! 5. `test_structured_complete_returns_repaired_json` — 首次返回带 trailing comma 的 JSON，
//!    验证自动修复后返回合法 Value
//!
//! # 重试策略（GREEN 阶段实现）
//! - 第 1 次：调用 `complete` → 尝试直接 `serde_json::from_str`
//! - 第 2 次：`repair_json(原始文本)` → 解析（不再调用 LLM）
//! - 第 3 次：重新调用 `complete`（prompt 追加 "请返回合法 JSON" 提示）→ 解析
//! - 3 次都失败 → 返回 Err

#![forbid(unsafe_code)]

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

use sparkfox_core::{Error, Result};
use sparkfox_llm::{LlmProvider, LlmStream};

// ---------------------------------------------------------------------------
// 测试用 MockProvider — 按预设队列返回 complete 结果，并记录调用次数
// ---------------------------------------------------------------------------

/// 测试用 Mock Provider — 按预设队列返回 `complete` 结果
///
/// 用于验证 `structured_complete` 默认实现的 JSON 解析 / repair / 重试逻辑，
/// 不实际调用 LLM。`call_count()` 暴露 `complete` 被调用的次数，用于断言重试上限。
struct MockProvider {
    responses: Vec<String>,
    call_count: AtomicUsize,
}

impl MockProvider {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            call_count: AtomicUsize::new(0),
        }
    }

    /// 返回 `complete` 被调用的总次数
    fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
        self.responses
            .get(idx)
            .cloned()
            .ok_or_else(|| Error::llm(format!("MockProvider 队列耗尽（call #{idx})")))
    }

    async fn stream_complete(&self, _prompt: &str) -> Result<LlmStream> {
        Err(Error::llm("MockProvider 不支持 stream_complete"))
    }
}

// ---------------------------------------------------------------------------
// 测试 1: repair_json 修复尾随逗号
// ---------------------------------------------------------------------------

/// 验证 [`sparkfox_llm::repair_json`] 能修复 `{"a":1,}` → `{"a":1}`。
///
/// 尾随逗号是 LLM 输出 JSON 时最常见的格式错误之一（RISK-SAG-04）。
#[test]
fn test_repair_trailing_comma() {
    let value = sparkfox_llm::repair_json("{\"a\":1,}")
        .expect("带尾随逗号的 JSON 应被修复");
    assert_eq!(value["a"], 1, "修复后应为 {{\"a\":1}}");
    // 修复后应为 object，且仅有 1 个字段
    assert!(value.is_object(), "修复后应为 object");
    assert_eq!(value.as_object().unwrap().len(), 1, "应仅 1 个字段");
}

// ---------------------------------------------------------------------------
// 测试 2: repair_json 修复未引号键
// ---------------------------------------------------------------------------

/// 验证 [`sparkfox_llm::repair_json`] 能修复 `{a:1}` → `{"a":1}`。
///
/// 未引号键在 JavaScript 中合法但 JSON 标准不允许，国产模型常犯此错误。
#[test]
fn test_repair_unquoted_key() {
    let value = sparkfox_llm::repair_json("{a:1}")
        .expect("未引号键的 JSON 应被修复");
    assert_eq!(value["a"], 1, "修复后应为 {{\"a\":1}}");
    assert!(value.is_object(), "修复后应为 object");
}

// ---------------------------------------------------------------------------
// 测试 3: repair_json 从 markdown code fence 中提取 JSON
// ---------------------------------------------------------------------------

/// 验证 [`sparkfox_llm::repair_json`] 能从 ` ```json\n{...}\n``` ` 包裹的文本中提取 JSON。
///
/// LLM（特别是聊天模型）常将 JSON 输出包裹在 markdown 代码块中，需先剥离 fence 再解析。
#[test]
fn test_repair_markdown_code_fence() {
    let input = "```json\n{\"a\":1}\n```";
    let value = sparkfox_llm::repair_json(input)
        .expect("markdown fence 包裹的 JSON 应被提取并修复");
    assert_eq!(value["a"], 1, "提取后应为 {{\"a\":1}}");
    assert!(value.is_object(), "提取后应为 object");
}

// ---------------------------------------------------------------------------
// 测试 4: structured_complete 重试上限 ≤ 3 次
// ---------------------------------------------------------------------------

/// 验证 [`LlmProvider::structured_complete`] 默认实现在连续无效 JSON 时重试上限 ≤ 3 次。
///
/// 策略：MockProvider 队列返回 3 次无效 JSON（彻底无法 repair 的乱码文本），
/// 期望：
/// - 最终返回 `Err`（3 次尝试全部失败）
/// - `complete` 调用次数 ≤ 2（第 2 次重试为本地 repair，不调 LLM；第 3 次重试才再调 LLM）
///
/// 该测试确保 RISK-SAG-04 缓解机制有重试上限，不会无限重试消耗 token。
#[tokio::test]
async fn test_structured_complete_retries_3_times_on_invalid_json() {
    // 队列：3 次都返回彻底无法 repair 的乱码（repair 后仍非 object，schema 校验失败）
    let provider = MockProvider::new(vec![
        "!!!这不是 JSON 也不是 JSON 的修复候选!!!".to_string(),
        "@@@依然不是 JSON@@@".to_string(),
        "###第三次仍然不是 JSON###".to_string(),
    ]);
    let schema = serde_json::json!({
        "type": "object",
        "required": ["a"],
    });

    let result = provider.structured_complete("test", &schema).await;
    assert!(
        result.is_err(),
        "3 次重试都失败时应返回 Err，实际: {:?}",
        result
    );

    let calls = provider.call_count();
    assert!(
        calls <= 2,
        "complete 调用次数应 ≤ 2（第 2 次重试为本地 repair 不调 LLM，第 3 次重试才再调），实际: {calls}"
    );
    assert!(
        calls >= 1,
        "至少应调用 1 次 complete，实际: {calls}"
    );

    let err_msg = format!("{}", result.unwrap_err());
    // 错误信息应包含上下文，便于排查
    assert!(
        !err_msg.is_empty(),
        "错误信息不应为空"
    );
}

// ---------------------------------------------------------------------------
// 测试 5: structured_complete 首次返回带 trailing comma 的 JSON，自动修复
// ---------------------------------------------------------------------------

/// 验证 [`LlmProvider::structured_complete`] 默认实现能自动修复首次返回的带 trailing comma 的 JSON。
///
/// 模拟场景：LLM 首次返回 `{"a":1,}`（尾随逗号，常见国产模型错误），
/// 期望 `structured_complete` 自动 repair 后返回合法 `Value`，无需用户介入。
#[tokio::test]
async fn test_structured_complete_returns_repaired_json() {
    let provider = MockProvider::new(vec![r#"{"a":1,}"#.to_string()]);
    let schema = serde_json::json!({
        "type": "object",
        "required": ["a"],
    });

    let value = provider
        .structured_complete("输出 JSON", &schema)
        .await
        .expect("带尾随逗号的 JSON 应被 repair 后返回合法 Value");

    assert_eq!(value["a"], 1, "应返回 {{\"a\":1}}");
    assert!(value.is_object(), "应为 object");
    assert_eq!(
        value.as_object().unwrap().len(),
        1,
        "应仅含 1 个字段（trailing comma 已被剥离）"
    );

    // 仅 1 次 complete 调用（repair 在本地完成，无需再次请求 LLM）
    assert_eq!(
        provider.call_count(),
        1,
        "首次 repair 成功后不应再次调用 complete"
    );
}
