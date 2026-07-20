//! SparkFox Embedding — 嵌入与重排（candle-transformers）
//!
//! 基于 candle-transformers 0.8（Hugging Face 官方 Rust 栈）。
//! 嵌入模型：bge-small-zh-v1.5（默认，120MB）/ bge-large-zh-v1.5（可选，1.2GB）
//! 重排模型：bge-reranker-v2-m3（560MB，v0.3.2+）
//!
//! 本 crate 是 PoC-3（bge Rust 推理性）的载体，验收门槛：
//! - 单条嵌入 < 50ms
//! - 1000 条批量 < 30s
//! - 与 Python sentence-transformers cosine > 0.99（严格门槛，用户决策 B）
//!
//! 【unsafe 说明】candle-nn 的 `VarBuilder::from_mmaped_safetensors` 是 unsafe 的
//! （mmap + FFI），因此本 crate 用 `#![deny(unsafe_code)]` 而非 `forbid`，
//! 仅在 mmap 加载处局部 `#[allow(unsafe_code)]`。

#![deny(unsafe_code)]

pub mod cache;
pub mod clip;
pub mod config;
pub mod downloader;
pub mod embedder;
pub mod reranker;

pub use cache::QueryCache;
pub use clip::ClipEmbedder;
pub use config::{EmbeddingConfig, EmbeddingModel, RerankerModel};
pub use downloader::{find_local_model_dir, ModelVariant};
pub use embedder::BgeEmbedder;
pub use reranker::{BgeReranker, RerankResult};

use sparkfox_core::Result;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 嵌入器 trait — 支持多模型切换与降级
///
/// 实现方：
/// - [`BgeEmbedder`]：bge-small-zh / bge-large-zh（candle-transformers 推理）
///
/// 上层（sparkfox-knowledge）通过此 trait 解耦具体嵌入实现，
/// 便于未来切换到 ONNX/Python sidecar 等后端。
pub trait Embedder: Send + Sync {
    /// 单条文本嵌入（返回 L2 归一化后的向量）
    fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// 批量嵌入（逐条实现可被 batch padding 优化覆盖）
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// 嵌入维度
    fn dim(&self) -> usize;

    /// 模型名（如 "BAAI/bge-small-zh-v1.5"）
    fn model_name(&self) -> &str;
}

pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-embedding v{} initialized", VERSION);
}
