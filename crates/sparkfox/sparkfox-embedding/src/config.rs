//! 嵌入模型配置 — 支持运行时切换 bge-small-zh / bge-large-zh
//!
//! 【模型选择策略】
//! - **bge-small-zh-v1.5**（默认）：120MB，512 维，桌面端友好，PoC-3 已验证
//! - **bge-large-zh-v1.5**（可选）：1.2GB，1024 维，更高精度，需用户显式启用
//!
//! 切换方式：通过 `EmbeddingConfig::model` 字段序列化到设置文件，
//! 运行时 `BgeEmbedder::load(config.model.to_variant())` 加载对应模型。

use serde::{Deserialize, Serialize};

use crate::downloader::ModelVariant;

/// 嵌入模型枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingModel {
    /// bge-small-zh-v1.5（120MB，512 维，默认）
    BgeSmallZh,
    /// bge-large-zh-v1.5（1.2GB，1024 维，可选）
    BgeLargeZh,
}

impl Default for EmbeddingModel {
    fn default() -> Self {
        Self::BgeSmallZh
    }
}

impl EmbeddingModel {
    /// 转换为下载器变体
    pub fn to_variant(&self) -> ModelVariant {
        match self {
            Self::BgeSmallZh => ModelVariant::BgeSmallZh,
            Self::BgeLargeZh => ModelVariant::BgeLargeZh,
        }
    }

    /// 嵌入维度
    pub fn dim(&self) -> usize {
        match self {
            Self::BgeSmallZh => 512,
            Self::BgeLargeZh => 1024,
        }
    }

    /// 模型大小（MB）
    pub fn size_mb(&self) -> usize {
        match self {
            Self::BgeSmallZh => 120,
            Self::BgeLargeZh => 1200,
        }
    }

    /// 模型 repo_id（如 "BAAI/bge-small-zh-v1.5"）
    pub fn repo_id(&self) -> &'static str {
        self.to_variant().repo_id()
    }

    /// 序列化友好的模型名（用于缓存 key、日志）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BgeSmallZh => "bge-small-zh",
            Self::BgeLargeZh => "bge-large-zh",
        }
    }
}

impl std::fmt::Display for EmbeddingModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.repo_id())
    }
}

/// 重排模型枚举（Task 5.3）
///
/// 与 [`EmbeddingModel`] 对称：解耦配置层与 downloader 层，
/// 便于序列化到设置文件。`ModelVariant`（在 `downloader.rs` 中）
/// 未实现 `Serialize`/`Deserialize`，故此处定义独立的配置枚举。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RerankerModel {
    /// bge-reranker-v2-m3（560MB，CrossEncoder，默认）
    BgeRerankerV2M3,
}

impl Default for RerankerModel {
    fn default() -> Self {
        Self::BgeRerankerV2M3
    }
}

impl RerankerModel {
    /// 转换为下载器变体
    pub fn to_variant(&self) -> ModelVariant {
        match self {
            Self::BgeRerankerV2M3 => ModelVariant::BgeRerankerV2M3,
        }
    }

    /// 模型大小（MB）
    pub fn size_mb(&self) -> usize {
        self.to_variant().size_mb()
    }

    /// 模型 repo_id（如 "BAAI/bge-reranker-v2-m3"）
    pub fn repo_id(&self) -> &'static str {
        self.to_variant().repo_id()
    }

    /// 序列化友好的模型名（用于缓存 key、日志）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BgeRerankerV2M3 => "bge-reranker-v2-m3",
        }
    }
}

impl std::fmt::Display for RerankerModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.repo_id())
    }
}

/// 嵌入模块运行时配置
///
/// 序列化到 `~/.sparkfox/embedding.toml` 或通过 Tauri 设置面板修改。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// 嵌入模型（默认 bge-small-zh）
    pub model: EmbeddingModel,

    /// Rerank 开关（Task 5.3，v0.3.2+ 启用 bge-reranker-v2-m3）
    ///
    /// 关闭时：仅向量 + 关键词 RRF 融合
    /// 开启时：在 RRF 之上加 bge-reranker-v2-m3 重排
    ///
    /// 默认 `true`（spec 1.0 Task 5.3 要求）。
    /// 注意：开启后首次使用会触发 560MB 模型下载。
    pub reranker_enabled: bool,

    /// Rerank 前从混合检索召回的文档数量（Task 5.3）
    ///
    /// 典型流程：hybrid_search 召回 `rerank_top_k` 条 → reranker 重排 → 取前 K 条。
    /// 默认 `10`。值越大召回越全但 rerank 越慢。
    #[serde(default = "default_rerank_top_k")]
    pub rerank_top_k: usize,

    /// 重排模型变体（Task 5.3）
    ///
    /// 默认 `BgeRerankerV2M3`。未来可扩展其他 reranker。
    #[serde(default)]
    pub rerank_model: RerankerModel,

    /// 查询嵌入缓存开关（默认开启）
    ///
    /// 用户决策 B：文档嵌入每次重建，仅缓存查询嵌入。
    /// 关闭时：每次查询都重新计算嵌入（用于调试或模型对比）
    pub query_cache_enabled: bool,
}

/// `EmbeddingConfig::rerank_top_k` 的 serde 默认值函数
fn default_rerank_top_k() -> usize {
    10
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: EmbeddingModel::default(),
            reranker_enabled: true,
            rerank_top_k: default_rerank_top_k(),
            rerank_model: RerankerModel::default(),
            query_cache_enabled: true,
        }
    }
}

impl EmbeddingConfig {
    /// 创建默认配置（bge-small-zh，开启 rerank，开启查询缓存）
    pub fn new() -> Self {
        Self::default()
    }

    /// 切换到 bge-large-zh
    pub fn with_large_model(mut self) -> Self {
        self.model = EmbeddingModel::BgeLargeZh;
        self
    }

    /// 启用 reranker
    pub fn with_reranker(mut self) -> Self {
        self.reranker_enabled = true;
        self
    }

    /// 禁用 reranker（Task 5.3）
    pub fn without_reranker(mut self) -> Self {
        self.reranker_enabled = false;
        self
    }

    /// 设置 rerank 前召回数量（Task 5.3）
    pub fn with_rerank_top_k(mut self, top_k: usize) -> Self {
        self.rerank_top_k = top_k;
        self
    }

    /// 设置重排模型变体（Task 5.3）
    pub fn with_rerank_model(mut self, model: RerankerModel) -> Self {
        self.rerank_model = model;
        self
    }

    /// 禁用查询缓存
    pub fn without_query_cache(mut self) -> Self {
        self.query_cache_enabled = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_small_model() {
        let cfg = EmbeddingConfig::default();
        assert_eq!(cfg.model, EmbeddingModel::BgeSmallZh);
        // Task 5.3：reranker 默认开启
        assert!(cfg.reranker_enabled);
        assert_eq!(cfg.rerank_top_k, 10);
        assert_eq!(cfg.rerank_model, RerankerModel::BgeRerankerV2M3);
        assert!(cfg.query_cache_enabled);
    }

    #[test]
    fn model_dim_matches_spec() {
        assert_eq!(EmbeddingModel::BgeSmallZh.dim(), 512);
        assert_eq!(EmbeddingModel::BgeLargeZh.dim(), 1024);
    }

    #[test]
    fn model_size_matches_spec() {
        assert_eq!(EmbeddingModel::BgeSmallZh.size_mb(), 120);
        assert_eq!(EmbeddingModel::BgeLargeZh.size_mb(), 1200);
    }

    #[test]
    fn model_to_variant_round_trip() {
        assert_eq!(
            EmbeddingModel::BgeSmallZh.to_variant(),
            ModelVariant::BgeSmallZh
        );
        assert_eq!(
            EmbeddingModel::BgeLargeZh.to_variant(),
            ModelVariant::BgeLargeZh
        );
    }

    #[test]
    fn model_repo_id_stable() {
        assert_eq!(
            EmbeddingModel::BgeSmallZh.repo_id(),
            "BAAI/bge-small-zh-v1.5"
        );
        assert_eq!(
            EmbeddingModel::BgeLargeZh.repo_id(),
            "BAAI/bge-large-zh-v1.5"
        );
    }

    #[test]
    fn config_builder_chain() {
        let cfg = EmbeddingConfig::new()
            .with_large_model()
            .with_reranker()
            .without_query_cache();
        assert_eq!(cfg.model, EmbeddingModel::BgeLargeZh);
        assert!(cfg.reranker_enabled);
        assert!(!cfg.query_cache_enabled);
    }

    #[test]
    fn config_serializes_to_json() {
        let cfg = EmbeddingConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize");
        assert!(json.contains("bge_small_zh"), "json: {json}");
        // Task 5.3：reranker 默认开启
        assert!(json.contains("\"reranker_enabled\":true"), "json: {json}");
        assert!(json.contains("\"rerank_top_k\":10"), "json: {json}");
        assert!(
            json.contains("\"rerank_model\":\"bge_reranker_v2_m3\""),
            "json: {json}"
        );
        assert!(
            json.contains("\"query_cache_enabled\":true"),
            "json: {json}"
        );
    }

    #[test]
    fn config_deserializes_from_json() {
        // 旧格式 JSON（无 rerank_top_k / rerank_model）应通过 serde 默认值兼容
        let json = r#"{"model":"bge_large_zh","reranker_enabled":true,"query_cache_enabled":false}"#;
        let cfg: EmbeddingConfig = serde_json::from_str(json).expect("deserialize");
        assert_eq!(cfg.model, EmbeddingModel::BgeLargeZh);
        assert!(cfg.reranker_enabled);
        assert!(!cfg.query_cache_enabled);
        // 缺失字段回退到默认值
        assert_eq!(cfg.rerank_top_k, 10);
        assert_eq!(cfg.rerank_model, RerankerModel::BgeRerankerV2M3);
    }

    #[test]
    fn config_deserializes_full_json() {
        // 完整格式 JSON（含 Task 5.3 新字段）
        let json = r#"{"model":"bge_large_zh","reranker_enabled":false,"rerank_top_k":20,"rerank_model":"bge_reranker_v2_m3","query_cache_enabled":false}"#;
        let cfg: EmbeddingConfig = serde_json::from_str(json).expect("deserialize");
        assert_eq!(cfg.model, EmbeddingModel::BgeLargeZh);
        assert!(!cfg.reranker_enabled);
        assert_eq!(cfg.rerank_top_k, 20);
        assert_eq!(cfg.rerank_model, RerankerModel::BgeRerankerV2M3);
        assert!(!cfg.query_cache_enabled);
    }

    #[test]
    fn model_display() {
        assert_eq!(
            format!("{}", EmbeddingModel::BgeSmallZh),
            "BAAI/bge-small-zh-v1.5"
        );
    }

    // ===== Task 5.3: RerankerModel + 新增配置项测试 =====

    #[test]
    fn reranker_model_default_is_v2_m3() {
        assert_eq!(RerankerModel::default(), RerankerModel::BgeRerankerV2M3);
    }

    #[test]
    fn reranker_model_to_variant_round_trip() {
        assert_eq!(
            RerankerModel::BgeRerankerV2M3.to_variant(),
            ModelVariant::BgeRerankerV2M3
        );
    }

    #[test]
    fn reranker_model_repo_id_stable() {
        assert_eq!(
            RerankerModel::BgeRerankerV2M3.repo_id(),
            "BAAI/bge-reranker-v2-m3"
        );
    }

    #[test]
    fn reranker_model_size_matches_spec() {
        assert_eq!(RerankerModel::BgeRerankerV2M3.size_mb(), 560);
    }

    #[test]
    fn reranker_model_as_str_stable() {
        assert_eq!(RerankerModel::BgeRerankerV2M3.as_str(), "bge-reranker-v2-m3");
    }

    #[test]
    fn reranker_model_display() {
        assert_eq!(
            format!("{}", RerankerModel::BgeRerankerV2M3),
            "BAAI/bge-reranker-v2-m3"
        );
    }

    #[test]
    fn reranker_model_serializes_to_snake_case() {
        let json = serde_json::to_string(&RerankerModel::BgeRerankerV2M3).expect("serialize");
        assert_eq!(json, "\"bge_reranker_v2_m3\"");
    }

    #[test]
    fn reranker_model_deserializes_from_snake_case() {
        let m: RerankerModel =
            serde_json::from_str("\"bge_reranker_v2_m3\"").expect("deserialize");
        assert_eq!(m, RerankerModel::BgeRerankerV2M3);
    }

    #[test]
    fn config_without_reranker_builder() {
        let cfg = EmbeddingConfig::default().without_reranker();
        assert!(!cfg.reranker_enabled);
    }

    #[test]
    fn config_with_rerank_top_k_builder() {
        let cfg = EmbeddingConfig::default().with_rerank_top_k(50);
        assert_eq!(cfg.rerank_top_k, 50);
    }

    #[test]
    fn config_with_rerank_model_builder() {
        let cfg = EmbeddingConfig::default().with_rerank_model(RerankerModel::BgeRerankerV2M3);
        assert_eq!(cfg.rerank_model, RerankerModel::BgeRerankerV2M3);
    }

    #[test]
    fn config_full_builder_chain() {
        let cfg = EmbeddingConfig::new()
            .with_large_model()
            .with_reranker()
            .with_rerank_top_k(20)
            .with_rerank_model(RerankerModel::BgeRerankerV2M3)
            .without_query_cache();
        assert_eq!(cfg.model, EmbeddingModel::BgeLargeZh);
        assert!(cfg.reranker_enabled);
        assert_eq!(cfg.rerank_top_k, 20);
        assert_eq!(cfg.rerank_model, RerankerModel::BgeRerankerV2M3);
        assert!(!cfg.query_cache_enabled);
    }
}
