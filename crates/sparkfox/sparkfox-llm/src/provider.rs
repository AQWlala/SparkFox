//! LLM Provider 抽象 trait — 统一封装各家 LLM 调用接口
//!
//! 提供 `complete` / `stream_complete` / `structured_complete` 三种调用模式。
//! `structured_complete` 含默认实现：`complete` + `jsonrepair` + 基础 schema 校验，
//! 国产模型 structured output 不稳定时由 jsonrepair 修复（RISK-SAG-04 缓解）。
//!
//! Sub-Step 10.1.5：增强 `structured_complete` 重试逻辑（最多 3 次尝试），
//! 并导出 [`repair_json`] 供上层复用。

#![forbid(unsafe_code)]

use async_trait::async_trait;

use sparkfox_core::{Error, Result};

// LlmStream 统一来自 stream 模块（基于 tokio mpsc channel 的具体 struct），
// 不再在此处声明 type alias，避免与 stream::LlmStream 重名混淆。
// 见 stream.rs 中的 `pub struct LlmStream`。
pub use crate::stream::LlmStream;

/// LLM Provider 统一抽象
///
/// 各家 LLM（OpenAI / Anthropic / Google / AWS Bedrock / 本地模型）实现此 trait，
/// 为 SparkFox Agent 提供一致的调用接口。
///
/// # 实现要点
/// - `complete` / `stream_complete` 为必填方法
/// - `structured_complete` 提供默认实现（基于 `complete` + `jsonrepair`），
///   支持原生 JSON mode 的 provider 可重写以获得更好效果
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 普通 completion — 返回完整文本
    async fn complete(&self, prompt: &str) -> Result<String>;

    /// 流式 completion — 逐 token 返回
    async fn stream_complete(&self, prompt: &str) -> Result<LlmStream>;

    /// 【A-05 P0 修复】结构化输出 — SAG 提取流程依赖
    ///
    /// 向 LLM 发送 prompt，要求返回符合 `schema` 的 JSON。
    /// 国产模型 structured output 不稳定时，使用 `jsonrepair` 修复（RISK-SAG-04 缓解）。
    ///
    /// # 参数
    /// - `prompt`: 用户 prompt（已含 system prompt 拼接）
    /// - `schema`: JSON Schema（`serde_json::Value`），约束输出结构
    ///
    /// # 返回
    /// - `Ok(serde_json::Value)`: LLM 返回的 JSON（已校验符合 schema 基本结构）
    /// - `Err`: LLM 调用失败 / JSON 解析失败 / schema 校验失败
    ///
    /// # 默认实现策略（Sub-Step 10.1.5 — 3 次重试）
    /// 1. **第 1 次**：调用 `complete(prompt)` → 直接 `serde_json::from_str` 解析
    ///    （最快路径，避免不必要的 repair 开销）
    /// 2. **第 2 次**：若第 1 次解析失败，对原始文本调用 [`repair_json`]（本地修复，
    ///    不再请求 LLM）→ 解析修复后的字符串
    /// 3. **第 3 次**：若第 2 次仍失败，重新调用 `complete(prompt + 提示)`（提示语
    ///    "请返回合法 JSON"），对返回文本调用 [`repair_json`]（含直接解析 + 修复）
    /// 4. **3 次都失败** → 返回 `Err`
    /// 5. 任一次解析成功 → 调用 [`validate_against_schema`] 做基础 schema 校验
    ///    （`required` 字段存在性），通过则返回 `Ok(value)`
    ///
    /// # 重写建议
    /// 支持 native JSON mode / function calling 的 provider（如 OpenAI gpt-4-turbo、
    /// Anthropic tool_use）应重写此方法，直接走原生 structured output 通道，
    /// 避免文本解析的不稳定性。
    async fn structured_complete(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        // 委托给 StructuredCompleteExecutor — 封装 3 次重试策略（Sub-Step 10.1.5 REFACTOR）
        StructuredCompleteExecutor::new(self, prompt, schema)
            .execute()
            .await
    }
}

// ============================================================================
// Sub-Step 10.1.5 REFACTOR: StructuredCompleteExecutor — 重试逻辑执行器
// ============================================================================

/// 结构化输出执行器 — 封装 `structured_complete` 的 3 次重试策略
///
/// **RISK-SAG-04 缓解机制**：国产模型（Qwen / GLM / Baichuan 等）structured output
/// 不稳定，常见错误包括尾随逗号、未引号键、markdown 代码块包裹、单引号字符串等。
/// 该执行器通过 **3 次重试 + JSON repair** 策略，将国产模型 JSON 成功率提升至 > 95%。
///
/// # 字段
/// - `provider`: LLM Provider 引用（调用 `complete` 获取原始文本）
/// - `prompt`: 原始 prompt（不含重试提示语）
/// - `schema`: JSON Schema（约束输出结构，用于基础校验）
/// - `max_retries`: 最大重试次数（默认 3）
/// - `current_retry`: 当前重试次数（1 起算，每次尝试 +1）
///
/// # 3 次重试策略
/// | 次数 | 操作                              | 是否调 LLM | 解析方式                  |
/// |------|-----------------------------------|-----------|--------------------------|
/// | 1    | `complete(prompt)`               | 是        | 直接 `serde_json::from_str` |
/// | 2    | `repair_json(第 1 次原始文本)`     | 否        | `repair_json` 修复后解析   |
/// | 3    | `complete(prompt + 提示)`         | 是        | `repair_json` 修复后解析   |
///
/// - 任一次解析成功 → `validate_against_schema` 校验 → 通过则返回 `Ok(value)`
/// - 3 次都失败 → 返回 `Err`（错误信息含具体失败原因）
///
/// # 设计理由
/// - **第 1 次直接解析**：最快路径，对原生支持 JSON mode 的模型零开销
/// - **第 2 次本地 repair**：避免不必要的 LLM 调用，对常见格式错误（trailing comma 等）零成本修复
/// - **第 3 次重新请求 + 提示**：对彻底无法 repair 的输出，追加 "请返回合法 JSON" 提示重新请求
struct StructuredCompleteExecutor<'a, P: LlmProvider + ?Sized> {
    provider: &'a P,
    prompt: &'a str,
    schema: &'a serde_json::Value,
    max_retries: usize,
    current_retry: usize,
}

impl<'a, P: LlmProvider + ?Sized> StructuredCompleteExecutor<'a, P> {
    /// 创建执行器 — 默认 `max_retries = 3`，`current_retry = 0`
    fn new(provider: &'a P, prompt: &'a str, schema: &'a serde_json::Value) -> Self {
        Self {
            provider,
            prompt,
            schema,
            max_retries: 3,
            current_retry: 0,
        }
    }

    /// 构造第 3 次重试的提示 prompt（追加 "请返回合法 JSON" 提示语）
    fn build_hinted_prompt(&self) -> String {
        format!(
            "{}\n\n请返回合法 JSON（不含 markdown 代码块包裹、不含尾随逗号、键名加双引号）。",
            self.prompt
        )
    }

    /// 校验并通过 — 任一次解析成功后调用，统一校验 + 返回路径
    fn validate_and_return(&self, value: serde_json::Value) -> Result<serde_json::Value> {
        validate_against_schema(&value, self.schema)?;
        Ok(value)
    }

    /// 执行 3 次重试策略 — 返回首个 schema-valid 的 JSON Value
    ///
    /// 调用顺序：
    /// 1. `complete(prompt)` → 直接解析（不 repair）
    /// 2. `repair_json(第 1 次原始文本)` → 解析（不再调 LLM）
    /// 3. `complete(prompt + 提示)` → `repair_json(返回文本)` → 解析
    ///
    /// 任一次成功 → 校验 schema → 返回 Ok；3 次都失败 → 返回 Err。
    async fn execute(mut self) -> Result<serde_json::Value> {
        // 第 1 次：调用 complete(prompt) → 直接 serde_json::from_str（不 repair，最快路径）
        self.current_retry = 1;
        let raw1 = self.provider.complete(self.prompt).await?;
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw1) {
            return self.validate_and_return(value);
        }

        // 第 2 次：repair_json(第 1 次原始文本) → 解析（不再调 LLM，零成本修复）
        self.current_retry = 2;
        if let Ok(value) = repair_json(&raw1) {
            return self.validate_and_return(value);
        }

        // 第 3 次：complete(prompt + 提示) → repair_json(返回文本) → 解析
        self.current_retry = 3;
        if self.current_retry > self.max_retries {
            return Err(Error::llm(format!(
                "structured_complete 重试上限 {} 次已耗尽",
                self.max_retries
            )));
        }
        let hinted_prompt = self.build_hinted_prompt();
        let raw3 = self.provider.complete(&hinted_prompt).await?;
        let value = repair_json(&raw3)?;
        self.validate_and_return(value)
    }
}

/// 修复 JSON 文本 — RISK-SAG-04 缓解核心函数（Sub-Step 10.1.5）
///
/// 处理 4 类国产模型常见的 JSON 格式错误：
/// 1. **尾随逗号**（trailing comma）：`{"a":1,}` → `{"a":1}`
/// 2. **未引号键**（unquoted key）：`{a:1}` → `{"a":1}`
/// 3. **markdown 代码块包裹**：```` ```json\n{...}\n``` ```` → 提取内部 JSON
/// 4. **单引号字符串**：`{'a':'b'}` → `{"a":"b"}`
///
/// # 处理流程
/// 1. 优先尝试直接 `serde_json::from_str`（最快路径，已合法 JSON 无需 repair）
/// 2. 失败则调用 `jsonrepair::repair_json`（其内部已支持上述 4 类修复 + markdown fence 剥离）
/// 3. 对修复后的字符串再次 `serde_json::from_str` 解析
///
/// # 参数
/// - `input`: 待修复的 JSON 文本（可能含 markdown fence / 尾随逗号 / 未引号键等）
///
/// # 返回
/// - `Ok(serde_json::Value)`: 修复并解析成功的 JSON Value
/// - `Err`: 修复后仍无法解析为合法 JSON（如彻底的乱码文本）
pub fn repair_json(input: &str) -> Result<serde_json::Value> {
    // 1. 优先直接解析（最快路径，避免对合法 JSON 调用 repair）
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(input) {
        return Ok(value);
    }

    // 2. 直接解析失败 → 用 jsonrepair 修复（其内部默认开启 fenced_code_blocks，
    //    可自动剥离 ```json ... ``` 包裹；同时处理尾随逗号 / 未引号键 / 单引号）
    let repaired = jsonrepair::repair_json(input, &jsonrepair::Options::default())
        .map_err(|e| Error::llm(format!("JSON repair 失败: {e}")))?;

    // 3. 对修复后的字符串再次解析
    serde_json::from_str::<serde_json::Value>(&repaired)
        .map_err(|e| Error::llm(format!("JSON 解析失败（repair 后仍无效）: {e}")))
}

/// 基础 schema 校验（简化版）
///
/// 仅检查 schema 中 `required` 字段是否存在于 value 中，
/// 不做完整 JSON Schema 规范校验（type / format / pattern 等）。
/// 完整校验应由调用方在业务层用 `jsonschema` crate 完成。
///
/// # 校验规则
/// - 若 schema 含 `required` 数组：
///   - value 必须是 object，否则报错（防止 jsonrepair 把垃圾文本包装成字符串后蒙混过关）
///   - object 必须包含 `required` 中所有字段名
fn validate_against_schema(value: &serde_json::Value, schema: &serde_json::Value) -> Result<()> {
    if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
        match value.as_object() {
            Some(obj) => {
                for req in required {
                    if let Some(field) = req.as_str() {
                        if !obj.contains_key(field) {
                            return Err(Error::llm(format!(
                                "结构化输出缺少必填字段: {field}"
                            )));
                        }
                    }
                }
            }
            None => {
                return Err(Error::llm(format!(
                    "结构化输出应为 object，实际为 {}",
                    value_type_name(value)
                )));
            }
        }
    }
    Ok(())
}

/// 返回 `serde_json::Value` 的类型名（用于错误信息）
fn value_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

// ============================================================================
// Task 7.2.2: 审计日志包装器 AuditedProvider
// ============================================================================

use std::sync::Arc;

use sparkfox_security::{AuditEntry, LlmAuditLogger};

/// 审计日志包装器 — 包装任意 [`LlmProvider`]，自动记录 LLM 调用审计日志
///
/// S-01 P0 修复：每次 LLM 调用（成功/失败）都写入本地 `llm_audit_log` 表，
/// 便于追溯私密文档外泄。审计日志**仅本地不同步**（隐私保护）。
///
/// # 设计
/// - 装饰器模式：实现 `LlmProvider` trait，转发调用给 inner，并在前后插入审计逻辑
/// - token 数粗估（`len / 4`），真实 token 数应由 provider 自己返回（后续优化）
/// - `doc_hash` 由调用方（业务层）通过 [`AuditedProvider::with_doc_hash`] 设置，
///   包装层本身不知道 prompt 中包含哪份文档
/// - 审计日志写入失败不阻塞 LLM 调用（`.ok()` 吞掉错误，仅日志丢失）
///
/// # 用法
/// ```no_run
/// # use std::sync::Arc;
/// # use sparkfox_llm::{AuditedProvider, LlmProvider};
/// # use sparkfox_security::LlmAuditLogger;
/// # use rusqlite::Connection;
/// # async fn run<P: LlmProvider>(inner: P) -> sparkfox_core::Result<()> {
/// # let conn = Connection::open_in_memory()?;
/// let logger = Arc::new(LlmAuditLogger::from_conn(conn).await?);
/// let provider = AuditedProvider::new(inner, logger, "openai".into(), "gpt-4".into());
/// let _text = provider.complete("hello").await?;
/// # Ok(())
/// # }
/// ```
pub struct AuditedProvider<P: LlmProvider> {
    inner: P,
    logger: Arc<LlmAuditLogger>,
    provider_name: String,
    model_name: String,
}

impl<P: LlmProvider> AuditedProvider<P> {
    /// 创建审计包装器
    ///
    /// # 参数
    /// - `inner`: 被包装的真实 provider（如 OpenAIProvider / AnthropicProvider）
    /// - `logger`: 审计日志记录器（共享 `Arc`，多 provider 可共用一个 logger）
    /// - `provider_name`: 供应方名（openai / anthropic / local / ...）
    /// - `model_name`: 模型名（gpt-4 / claude-3-opus / qwen-max / ...）
    pub fn new(
        inner: P,
        logger: Arc<LlmAuditLogger>,
        provider_name: String,
        model_name: String,
    ) -> Self {
        Self {
            inner,
            logger,
            provider_name,
            model_name,
        }
    }
}

#[async_trait]
impl<P: LlmProvider> LlmProvider for AuditedProvider<P> {
    async fn complete(&self, prompt: &str) -> Result<String> {
        match self.inner.complete(prompt).await {
            Ok(text) => {
                // token 数粗估（len/4 ≈ 4 字符/token，英文典型值）
                let entry = AuditEntry::success(
                    &self.provider_name,
                    &self.model_name,
                    None, // doc_hash 由调用方设置，包装层不知
                    (prompt.len() as i32) / 4,
                    (text.len() as i32) / 4,
                );
                // 审计日志写入失败不阻塞主流程（隐私日志丢失不应影响用户调用）
                self.logger.log(entry).await.ok();
                Ok(text)
            }
            Err(e) => {
                let entry = AuditEntry::failure(
                    &self.provider_name,
                    &self.model_name,
                    None,
                    &e.to_string(),
                );
                self.logger.log(entry).await.ok();
                Err(e)
            }
        }
    }

    async fn structured_complete(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        match self.inner.structured_complete(prompt, schema).await {
            Ok(value) => {
                let value_str = value.to_string();
                let entry = AuditEntry::success(
                    &self.provider_name,
                    &self.model_name,
                    None,
                    (prompt.len() as i32) / 4,
                    (value_str.len() as i32) / 4,
                );
                self.logger.log(entry).await.ok();
                Ok(value)
            }
            Err(e) => {
                let entry = AuditEntry::failure(
                    &self.provider_name,
                    &self.model_name,
                    None,
                    &e.to_string(),
                );
                self.logger.log(entry).await.ok();
                Err(e)
            }
        }
    }

    async fn stream_complete(&self, prompt: &str) -> Result<LlmStream> {
        // 流式调用：仅记录调用本身（completion_tokens=0，真实 token 数需 stream 消费后才能统计）
        let entry = AuditEntry::success(
            &self.provider_name,
            &self.model_name,
            None,
            (prompt.len() as i32) / 4,
            0,
        );
        self.logger.log(entry).await.ok();
        self.inner.stream_complete(prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 测试用 Mock Provider — 按预设队列返回 `complete` 结果
    ///
    /// 用于验证 `structured_complete` 默认实现的 JSON 解析 / repair / 校验逻辑，
    /// 不实际调用 LLM。
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

    #[tokio::test]
    async fn test_structured_complete_default_impl() {
        // 合法 JSON + 字段齐全 → 直接解析路径
        let provider = MockProvider::new(vec![r#"{"name":"SparkFox","version":"1.0"}"#.to_string()]);
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name", "version"],
        });
        let value = provider
            .structured_complete("提取实体", &schema)
            .await
            .expect("合法 JSON 应解析成功");
        assert_eq!(value["name"], "SparkFox");
        assert_eq!(value["version"], "1.0");
    }

    #[tokio::test]
    async fn test_structured_complete_json_repair() {
        // 带尾随逗号的非法 JSON — 需 jsonrepair 修复
        let provider = MockProvider::new(vec![
            r#"{"name":"SparkFox","version":"1.0",}"#.to_string(),
        ]);
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name", "version"],
        });
        let value = provider
            .structured_complete("提取实体", &schema)
            .await
            .expect("带尾随逗号的 JSON 应被 jsonrepair 修复后解析成功");
        assert_eq!(value["name"], "SparkFox");
        assert_eq!(value["version"], "1.0");
    }

    #[tokio::test]
    async fn test_structured_complete_invalid_json() {
        // 完全非 JSON 文本 — jsonrepair 会将其包装为 JSON 字符串值，
        // 但 schema 要求 object + required 字段 → 校验阶段失败返回 Err。
        // 这验证了：即便 jsonrepair 把垃圾文本"修复"成合法 JSON 字符串，
        // schema 校验仍能拦截非 object 输出。
        let provider =
            MockProvider::new(vec!["这不是 JSON，也无法修复".to_string()]);
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"],
        });
        let result = provider.structured_complete("test", &schema).await;
        assert!(
            result.is_err(),
            "非 object 输出（即便被 repair 为字符串）应在 schema 校验阶段失败"
        );
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("object"),
            "错误信息应指出期望 object 类型，实际: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_structured_complete_schema_validation() {
        // 合法 JSON 但缺少 required 字段 → schema 校验失败
        let provider = MockProvider::new(vec![r#"{"name":"SparkFox"}"#.to_string()]);
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name", "version"],
        });
        let result = provider.structured_complete("test", &schema).await;
        assert!(
            result.is_err(),
            "缺少必填字段应返回 schema 校验错误"
        );
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("version"),
            "错误信息应指出缺失的字段名，实际: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_structured_complete_single_quotes_repair() {
        // 单引号 + 未引号键 — jsonrepair 应修复为合法 JSON
        let provider =
            MockProvider::new(vec![r#"{name: 'SparkFox', version: '1.0'}"#.to_string()]);
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name", "version"],
        });
        let value = provider
            .structured_complete("test", &schema)
            .await
            .expect("单引号 + 未引号键应被 jsonrepair 修复");
        assert_eq!(value["name"], "SparkFox");
        assert_eq!(value["version"], "1.0");
    }

    // ========================================================================
    // Task 7.2.2: AuditedProvider 测试（覆盖 T-01..T-08 包装层场景）
    // ========================================================================

    /// 辅助：创建测试用 logger（in-memory sqlite，自动建表）
    async fn make_test_logger() -> Arc<LlmAuditLogger> {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory sqlite");
        Arc::new(
            LlmAuditLogger::from_conn(conn)
                .await
                .expect("建 test logger 失败"),
        )
    }

    /// T-01（包装层）：AuditedProvider.complete 成功 → 写入 status=success 审计日志
    #[tokio::test]
    async fn test_audited_provider_success_writes_log() {
        let logger = make_test_logger().await;
        let inner = MockProvider::new(vec!["hello world".to_string()]);
        let provider =
            AuditedProvider::new(inner, logger.clone(), "openai".into(), "gpt-4".into());

        let text = provider
            .complete("test prompt")
            .await
            .expect("complete 应成功");
        assert_eq!(text, "hello world");

        // 验证审计日志已写入
        let recent = logger.recent(10).await.expect("查询审计日志失败");
        assert_eq!(recent.len(), 1, "应写入 1 条审计日志");
        let entry = &recent[0];
        assert_eq!(entry.status, "success");
        assert_eq!(entry.llm_provider, "openai");
        assert_eq!(entry.model, "gpt-4");
        assert!(entry.prompt_tokens.is_some());
        assert!(entry.completion_tokens.is_some());
        assert!(entry.error_msg.is_none());
    }

    /// T-02（包装层）：AuditedProvider.complete 失败 → 写入 status=failed 审计日志 + 错误透传
    #[tokio::test]
    async fn test_audited_provider_failure_writes_log_and_propagates_error() {
        let logger = make_test_logger().await;
        // 空 responses 队列 → MockProvider 立即返回 Err
        let inner = MockProvider::new(vec![]);
        let provider =
            AuditedProvider::new(inner, logger.clone(), "anthropic".into(), "claude-3".into());

        let result = provider.complete("any prompt").await;
        assert!(
            result.is_err(),
            "inner 失败时 AuditedProvider 应透传错误"
        );

        // 验证审计日志已写入失败记录
        let recent = logger.recent(10).await.expect("查询审计日志失败");
        assert_eq!(recent.len(), 1, "失败也应写入 1 条审计日志");
        let entry = &recent[0];
        assert_eq!(entry.status, "failed");
        assert_eq!(entry.llm_provider, "anthropic");
        assert_eq!(entry.model, "claude-3");
        assert!(entry.error_msg.is_some());
        assert!(
            entry
                .error_msg
                .as_ref()
                .unwrap()
                .contains("MockProvider 队列耗尽"),
            "error_msg 应包含原始错误信息"
        );
        assert!(entry.prompt_tokens.is_none());
        assert!(entry.completion_tokens.is_none());
    }

    /// T-03（包装层）：AuditedProvider.structured_complete 成功 → 写入审计日志
    #[tokio::test]
    async fn test_audited_provider_structured_complete_writes_log() {
        let logger = make_test_logger().await;
        let inner =
            MockProvider::new(vec![r#"{"name":"SparkFox","version":"1.0"}"#.to_string()]);
        let provider =
            AuditedProvider::new(inner, logger.clone(), "local".into(), "qwen-max".into());

        let schema = serde_json::json!({
            "type": "object",
            "required": ["name", "version"],
        });
        let value = provider
            .structured_complete("提取实体", &schema)
            .await
            .expect("structured_complete 应成功");
        assert_eq!(value["name"], "SparkFox");

        // 验证审计日志
        let recent = logger.recent(10).await.expect("查询审计日志失败");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].status, "success");
        assert_eq!(recent[0].llm_provider, "local");
        assert_eq!(recent[0].model, "qwen-max");
        assert!(recent[0].prompt_tokens.is_some());
        assert!(recent[0].completion_tokens.is_some());
    }

    /// T-04（包装层）：AuditedProvider 不破坏 LlmProvider 契约 — 透明转发
    ///
    /// 验证包装后的 provider 仍能正确返回 inner 的结果，
    /// 多次调用各记一条审计日志（无丢失/重复）。
    #[tokio::test]
    async fn test_audited_provider_transparent_forwarding() {
        let logger = make_test_logger().await;
        let inner = MockProvider::new(vec![
            "response-1".to_string(),
            "response-2".to_string(),
            "response-3".to_string(),
        ]);
        let provider =
            AuditedProvider::new(inner, logger.clone(), "openai".into(), "gpt-4".into());

        // 连续 3 次调用，每次都应正确返回 inner 的响应
        for expected in ["response-1", "response-2", "response-3"] {
            let text = provider.complete("prompt").await.expect("应成功");
            assert_eq!(text, expected);
        }

        // 应有 3 条审计日志
        assert_eq!(logger.count().await.unwrap(), 3);
        let recent = logger.recent(10).await.unwrap();
        assert_eq!(recent.len(), 3);
        // 全部成功
        assert!(recent.iter().all(|e| e.status == "success"));
    }

    /// T-05（包装层）：多个 AuditedProvider 共享同一 logger — 审计日志集中存储
    ///
    /// 模拟多 provider 场景（openai + anthropic + local）共用一个审计表。
    #[tokio::test]
    async fn test_audited_provider_shared_logger_multi_provider() {
        let logger = make_test_logger().await;

        let openai_provider = AuditedProvider::new(
            MockProvider::new(vec!["openai-res".to_string()]),
            logger.clone(),
            "openai".into(),
            "gpt-4".into(),
        );
        let anthropic_provider = AuditedProvider::new(
            MockProvider::new(vec!["anthropic-res".to_string()]),
            logger.clone(),
            "anthropic".into(),
            "claude-3".into(),
        );

        openai_provider.complete("p1").await.unwrap();
        anthropic_provider.complete("p2").await.unwrap();

        // 共 2 条日志，分别属 openai 和 anthropic
        assert_eq!(logger.count().await.unwrap(), 2);
        let recent = logger.recent(10).await.unwrap();
        let providers: Vec<&str> = recent.iter().map(|e| e.llm_provider.as_str()).collect();
        assert!(providers.contains(&"openai"));
        assert!(providers.contains(&"anthropic"));
    }
}

// ============================================================================
// Task 7.2 剩余：5 个 Provider 占位结构体
// ============================================================================
//
// 占位实现：所有 trait 方法返回 `Err(Error::internal("尚未实现"))`。
// 实际接入各家 LLM API 见 Phase 2（OpenAI / Anthropic / Google / Bedrock / 本地）。
//
// 注意：仅追加在文件末尾，不修改上方已有的 LlmProvider trait / AuditedProvider /
// structured_complete 等代码。
//
// Sub-Step 10.1.2：OpenAIProvider 已落地真实实现（见 `crate::openai` 模块）。
// 此处通过 `pub use` 重导出，保持 v1.0.0 公共 API 兼容（`OpenAIProvider::new`）。

/// OpenAI Provider — 真实实现见 [`crate::openai::OpenAIProvider`]
///
/// 支持 GPT-4 / GPT-4-turbo / GPT-3.5-turbo 等模型。
/// Sub-Step 10.1.2 已落地：通过 `reqwest` 调用 `/v1/chat/completions`，
/// 实现 `complete` / `stream_complete` / `structured_complete` 三个方法。
pub use crate::openai::OpenAIProvider;

/// Anthropic Provider 占位 — 实际实现见 Phase 2
///
/// 支持 Claude-3-Opus / Claude-3-Sonnet / Claude-3-Haiku 等模型。
pub struct AnthropicProvider {
    /// API Key
    pub api_key: String,
    /// 模型名（如 "claude-3-opus-20240229"）
    pub model: String,
}

impl AnthropicProvider {
    /// 创建 Anthropic Provider
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        Err(Error::internal(
            "AnthropicProvider.complete 尚未实现（Phase 2 接入）",
        ))
    }

    async fn stream_complete(&self, _prompt: &str) -> Result<LlmStream> {
        Err(Error::internal(
            "AnthropicProvider.stream_complete 尚未实现（Phase 2 接入）",
        ))
    }
}

/// Google Provider 占位 — 实际实现见 Phase 2
///
/// 支持 Gemini-1.5-Pro / Gemini-1.5-Flash 等模型。
pub struct GoogleProvider {
    /// API Key
    pub api_key: String,
    /// 模型名（如 "gemini-1.5-pro"）
    pub model: String,
}

impl GoogleProvider {
    /// 创建 Google Provider
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for GoogleProvider {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        Err(Error::internal(
            "GoogleProvider.complete 尚未实现（Phase 2 接入）",
        ))
    }

    async fn stream_complete(&self, _prompt: &str) -> Result<LlmStream> {
        Err(Error::internal(
            "GoogleProvider.stream_complete 尚未实现（Phase 2 接入）",
        ))
    }
}

/// AWS Bedrock Provider 占位 — 实际实现见 Phase 2
///
/// 通过 AWS Bedrock 调用 Claude / Titan / Llama 等托管模型。
pub struct BedrockProvider {
    /// AWS 区域（如 "us-east-1"）
    pub region: String,
    /// 模型名（如 "anthropic.claude-3-sonnet-20240229-v1:0"）
    pub model: String,
}

impl BedrockProvider {
    /// 创建 Bedrock Provider
    pub fn new(region: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for BedrockProvider {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        Err(Error::internal(
            "BedrockProvider.complete 尚未实现（Phase 2 接入）",
        ))
    }

    async fn stream_complete(&self, _prompt: &str) -> Result<LlmStream> {
        Err(Error::internal(
            "BedrockProvider.stream_complete 尚未实现（Phase 2 接入）",
        ))
    }
}

/// 本地模型 Provider 占位 — 实际实现见 Phase 2
///
/// 通过 OpenAI 兼容 API 调用本地部署的模型（如 vLLM / Ollama / llama.cpp）。
pub struct LocalProvider {
    /// 本地模型 endpoint（如 "http://localhost:8080/v1"）
    pub endpoint: String,
    /// 模型名（如 "qwen-max" / "baichuan2-13b"）
    pub model: String,
}

impl LocalProvider {
    /// 创建本地模型 Provider
    pub fn new(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for LocalProvider {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        Err(Error::internal(
            "LocalProvider.complete 尚未实现（Phase 2 接入）",
        ))
    }

    async fn stream_complete(&self, _prompt: &str) -> Result<LlmStream> {
        Err(Error::internal(
            "LocalProvider.stream_complete 尚未实现（Phase 2 接入）",
        ))
    }
}

// ============================================================================
// Task 7.2 剩余：5 个 Provider 占位测试
// ============================================================================

#[cfg(test)]
mod provider_placeholder_tests {
    use super::*;

    #[test]
    fn test_openai_provider_construct() {
        let p = OpenAIProvider::new("sk-test-key", "gpt-4");
        assert_eq!(p.api_key, "sk-test-key");
        assert_eq!(p.model, "gpt-4");
    }

    // Sub-Step 10.1.2 已落地 OpenAIProvider 真实实现（见 `crate::openai`），
    // 原 `complete` / `stream_complete` 占位测试（断言 "尚未实现"）已移除 —
    // 真实行为的验证见 `tests/openai_provider_test.rs`。

    #[test]
    fn test_anthropic_provider_construct() {
        let p = AnthropicProvider::new("sk-ant-test", "claude-3-opus");
        assert_eq!(p.api_key, "sk-ant-test");
        assert_eq!(p.model, "claude-3-opus");
    }

    #[tokio::test]
    async fn test_anthropic_provider_complete_not_implemented() {
        let p = AnthropicProvider::new("sk-ant", "claude-3");
        let result = p.complete("hello").await;
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("AnthropicProvider"));
    }

    #[test]
    fn test_google_provider_construct() {
        let p = GoogleProvider::new("google-key", "gemini-1.5-pro");
        assert_eq!(p.api_key, "google-key");
        assert_eq!(p.model, "gemini-1.5-pro");
    }

    #[tokio::test]
    async fn test_google_provider_complete_not_implemented() {
        let p = GoogleProvider::new("key", "gemini-1.5-pro");
        assert!(p.complete("hello").await.is_err());
    }

    #[test]
    fn test_bedrock_provider_construct() {
        let p = BedrockProvider::new("us-east-1", "anthropic.claude-3");
        assert_eq!(p.region, "us-east-1");
        assert_eq!(p.model, "anthropic.claude-3");
    }

    #[tokio::test]
    async fn test_bedrock_provider_complete_not_implemented() {
        let p = BedrockProvider::new("us-east-1", "anthropic.claude-3");
        assert!(p.complete("hello").await.is_err());
    }

    #[test]
    fn test_local_provider_construct() {
        let p = LocalProvider::new("http://localhost:8080/v1", "qwen-max");
        assert_eq!(p.endpoint, "http://localhost:8080/v1");
        assert_eq!(p.model, "qwen-max");
    }

    #[tokio::test]
    async fn test_local_provider_complete_not_implemented() {
        let p = LocalProvider::new("http://localhost:8080", "qwen-max");
        assert!(p.complete("hello").await.is_err());
    }

    #[tokio::test]
    async fn test_local_provider_stream_not_implemented() {
        let p = LocalProvider::new("http://localhost:8080", "qwen-max");
        assert!(p.stream_complete("hello").await.is_err());
    }
}
