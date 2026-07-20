//! Sub-Step 10.4.2 — AliasTable 别名表 + 审核日志（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 10.5.2）
//! 1. 从 config/alias.yaml 加载
//! 2. 解析历史名：「毛泽东」vs「毛润之」→ 同一 entity_id
//! 3. 解析尊称：「孔子」vs「孔丘」vs「仲尼」→ 同一 entity_id
//! 4. 解析简称：「北大」vs「北京大学」→ 同一 entity_id
//! 5. 未命中时回退到编辑距离
//! 6. 记录每次别名解析（审核日志）

#![forbid(unsafe_code)]

use sparkfox_knowledge::alias_table::AliasTable;

#[test]
fn test_alias_table_loads_from_yaml() {
    let table = AliasTable::load("config/alias.yaml").expect("加载 alias.yaml 失败");
    assert!(table.len() >= 60, "alias.yaml 应含 ≥ 60 条种子别名，实际 {}", table.len());
}

#[test]
fn test_alias_table_resolves_historical_name() {
    let table = AliasTable::load("config/alias.yaml").expect("加载失败");
    let id1 = table.resolve("毛泽东");
    let id2 = table.resolve("毛润之");
    assert!(id1.is_some() && id2.is_some(), "毛泽东 / 毛润之 应能解析");
    assert_eq!(id1, id2, "毛泽东 vs 毛润之 应解析为同一 entity_id");
}

#[test]
fn test_alias_table_resolves_honorific() {
    let table = AliasTable::load("config/alias.yaml").expect("加载失败");
    let id1 = table.resolve("孔子");
    let id2 = table.resolve("孔丘");
    let id3 = table.resolve("仲尼");
    assert_eq!(id1, id2, "孔子 vs 孔丘 应解析为同一 entity_id");
    assert_eq!(id2, id3, "孔丘 vs 仲尼 应解析为同一 entity_id");
}

#[test]
fn test_alias_table_resolves_abbreviation() {
    let table = AliasTable::load("config/alias.yaml").expect("加载失败");
    let id1 = table.resolve("北大");
    let id2 = table.resolve("北京大学");
    assert_eq!(id1, id2, "北大 vs 北京大学 应解析为同一 entity_id");
}

#[test]
fn test_alias_table_unmatched_falls_back_to_edit_distance() {
    let table = AliasTable::load("config/alias.yaml").expect("加载失败");
    // 未命中的别名应返回 None（由调用方回退到编辑距离）
    let result = table.resolve("完全不存在的名字XYZ");
    assert!(result.is_none(), "未命中别名表应返回 None");
}

#[test]
fn test_alias_table_audit_log_records_resolutions() {
    let mut table = AliasTable::load("config/alias.yaml").expect("加载失败");
    table.resolve("毛泽东");
    table.resolve("孔子");
    let log = table.audit_log();
    assert!(log.len() >= 2, "应记录至少 2 次解析");
    // 每条记录含 raw / resolved_id / timestamp
    assert!(log.iter().any(|r| r.raw == "毛泽东"), "应记录毛泽东的解析");
    assert!(log.iter().any(|r| r.raw == "孔子"), "应记录孔子的解析");
}
