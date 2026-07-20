#![forbid(unsafe_code)]
//! 关系抽取 — LLM function calling 占位
//!
//! v1.0.0：占位实现，返回空 Vec（避免对 sparkfox-llm 形成循环依赖）
//! v1.1.0+：集成 sparkfox-llm `structured_complete`
//!
//! 参考：spec 1.0 第 2959-2987 行（Task 8.14）。

use serde::{Deserialize, Serialize};
use sparkfox_core::Result;

use crate::extractor::Entity;

/// 抽取出的关系（三元组：source → target，附关系类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    /// 头实体名称
    pub source: String,
    /// 尾实体名称
    pub target: String,
    /// 关系类型（如 works_for / located_in / participates_in 等）
    pub relation_type: String,
    /// 置信度 [0.0, 1.0]
    pub confidence: f32,
}

impl Relation {
    /// 创建一个置信度默认为 1.0 的关系
    pub fn new(source: String, target: String, relation_type: String) -> Self {
        Self {
            source,
            target,
            relation_type,
            confidence: 1.0,
        }
    }
}

/// 关系抽取器
///
/// v1.0.0 占位实现；v1.1.0+ 内部持有 LLM 客户端句柄。
pub struct RelationExtractor;

impl RelationExtractor {
    pub fn new() -> Self {
        Self
    }

    /// 从文本 + 已抽取的实体中识别关系
    ///
    /// v1.0.0 占位：始终返回空 Vec
    /// v1.1.0+ 调用 sparkfox-llm `structured_complete`
    pub async fn extract(
        &self,
        _text: &str,
        _entities: &[Entity],
    ) -> Result<Vec<Relation>> {
        Ok(vec![])
    }
}

impl Default for RelationExtractor {
    fn default() -> Self {
        Self::new()
    }
}
