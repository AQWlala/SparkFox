//! BgeEmbedder — bge 嵌入推理（candle-transformers 0.8）
//!
//! 流程：tokenize → BertModel.forward → mean pooling → L2 normalize
//!
//! 模型变体：
//! - bge-small-zh-v1.5（512 维）
//! - bge-large-zh-v1.5（1024 维）

use std::path::PathBuf;

use candle_core::{DType, Device, Tensor};
use candle_transformers::models::bert::{BertModel, Config};
use tokenizers::Tokenizer;

use sparkfox_core::{Error, Result};

use crate::downloader::{download_model, verify_model_files, ModelVariant};

/// bge 嵌入推理器
pub struct BgeEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    dim: usize,
    model_name: String,
}

impl BgeEmbedder {
    /// 加载模型
    ///
    /// 若模型未下载，会自动通过 hf-hub 下载到 `cache_dir()`。
    ///
    /// 【S-02 P0】加载前强制 SHA256 校验，防供应链攻击：
    /// - 常量表 `MODEL_SHA256` 中已配置的文件：强制校验，失败返回 `Error::crypto`
    /// - 常量表中未配置的文件：记录 warn 日志，不阻塞（开发阶段）
    /// - 生产部署前必须填充 `MODEL_SHA256` 常量表
    pub fn load(variant: ModelVariant) -> Result<Self> {
        let model_dir = download_model(&variant)?;
        // 【S-02 P0】强制 SHA256 校验
        verify_model_files(&model_dir, &variant)?;
        let device = Device::Cpu; // 桌面端默认 CPU，GPU 留 v1.1+
        Self::load_from_dir(&model_dir, &device, variant)
    }

    /// 尝试加载模型，失败时返回 [`LoadError`]（上层可降级为纯关键词检索）
    ///
    /// 与 [`load`](Self::load) 的区别：错误被分类为 `ModelNotFound` /
    /// `InsufficientMemory` / `Other`，便于上层（sparkfox-knowledge）
    /// 决定降级策略：
    /// - `ModelNotFound` → 提示用户下载模型，临时降级为关键词检索
    /// - `InsufficientMemory` → 提示关闭其他应用，临时降级为关键词检索
    /// - `Other` → 抛出致命错误（不应继续）
    pub fn try_load(variant: ModelVariant) -> std::result::Result<Self, LoadError> {
        Self::load(variant).map_err(LoadError::from)
    }
}

/// 嵌入模型加载错误（用于上层降级决策）
///
/// 由 [`BgeEmbedder::try_load`] 返回，由 `sparkfox_core::Error` 分类而来。
#[derive(Debug)]
pub enum LoadError {
    /// 模型文件未找到（未下载 / 路径错误 / SHA256 校验失败）
    ModelNotFound(String),
    /// 内存不足（mmap 加载失败 / OOM）
    InsufficientMemory(String),
    /// 其他错误（不应触发降级）
    Other(String),
}

impl LoadError {
    /// 是否可降级为纯关键词检索
    ///
    /// `ModelNotFound` 和 `InsufficientMemory` 可降级；
    /// `Other` 通常表示代码 bug，不应继续。
    pub fn is_degradable(&self) -> bool {
        matches!(self, Self::ModelNotFound(_) | Self::InsufficientMemory(_))
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModelNotFound(msg) => write!(f, "[load:model_not_found] {msg}"),
            Self::InsufficientMemory(msg) => write!(f, "[load:insufficient_memory] {msg}"),
            Self::Other(msg) => write!(f, "[load:other] {msg}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<sparkfox_core::Error> for LoadError {
    fn from(e: sparkfox_core::Error) -> Self {
        use sparkfox_core::Error as E;
        let msg = e.to_string();
        match e {
            // 显式 NotFound → ModelNotFound
            E::NotFound { .. } => Self::ModelNotFound(msg),
            // IO 错误细查 kind（NotFound / PermissionDenied → ModelNotFound）
            E::Io(io_err) => {
                use std::io::ErrorKind;
                match io_err.kind() {
                    ErrorKind::NotFound | ErrorKind::PermissionDenied => {
                        Self::ModelNotFound(msg)
                    }
                    _ => Self::Other(msg),
                }
            }
            // 其余 variant 通过消息文本判断内存类
            _ => {
                let lower = msg.to_lowercase();
                if lower.contains("memory")
                    || lower.contains("oom")
                    || lower.contains("out of memory")
                    || lower.contains("mmap")
                {
                    Self::InsufficientMemory(msg)
                } else if lower.contains("not found")
                    || lower.contains("未找到")
                    || lower.contains("no such file")
                    || lower.contains("下载")
                {
                    Self::ModelNotFound(msg)
                } else {
                    Self::Other(msg)
                }
            }
        }
    }
}

impl BgeEmbedder {
    /// 从指定目录加载模型（用于测试或自定义路径）
    pub fn load_from_dir(model_dir: &PathBuf, device: &Device, variant: ModelVariant) -> Result<Self> {
        let config_path = model_dir.join("config.json");
        let tokenizer_path = model_dir.join("tokenizer.json");
        let weights_path = model_dir.join("model.safetensors");

        // 解析 config.json
        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| Error::internal(format!("读取 config.json 失败: {e}")))?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| Error::internal(format!("解析 config.json 失败: {e}")))?;

        // 加载 tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| Error::internal(format!("tokenizer 加载失败: {e}")))?;

        // 加载权重（mmap，unsafe）
        let vb = {
            // SAFETY: from_mmaped_safetensors 使用 mmap，文件路径由我们控制
            // （已通过 hf-hub 下载或用户指定），不会在运行时被修改。
            #[allow(unsafe_code)]
            unsafe {
                candle_nn::VarBuilder::from_mmaped_safetensors(
                    &[weights_path],
                    DType::F32,
                    device,
                )
                .map_err(|e| Error::internal(format!("weights 加载失败: {e}")))?
            }
        };

        let model = BertModel::load(vb, &config)
            .map_err(|e| Error::internal(format!("BertModel 加载失败: {e}")))?;

        let dim = variant.dim();
        let model_name = variant.repo_id().to_string();

        Ok(Self {
            model,
            tokenizer,
            device: device.clone(),
            dim,
            model_name,
        })
    }

    /// 单条文本嵌入
    ///
    /// 返回归一化后的嵌入向量（L2 norm = 1）。
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| Error::internal(format!("tokenize 失败: {e}")))?;

        let ids = encoding.get_ids();
        let input_ids = Tensor::from_slice(ids as &[u32], (1, ids.len()), &self.device)
            .map_err(|e| Error::internal(format!("input_ids 构造失败: {e}")))?;

        let token_type_ids = input_ids
            .zeros_like()
            .map_err(|e| Error::internal(format!("token_type_ids 构造失败: {e}")))?;

        let attention_mask = input_ids
            .ones_like()
            .map_err(|e| Error::internal(format!("attention_mask 构造失败: {e}")))?;

        // forward 返回 (batch, seq_len, hidden_size)
        let embeddings = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask))
            .map_err(|e| Error::internal(format!("forward 失败: {e}")))?;

        // mean pooling: (batch, hidden_size)
        let pooled = mean_pool_last_hidden(&embeddings, &attention_mask)?;

        // L2 normalize
        let normalized = l2_normalize(&pooled)?;

        // squeeze batch dim → (hidden_size,)
        let squeezed = normalized
            .squeeze(0)
            .map_err(|e| Error::internal(format!("squeeze 失败: {e}")))?;

        let vec = squeezed
            .to_vec1::<f32>()
            .map_err(|e| Error::internal(format!("to_vec1 失败: {e}")))?;

        Ok(vec)
    }

    /// 批量嵌入（逐条调用 embed）
    ///
    /// 生产级实现可用 batch padding + 一次性 forward，
    /// 这里为简化实现，逐条处理。
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// 嵌入维度
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// 模型名（如 "BAAI/bge-small-zh-v1.5"）
    pub fn model_name(&self) -> &str {
        &self.model_name
    }
}

impl crate::Embedder for BgeEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Rust 方法解析：inherent 方法优先于 trait 方法，此处调用 BgeEmbedder::embed
        BgeEmbedder::embed(self, text)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        BgeEmbedder::embed_batch(self, texts)
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// mean pooling on last hidden state
///
/// 输入：
/// - `hidden`: (batch, seq_len, hidden_size)
/// - `attention_mask`: (batch, seq_len)
///
/// 输出：(batch, hidden_size)
fn mean_pool_last_hidden(hidden: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
    // mask: (batch, seq_len, 1) — 广播到 hidden_size
    let mask = attention_mask
        .to_dtype(DType::F32)
        .map_err(|e| Error::internal(format!("mask dtype 转换失败: {e}")))?
        .unsqueeze(2)
        .map_err(|e| Error::internal(format!("mask unsqueeze 失败: {e}")))?;

    // masked_hidden = hidden * mask: (batch, seq_len, hidden_size)
    let masked_hidden = hidden
        .broadcast_mul(&mask)
        .map_err(|e| Error::internal(format!("masked_hidden 计算失败: {e}")))?;

    // sum along seq_len: (batch, hidden_size)
    let sum = masked_hidden
        .sum(1)
        .map_err(|e| Error::internal(format!("sum 失败: {e}")))?;

    // count: (batch,) → (batch, 1) 用于广播
    let count = attention_mask
        .to_dtype(DType::F32)
        .map_err(|e| Error::internal(format!("count dtype 转换失败: {e}")))?
        .sum(1)
        .map_err(|e| Error::internal(format!("count sum 失败: {e}")))?
        .unsqueeze(1)
        .map_err(|e| Error::internal(format!("count unsqueeze 失败: {e}")))?;

    // pooled = sum / count: (batch, hidden_size)
    let pooled = sum
        .broadcast_div(&count)
        .map_err(|e| Error::internal(format!("pooled 计算失败: {e}")))?;

    Ok(pooled)
}

/// L2 归一化（沿 hidden_size 维度）
///
/// 输入：(batch, hidden_size)
/// 输出：(batch, hidden_size)，每行 L2 norm = 1
fn l2_normalize(t: &Tensor) -> Result<Tensor> {
    let norm = t
        .sqr()
        .map_err(|e| Error::internal(format!("sqr 失败: {e}")))?
        .sum(1)
        .map_err(|e| Error::internal(format!("norm sum 失败: {e}")))?
        .sqrt()
        .map_err(|e| Error::internal(format!("sqrt 失败: {e}")))?;

    // broadcast norm: (batch, 1)
    let norm = norm
        .unsqueeze(1)
        .map_err(|e| Error::internal(format!("norm unsqueeze 失败: {e}")))?;

    // normalized = t / norm（广播）
    let normalized = t
        .broadcast_div(&norm)
        .map_err(|e| Error::internal(format!("normalize 计算失败: {e}")))?;

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedder_dim_matches_variant() {
        // 仅测试 dim 字段，不实际加载模型
        assert_eq!(ModelVariant::BgeSmallZh.dim(), 512);
        assert_eq!(ModelVariant::BgeLargeZh.dim(), 1024);
    }

    #[test]
    fn l2_normalize_produces_unit_vector() {
        // 手动构造一个 1x3 张量，验证归一化后 L2 norm ≈ 1
        let device = Device::Cpu;
        let input = Tensor::from_slice(&[3.0f32, 4.0, 0.0], (1, 3), &device).unwrap();
        let normalized = l2_normalize(&input).unwrap();
        let vec = normalized.squeeze(0).unwrap().to_vec1::<f32>().unwrap();
        // 3,4,0 → 0.6, 0.8, 0
        assert!((vec[0] - 0.6).abs() < 1e-5, "vec[0] = {}", vec[0]);
        assert!((vec[1] - 0.8).abs() < 1e-5, "vec[1] = {}", vec[1]);
        assert!(vec[2].abs() < 1e-5, "vec[2] = {}", vec[2]);
    }

    #[test]
    fn mean_pool_basic() {
        // 构造 1x3x2 hidden，1x3 mask
        // hidden = [[1,1],[2,2],[3,3]]，mask = [1,1,0]
        // 期望 pooled = (1+2+0)/2 = 1.5, (1+2+0)/2 = 1.5
        let device = Device::Cpu;
        let hidden = Tensor::from_slice(&[1.0f32, 1.0, 2.0, 2.0, 3.0, 3.0], (1, 3, 2), &device).unwrap();
        let mask = Tensor::from_slice(&[1u32, 1, 0], (1, 3), &device).unwrap();
        let pooled = mean_pool_last_hidden(&hidden, &mask).unwrap();
        let vec = pooled.to_vec2::<f32>().unwrap();
        assert!((vec[0][0] - 1.5).abs() < 1e-5, "pooled[0][0] = {}", vec[0][0]);
        assert!((vec[0][1] - 1.5).abs() < 1e-5, "pooled[0][1] = {}", vec[0][1]);
    }

    #[test]
    fn embedder_trait_object_compiles() {
        // 验证 BgeEmbedder 可作为 Box<dyn Embedder> 使用（不实际加载模型）
        fn accept_embedder(_e: Box<dyn crate::Embedder>) {}

        // 仅验证类型签名兼容，不调用 load（避免测试依赖模型文件）
        fn type_check(_e: &BgeEmbedder) -> bool {
            true
        }

        // 编译期验证：BgeEmbedder: Embedder
        fn assert_impl<E: crate::Embedder>(_: &E) {}
        // 用泛型占位验证 trait 约束（不构造实例）
        // 真实实例化由 PoC-3 测试覆盖
        let _f = type_check;
        let _g = accept_embedder;
        let _h = assert_impl::<BgeEmbedder>;
        assert!(true);
    }
}
