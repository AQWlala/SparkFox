#![forbid(unsafe_code)]
//! 实体抽取 — LLM function calling 占位
//!
//! v1.0.0：占位实现，返回空 Vec（避免对 sparkfox-llm 形成循环依赖）
//! v1.1.0+：集成 sparkfox-llm `structured_complete` 进行 NER
//!
//! 参考：spec 1.0 第 2924-2954 行（Task 8.13）。

use serde::{Deserialize, Serialize};
use sparkfox_core::Result;

/// 抽取出的实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// 实体名称（规范化文本）
    pub name: String,
    /// 实体类型（如 person / organization / location / event 等）
    pub entity_type: String,
    /// 起始字符偏移（可选，用于溯源）
    pub start_offset: Option<usize>,
    /// 结束字符偏移（可选，用于溯源）
    pub end_offset: Option<usize>,
    /// 置信度 [0.0, 1.0]
    pub confidence: f32,
}

impl Entity {
    /// 创建一个置信度默认为 1.0 的实体
    pub fn new(name: String, entity_type: String) -> Self {
        Self {
            name,
            entity_type,
            start_offset: None,
            end_offset: None,
            confidence: 1.0,
        }
    }
}

/// 实体抽取器
///
/// v1.0.0 占位实现；v1.1.0+ 内部持有 LLM 客户端句柄。
pub struct EntityExtractor;

impl EntityExtractor {
    pub fn new() -> Self {
        Self
    }

    /// 从文本中抽取实体
    ///
    /// v1.0.0 占位：始终返回空 Vec
    /// v1.1.0+ 调用 sparkfox-llm `structured_complete`
    pub async fn extract(&self, _text: &str) -> Result<Vec<Entity>> {
        Ok(vec![])
    }
}

impl Default for EntityExtractor {
    fn default() -> Self {
        Self::new()
    }
}
