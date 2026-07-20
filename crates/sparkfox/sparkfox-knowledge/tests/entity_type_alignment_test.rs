//! Sub-Step 10.7.1 — 11 种默认实体类型 + extract.yaml 对齐测试（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标
//! 1. `entity_type` 表预填 11 行默认数据（is_default=1）
//! 2. `config/extract.yaml` 与 `schema.rs::ENTITY_TYPES` 完全一致
//! 3. 11 类完整覆盖 {人名, 地名, 机构, 时间, 数字, 事件, 物品, 概念, 法律, 疾病, 其他}
//!
//! ## 测试策略
//! - 使用 `rusqlite::Connection::open_in_memory()` 建临时库
//! - 依次执行 `DDL_ENTITY_TYPE` + `INSERT_DEFAULT_ENTITY_TYPES`
//! - 使用 `std::fs::read_to_string` 读取 `config/extract.yaml`
//! - 使用 `serde_yaml::from_str` 解析为 `ExtractConfig`，同时调用 `load_extract_config()` 交叉验证
//! - 直接断言 `ENTITY_TYPES` 常量数组

#![forbid(unsafe_code)]

use std::collections::HashSet;

use rusqlite::Connection;

use sparkfox_knowledge::schema::{
    DDL_ENTITY_TYPE, ENTITY_TYPES, INSERT_DEFAULT_ENTITY_TYPES,
};
use sparkfox_knowledge::{load_extract_config, ExtractConfig};

/// 测试 1：entity_type 表预填 11 行默认数据
///
/// 步骤：open_in_memory → 执行 DDL_ENTITY_TYPE → 执行 INSERT_DEFAULT_ENTITY_TYPES
///       → SELECT COUNT(*) FROM entity_type WHERE is_default=1
/// 期望：COUNT == 11
#[test]
fn test_entity_type_table_has_11_defaults() {
    let conn = Connection::open_in_memory().expect("打开内存数据库失败");

    conn.execute_batch(DDL_ENTITY_TYPE)
        .expect("执行 DDL_ENTITY_TYPE 失败");

    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES)
        .expect("执行 INSERT_DEFAULT_ENTITY_TYPES 失败");

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entity_type WHERE is_default = 1",
            [],
            |row| row.get(0),
        )
        .expect("查询默认实体类型数量失败");

    assert_eq!(
        count, 11,
        "entity_type 表应预填 11 行默认数据，实际 = {}",
        count
    );

    // 同时验证 is_default=0 的行数为 0（仅默认数据）
    let non_default: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entity_type WHERE is_default = 0",
            [],
            |row| row.get(0),
        )
        .expect("查询非默认实体类型数量失败");

    assert_eq!(
        non_default, 0,
        "当前仅有默认数据，is_default=0 行数应为 0，实际 = {}",
        non_default
    );
}

/// 测试 2：extract.yaml 与 schema.rs 完全一致
///
/// 步骤：
///   - 读取 config/extract.yaml → serde_yaml 解析为 ExtractConfig（直接解析）
///   - 调用 load_extract_config() 交叉验证（函数封装）
///   - 收集 (type, name) 集合 → 与 ENTITY_TYPES 比对
/// 期望：两个集合完全相等
#[test]
fn test_extract_yaml_consistent_with_schema() {
    // 方式 1：直接读取并解析（验证 yaml 文件本身）
    let yaml_text =
        std::fs::read_to_string("config/extract.yaml").expect("读取 config/extract.yaml 失败");

    let config_direct: ExtractConfig =
        serde_yaml::from_str(&yaml_text).expect("解析 extract.yaml 为 ExtractConfig 失败");

    // 方式 2：通过 load_extract_config() 函数（验证函数实现）
    let config_loaded: ExtractConfig =
        load_extract_config().expect("调用 load_extract_config() 失败");

    // 两者应一致
    assert_eq!(
        config_direct.entity_types.len(),
        config_loaded.entity_types.len(),
        "直接解析与 load_extract_config() 返回的实体类型数量不一致"
    );

    // 收集 yaml 中的 (type, name) 集合
    let yaml_set: HashSet<(&str, &str)> = config_direct
        .entity_types
        .iter()
        .map(|t| (t.type_.as_str(), t.name.as_str()))
        .collect();

    // 收集 schema.rs::ENTITY_TYPES 中的 (type, name) 集合
    let schema_set: HashSet<(&str, &str)> = ENTITY_TYPES.iter().map(|t| (t.1, t.2)).collect();

    assert_eq!(
        yaml_set.len(),
        11,
        "extract.yaml 应声明 11 类实体，实际 = {}",
        yaml_set.len()
    );

    assert_eq!(
        yaml_set, schema_set,
        "extract.yaml 与 schema.rs::ENTITY_TYPES 不一致\nyaml:    {:?}\nschema:  {:?}",
        yaml_set, schema_set
    );

    // 同时验证 yaml 中所有 is_default 标志应 == true
    for t in &config_direct.entity_types {
        assert!(
            t.is_default,
            "extract.yaml 中实体类型 {} ({}) 的 is_default 应为 true",
            t.type_,
            t.name
        );
    }
}

/// 测试 3：11 类完整覆盖（中文名匹配）
///
/// 期望集合 = {人名, 地名, 机构, 时间, 数字, 事件, 物品, 概念, 法律, 疾病, 其他}
#[test]
fn test_11_types_cover_expected_set() {
    // 期望的 11 类中文名
    let expected: HashSet<&str> = [
        "人名",
        "地名",
        "机构",
        "时间",
        "数字",
        "事件",
        "物品",
        "概念",
        "法律",
        "疾病",
        "其他",
    ]
    .into_iter()
    .collect();

    // 实际 ENTITY_TYPES 中的中文名集合
    let actual: HashSet<&str> = ENTITY_TYPES.iter().map(|t| t.2).collect();

    assert_eq!(
        ENTITY_TYPES.len(),
        11,
        "ENTITY_TYPES 应有 11 项，实际 = {}",
        ENTITY_TYPES.len()
    );

    assert_eq!(
        actual, expected,
        "ENTITY_TYPES 未完整覆盖期望的 11 类\nactual:   {:?}\nexpected: {:?}",
        actual, expected
    );

    // 同时验证 type 字段（英文枚举）的覆盖
    let expected_types: HashSet<&str> = [
        "PERSON",
        "LOCATION",
        "ORGANIZATION",
        "TIME",
        "NUMBER",
        "EVENT",
        "OBJECT",
        "CONCEPT",
        "LAW",
        "DISEASE",
        "OTHER",
    ]
    .into_iter()
    .collect();

    let actual_types: HashSet<&str> = ENTITY_TYPES.iter().map(|t| t.1).collect();

    assert_eq!(
        actual_types, expected_types,
        "ENTITY_TYPES type 字段未完整覆盖期望的 11 类英文枚举\nactual:   {:?}\nexpected: {:?}",
        actual_types, expected_types
    );
}
