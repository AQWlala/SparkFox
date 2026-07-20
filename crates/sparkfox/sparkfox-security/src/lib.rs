//! SparkFox Security — 11 层安全栈
//!
//! 融合 OpenAkita 安全设计 + Pangu Nebula 6 层记忆融合架构，
//! SparkFox 安全栈分 11 层，覆盖输入 → 处理 → 输出 → 持久化全链路。
//!
//! ## 11 层清单
//! 1. 输入校验（[`SecurityLayer::InputValidation`]）
//! 2. Prompt 注入防御（[`prompt_defense`] 模块，S-03 P0 修复）
//! 3. LLM 审计日志（[`audit`] 模块，S-01 P0 修复）
//! 4. E2EE 加密（依赖 `sparkfox-e2ee`，Double Ratchet）
//! 5. CRDT 冲突解决（依赖 `sparkfox-crdt`）
//! 6. 模型 SHA256 校验（依赖 `sparkfox-embedding`）
//! 7. 文件解析安全（依赖 `sparkfox-parser`）
//! 8. SQL 注入防御
//! 9. 路径遍历防御
//! 10. 资源限制（CPU / 内存 / 时间）
//! 11. 隐私保护（PII 识别 / 脱敏）
//!
//! ## 已实现
//! - [`audit`]：LLM 调用审计日志（S-01 P0 修复），仅本地不同步
//! - [`prompt_defense`]：Prompt 注入防御（S-03 P0 修复）
//! - [`mcp_audit`]：MCP 工具调用审计日志（Task 9.3），仅本地不同步

#![forbid(unsafe_code)]

pub mod audit;
pub mod mcp_audit;
pub mod prompt_defense;

pub use audit::{AuditEntry, LlmAuditLogger};
pub use mcp_audit::{McpAuditEntry, McpAuditLogger};
pub use prompt_defense::{
    assess_injection_risk, detect_injection_patterns, escape_document_content,
    wrap_document_prompt, InjectionRiskLevel,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 11 层安全栈枚举
///
/// 用于枚举 / 注册 / 文档化 SparkFox 的安全层。各层实际实现分散在
/// 对应 crate 中（如 `sparkfox-e2ee` 实现 E2EE，`sparkfox-parser` 实现
/// 文件解析安全），本枚举仅作为路由与文档索引。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLayer {
    /// L1 输入校验层
    InputValidation,
    /// L2 Prompt 注入防御层（[`prompt_defense`]）
    PromptInjectionDefense,
    /// L3 LLM 审计日志层（[`audit`]）
    LlmAuditLog,
    /// L4 E2EE 加密层（`sparkfox-e2ee`）
    E2eeEncryption,
    /// L5 CRDT 冲突解决层（`sparkfox-crdt`）
    CrdtConflictResolution,
    /// L6 模型 SHA256 校验层（`sparkfox-embedding`）
    ModelSha256Verification,
    /// L7 文件解析安全层（`sparkfox-parser`）
    FileParseSafety,
    /// L8 SQL 注入防御层
    SqlInjectionDefense,
    /// L9 路径遍历防御层
    PathTraversalDefense,
    /// L10 资源限制层（CPU / 内存 / 时间）
    ResourceLimit,
    /// L11 隐私保护层（PII 识别 / 脱敏）
    PrivacyProtection,
}

impl SecurityLayer {
    /// 层级编号（1-11）
    pub fn level(&self) -> u8 {
        match self {
            Self::InputValidation => 1,
            Self::PromptInjectionDefense => 2,
            Self::LlmAuditLog => 3,
            Self::E2eeEncryption => 4,
            Self::CrdtConflictResolution => 5,
            Self::ModelSha256Verification => 6,
            Self::FileParseSafety => 7,
            Self::SqlInjectionDefense => 8,
            Self::PathTraversalDefense => 9,
            Self::ResourceLimit => 10,
            Self::PrivacyProtection => 11,
        }
    }

    /// 层级短名（英文标识，用于配置 / 日志）
    pub fn name(&self) -> &'static str {
        match self {
            Self::InputValidation => "input_validation",
            Self::PromptInjectionDefense => "prompt_injection_defense",
            Self::LlmAuditLog => "llm_audit_log",
            Self::E2eeEncryption => "e2ee_encryption",
            Self::CrdtConflictResolution => "crdt_conflict_resolution",
            Self::ModelSha256Verification => "model_sha256_verification",
            Self::FileParseSafety => "file_parse_safety",
            Self::SqlInjectionDefense => "sql_injection_defense",
            Self::PathTraversalDefense => "path_traversal_defense",
            Self::ResourceLimit => "resource_limit",
            Self::PrivacyProtection => "privacy_protection",
        }
    }

    /// 层级中文描述
    pub fn description(&self) -> &'static str {
        match self {
            Self::InputValidation => "输入校验层：对所有外部输入进行 schema / 长度 / 字符集校验",
            Self::PromptInjectionDefense => {
                "Prompt 注入防御层：转义文档内容，检测注入模式（S-03 P0 修复）"
            }
            Self::LlmAuditLog => {
                "LLM 审计日志层：记录每次 LLM 调用，仅本地不同步（S-01 P0 修复）"
            }
            Self::E2eeEncryption => "E2EE 加密层：Double Ratchet 端到端加密（sparkfox-e2ee）",
            Self::CrdtConflictResolution => {
                "CRDT 冲突解决层：跨设备记忆同步自动合并（sparkfox-crdt）"
            }
            Self::ModelSha256Verification => {
                "模型 SHA256 校验层：本地模型完整性校验（sparkfox-embedding）"
            }
            Self::FileParseSafety => "文件解析安全层：沙箱化文档 / 表格解析（sparkfox-parser）",
            Self::SqlInjectionDefense => "SQL 注入防御层：参数化查询 + 输入过滤",
            Self::PathTraversalDefense => "路径遍历防御层：路径规范化 + 白名单根目录",
            Self::ResourceLimit => "资源限制层：CPU / 内存 / 时间配额，防 DoS",
            Self::PrivacyProtection => "隐私保护层：PII 识别与脱敏（nomi-redact）",
        }
    }

    /// 全部 11 层（按 L1-L11 顺序）
    pub fn all() -> &'static [SecurityLayer] {
        &[
            Self::InputValidation,
            Self::PromptInjectionDefense,
            Self::LlmAuditLog,
            Self::E2eeEncryption,
            Self::CrdtConflictResolution,
            Self::ModelSha256Verification,
            Self::FileParseSafety,
            Self::SqlInjectionDefense,
            Self::PathTraversalDefense,
            Self::ResourceLimit,
            Self::PrivacyProtection,
        ]
    }
}

/// 初始化日志（幂等）
pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-security v{} initialized", VERSION);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_layers_count_eleven() {
        assert_eq!(SecurityLayer::all().len(), 11, "安全栈必须是 11 层");
    }

    #[test]
    fn levels_are_sequential_1_to_11() {
        let levels: Vec<u8> = SecurityLayer::all().iter().map(|l| l.level()).collect();
        assert_eq!(levels, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    }

    #[test]
    fn names_and_descriptions_non_empty() {
        for layer in SecurityLayer::all() {
            assert!(!layer.name().is_empty(), "L{} name 为空", layer.level());
            assert!(
                !layer.description().is_empty(),
                "L{} description 为空",
                layer.level()
            );
        }
    }

    #[test]
    fn version_non_empty() {
        assert!(!VERSION.is_empty());
    }
}
