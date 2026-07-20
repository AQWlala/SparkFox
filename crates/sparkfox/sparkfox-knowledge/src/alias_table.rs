//! Sub-Step 10.4.2 — AliasTable 别名表 + 审核日志（spec §三 10.5.2）
//!
//! ## 职责
//! 维护 `alias → canonical` 映射，将历史名 / 尊称 / 简称解析为同一 entity_id：
//! - **历史名**：「毛泽东」vs「毛润之」→ 同一 entity_id
//! - **尊称**：「孔子」vs「孔丘」vs「仲尼」→ 同一 entity_id
//! - **简称**：「北大」vs「北京大学」→ 同一 entity_id
//!
//! ## 数据来源
//! 种子数据位于 `config/alias.yaml`，由 [`AliasTable::load`] 加载；
//! 测试运行时 cwd = crate 根目录，可直接使用相对路径 `config/alias.yaml`。
//!
//! ## 审核日志（RISK-SAG-08）
//! 每次调用 [`AliasTable::resolve`] 都会追加一条 [`AliasAuditEntry`]，
//! 含 `raw` / `resolved_id` / `timestamp`，供 RISK-SAG-08 人工审核。
//!
//! ## 别名解析链路
//! 1. `AliasTable::resolve(raw)` 命中 → 返回 `Some(canonical)`
//! 2. 未命中 → 返回 `None`，由调用方回退到
//!    [`crate::entity_normalize::NfkcNormalizer`] + [`levenshtein_normalized`]
//!
//! ## 时间戳
//! 与 [`crate::saver`] 一致，使用固定值 `"2026-07-20T00:00:00Z"`，便于测试断言。

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::Mutex;

use sparkfox_core::{Error, Result};

/// 别名条目（从 alias.yaml 加载，对应 YAML 中一项）
#[derive(Debug, Clone, serde::Deserialize)]
struct AliasEntry {
    /// 规范名（canonical，作为 entity_id）
    canonical: String,
    /// 别名列表
    aliases: Vec<String>,
}

/// 别名解析审核日志条目（RISK-SAG-08 人工审核依据）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasAuditEntry {
    /// 原始输入文本
    pub raw: String,
    /// 解析结果（命中为 canonical，未命中为 None）
    pub resolved_id: Option<String>,
    /// 解析时间戳（与 saver 一致使用固定值）
    pub timestamp: String,
}

/// 别名表 — alias → canonical 映射 + 审核日志
///
/// 见模块级文档 [`crate::alias_table`]。
pub struct AliasTable {
    /// alias / canonical → canonical 映射
    map: HashMap<String, String>,
    /// 审核日志（`resolve()` 是 `&self`，用 `Mutex` 保证线程安全）
    audit_log: Mutex<Vec<AliasAuditEntry>>,
}

impl AliasTable {
    /// 从 YAML 文件加载别名表
    ///
    /// ## 参数
    /// - `path`: YAML 文件路径（相对路径基于 cwd）
    ///
    /// ## 错误
    /// - 文件读取失败 → `Error::Storage`
    /// - YAML 解析失败 → `Error::Parse`
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::storage(
                format!("读取 alias.yaml 失败: {e} (path={path})"),
                "AliasTable::load",
            )
        })?;
        Self::from_yaml(&content)
    }

    /// 从 YAML 字符串构造别名表
    ///
    /// YAML 格式（每项含 `canonical` + `aliases` 数组）：
    /// ```yaml
    /// - canonical: "毛泽东"
    ///   aliases: ["毛润之", "李德胜"]
    /// ```
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let entries: Vec<AliasEntry> = serde_yaml::from_str(yaml).map_err(|e| {
            Error::parse(
                format!("解析 alias.yaml 失败: {e}"),
                "AliasTable::from_yaml",
            )
        })?;
        let mut map = HashMap::new();
        for entry in entries {
            // canonical 自身也映射为 canonical（保证 resolve(canonical) 命中）
            map.insert(entry.canonical.clone(), entry.canonical.clone());
            for alias in entry.aliases {
                map.insert(alias, entry.canonical.clone());
            }
        }
        Ok(Self {
            map,
            audit_log: Mutex::new(Vec::new()),
        })
    }

    /// 解析别名，返回 canonical entity_id
    ///
    /// ## 返回
    /// - `Some(canonical)`：命中别名表
    /// - `None`：未命中，由调用方回退到 NFKC + 编辑距离
    ///
    /// ## 副作用
    /// 每次调用都追加一条 [`AliasAuditEntry`] 到审核日志。
    pub fn resolve(&self, raw: &str) -> Option<String> {
        let resolved = self.map.get(raw).cloned();
        let entry = AliasAuditEntry {
            raw: raw.to_string(),
            resolved_id: resolved.clone(),
            timestamp: fixed_timestamp(),
        };
        // Mutex::lock 失败仅发生在 Mutex 被 poison（持有锁的线程 panic）时，
        // 此处 unwrap 是合理的：若 poison 说明已发生不可恢复的 panic
        self.audit_log.lock().unwrap().push(entry);
        resolved
    }

    /// 获取审核日志的快照（克隆）
    pub fn audit_log(&self) -> Vec<AliasAuditEntry> {
        self.audit_log.lock().unwrap().clone()
    }

    /// 别名条目数量（含 canonical 自映射）
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// 固定时间戳（与 [`crate::saver`] 保持一致，便于测试断言）
///
/// 生产环境应替换为真实时间戳。
fn fixed_timestamp() -> String {
    "2026-07-20T00:00:00Z".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_yaml_basic() {
        let yaml = r#"
- canonical: "毛泽东"
  aliases: ["毛润之", "李德胜"]
- canonical: "北京大学"
  aliases: ["北大"]
"#;
        let table = AliasTable::from_yaml(yaml).expect("解析 YAML 失败");
        assert_eq!(table.resolve("毛泽东"), Some("毛泽东".to_string()));
        assert_eq!(table.resolve("毛润之"), Some("毛泽东".to_string()));
        assert_eq!(table.resolve("李德胜"), Some("毛泽东".to_string()));
        assert_eq!(table.resolve("北京大学"), Some("北京大学".to_string()));
        assert_eq!(table.resolve("北大"), Some("北京大学".to_string()));
        assert_eq!(table.resolve("未命中"), None);
        // map 含 5 个条目：毛泽东、毛润之、李德胜、北京大学、北大
        assert_eq!(table.len(), 5);
    }

    #[test]
    fn test_audit_log_appends() {
        let yaml = r#"
- canonical: "孔子"
  aliases: ["孔丘", "仲尼"]
"#;
        let table = AliasTable::from_yaml(yaml).expect("解析失败");
        table.resolve("孔子");
        table.resolve("孔丘");
        table.resolve("未命中");
        let log = table.audit_log();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].raw, "孔子");
        assert_eq!(log[0].resolved_id, Some("孔子".to_string()));
        assert_eq!(log[2].raw, "未命中");
        assert_eq!(log[2].resolved_id, None);
        // 时间戳为固定值
        assert_eq!(log[0].timestamp, "2026-07-20T00:00:00Z");
    }
}
