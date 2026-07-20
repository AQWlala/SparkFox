//! Sub-Step 10.3.1 — 7 段式 prompt 模板（D2.15 决策）
//!
//! ## 职责
//! - 定义 `PromptTemplate` trait + `PromptContext` 上下文
//! - 提供 `SevenSectionPrompt` 公共 7 段式骨架（REFACTOR 阶段提取）
//! - 提供 `NerPrompt` / `ExtractPrompt` 两个具体模板
//!
//! ## D2.15 决策 — 7 段式 prompt 结构
//! 1. 角色（Role）
//! 2. 任务（Task）
//! 3. 输入格式（Input Format）
//! 4. 输出格式（Output Format）
//! 5. 中文适配（Chinese Adaptation）
//! 6. few-shot（10 个中文示例）
//! 7. 约束（Constraints）
//!
//! ## 设计动机
//! - **统一规范**：所有 LLM prompt（NER / 事件提取 / 后续 Rerank / 归一化）共用
//!   同一 7 段骨架，便于审计、版本管理与 few-shot 维护。
//! - **中文优先**：每段均以中文撰写（除 JSON Schema 与字段名外），减少跨语言
//!   token 损耗，并适配中文分词 / 繁简体 / 中英混合等场景。
//! - **可测试**：7 段标题固定，便于自动化测试断言段落数量与覆盖范围。
//!
//! ## 不修改 lib.rs
//! 本子模块由 Sub-Step 10.3.1 新增，主 agent 在后续合并阶段会将 `pub mod prompt;`
//! 注册到 `lib.rs`。当前测试通过 `#[path = "../src/prompt/mod.rs"] mod prompt;` 绕过。

pub mod extract;
pub mod ner;

pub use extract::ExtractPrompt;
pub use ner::NerPrompt;

/// Prompt 渲染上下文
///
/// 字段说明：
/// - `chunk`：待识别 / 待提取的文本块（来自 `Chunker::chunk` 的输出）
/// - `entity_types`：本次识别涉及的实体类型英文枚举列表（来自 10.7.1 的 11 类配置）
#[derive(Debug, Clone)]
pub struct PromptContext {
    /// 待处理的文本块
    pub chunk: String,
    /// 实体类型英文枚举列表（如 `["PERSON", "LOCATION", ...]`）
    pub entity_types: Vec<String>,
}

/// Prompt 模板 trait
///
/// 实现方需保证 `render` 返回的字符串符合 7 段式结构（D2.15），
/// 且 `{chunk}` 占位符被 `context.chunk` 替换。
pub trait PromptTemplate {
    /// 渲染最终 prompt（已替换占位符）
    fn render(&self, context: &PromptContext) -> String;
}

/// 7 段式 prompt 公共骨架（REFACTOR 阶段提取，D2.15 决策）
///
/// 将 7 段固定结构（角色 / 任务 / 输入格式 / 输出格式 / 中文适配 / few-shot / 约束）
/// 与具体业务内容解耦：业务方只需填充 7 段内容，骨架负责按顺序拼接 + 替换 `{chunk}`。
///
/// ## 使用示例
/// ```ignore
/// let sections = SevenSection {
///     role: "## 角色\n你是...".to_string(),
///     task: "## 任务\n从文本中...".to_string(),
///     input_format: "## 输入格式\n...{chunk}...".to_string(),
///     output_format: "## 输出格式\n...".to_string(),
///     chinese_adaptation: "## 中文适配\n...".to_string(),
///     few_shot: "## few-shot\n...".to_string(),
///     constraints: "## 约束\n...".to_string(),
/// };
/// let prompt = SevenSectionPrompt::build(sections, &context);
/// ```
#[derive(Debug, Clone)]
pub struct SevenSection {
    /// 1. 角色（Role）
    pub role: String,
    /// 2. 任务（Task）
    pub task: String,
    /// 3. 输入格式（Input Format）— 含 `{chunk}` 占位符
    pub input_format: String,
    /// 4. 输出格式（Output Format）
    pub output_format: String,
    /// 5. 中文适配（Chinese Adaptation）
    pub chinese_adaptation: String,
    /// 6. few-shot（10 个中文示例）
    pub few_shot: String,
    /// 7. 约束（Constraints）
    pub constraints: String,
}

/// 7 段式 prompt 骨架工具
///
/// 提供 `build` 方法将 7 段内容按 D2.15 决策顺序拼接为完整 prompt，
/// 并替换 `{chunk}` 占位符为 `context.chunk`。
pub struct SevenSectionPrompt;

impl SevenSectionPrompt {
    /// 将 7 段内容拼接为完整 prompt，并替换 `{chunk}` 占位符
    ///
    /// 步骤：
    /// 1. 按 `角色 / 任务 / 输入格式 / 输出格式 / 中文适配 / few-shot / 约束` 顺序拼接
    /// 2. 段间以空行分隔（视觉清晰）
    /// 3. 将 `{chunk}` 占位符替换为 `context.chunk`
    pub fn build(sections: SevenSection, context: &PromptContext) -> String {
        let template = format!(
            "{role}\n\n{task}\n\n{input_format}\n\n{output_format}\n\n{chinese_adaptation}\n\n{few_shot}\n\n{constraints}",
            role = sections.role,
            task = sections.task,
            input_format = sections.input_format,
            output_format = sections.output_format,
            chinese_adaptation = sections.chinese_adaptation,
            few_shot = sections.few_shot,
            constraints = sections.constraints,
        );
        template.replace("{chunk}", &context.chunk)
    }
}
