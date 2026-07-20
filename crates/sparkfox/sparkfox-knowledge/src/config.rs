//! Sub-Step 10.7.1 — extract.yaml 实体类型配置加载
//!
//! ## 职责
//! - 定义 `ExtractConfig` / `EntityTypeConfig` 数据结构（与 `config/extract.yaml` 对应）
//! - 提供 `load_extract_config()` 从默认路径读取并解析
//! - 提供 `load_extract_config_from(path)` 从指定路径读取并解析
//!
//! ## 配置文件位置
//! 默认相对路径 `config/extract.yaml`（相对于当前工作目录，集成测试 cwd = crate 根）。

use sparkfox_core::{Error, Result};

/// extract.yaml 顶层结构
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExtractConfig {
    /// 实体类型列表
    pub entity_types: Vec<EntityTypeConfig>,
}

/// 单个实体类型配置（yaml 中的一项）
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EntityTypeConfig {
    /// 实体类型 ID（如 `default_person`）
    pub id: String,
    /// 实体类型枚举（如 `PERSON`；YAML 字段名为 `type`，Rust 中避让关键字重命名为 `type_`）
    #[serde(rename = "type")]
    pub type_: String,
    /// 中文显示名（如 `人名`）
    pub name: String,
    /// 十六进制颜色（如 `#FF6B6B`，参考 Arco Design 调色板）
    pub color: String,
    /// 图标名（@icon-park/react 图标名，与前端风格一致）
    pub icon: String,
    /// 权重（用于实体抽取排序，默认 1.0）
    pub weight: f64,
    /// 相似度阈值（用于实体归一化/去重，默认 0.8）
    pub similarity_threshold: f64,
    /// 简要描述（识别范围说明）
    pub description: String,
    /// 是否为默认实体类型（11 种内置均为 true）
    pub is_default: bool,
}

/// 从默认路径 `config/extract.yaml` 加载配置
///
/// 调用方通常为 SAG 提取管线；集成测试 cwd = crate 根，可直接使用相对路径。
pub fn load_extract_config() -> Result<ExtractConfig> {
    load_extract_config_from("config/extract.yaml")
}

/// 从指定路径加载 extract.yaml 配置
///
/// 步骤：读取文件 → serde_yaml 解析 → 返回 ExtractConfig
pub fn load_extract_config_from(path: &str) -> Result<ExtractConfig> {
    let yaml_text = std::fs::read_to_string(path)?;
    let config: ExtractConfig = serde_yaml::from_str(&yaml_text).map_err(|e| {
        Error::parse(
            format!("serde_yaml 解析 extract.yaml 失败: {e}"),
            "extract.yaml",
        )
    })?;
    Ok(config)
}
