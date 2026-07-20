//! BgeReranker — bge-reranker-v2-m3 重排（CrossEncoder 架构，560MB，懒加载）
//!
//! 模型：BAAI/bge-reranker-v2-m3（560MB）
//! 架构：CrossEncoder（BERT base + 二分类头 classifier）
//! 推理：[CLS] query [SEP] document [SEP] → BERT forward → 取 [CLS] hidden
//!       → Linear classifier → logit → sigmoid → 相关性分数 [0,1]
//!
//! 与 BgeEmbedder（Bi-Encoder）的区别：
//! - Embedder：query 和 doc 分别编码，cosine 相似度，速度快精度低
//! - Reranker：query 和 doc 拼接后联合编码，输出标量分数，速度慢精度高
//! - 典型流程：Embedder 召回 top-50 → Reranker 重排 → top-10
//!
//! 【unsafe 说明】weights 加载使用 `VarBuilder::from_mmaped_safetensors`（mmap + FFI），
//! 与 `embedder.rs` 一致，在 unsafe 块上局部 `#[allow(unsafe_code)]`，
//! 由 crate 级 `#![deny(unsafe_code)]`（lib.rs）统一管控。
//!
//! 【架构说明】bge-reranker-v2-m3 在 HF 上是 XLM-RoBERTa 架构，
//! 此处按 spec 1.0 Task 5.1 要求使用 `BertModel` 加载。
//! 生产部署若遇权重 key 不匹配（roberta.* vs bert.*），需切换到
//! `candle_transformers::models::xlm_roberta` 或自定义权重映射。

use std::path::PathBuf;

use candle_core::{DType, Device, Tensor};
use candle_nn::{linear, Linear, Module};
use candle_transformers::models::bert::{BertModel, Config};
use tokenizers::Tokenizer;

use sparkfox_core::{Error, Result};

use crate::downloader::{download_model, verify_model_files, ModelVariant};

// ============================================================================
// XLM-RoBERTa 架构（v1.1.0 新增，修复 v1.0.0 架构偏差）
// ============================================================================
//
// 【架构差异说明】
// bge-reranker-v2-m3 在 HuggingFace 上是 XLM-RoBERTa 架构，权重 key 前缀为
// `roberta.*`（而非 BERT 的 `bert.*`）。XLM-RoBERTa 与 BERT 在结构上几乎一致，
// 主要差异：
// 1. 权重 key 前缀：`roberta.*` vs `bert.*`
// 2. position_ids 起始偏移：XLM-R 从 `pad_token_id + 1`（=2）起算，
//    BERT 从 0 起算。XLM-R 保留 position 0/1 给 <pad>/<s>，position 2+ 才是真实位置
// 3. token_type_ids：XLM-R 不使用 segment embedding（但权重存在，传入 zeros 即可）
// 4. 特殊 token：XLM-R 用 `<s>` (1) / `</s>` (2) / `<pad>` (1)，BERT 用 [CLS] (0) / [SEP] (102)
//
// v1.0.0 直接用 `BertModel::load` 加载会因 key 不匹配失败（model_type=xlm-roberta
// 时 fallback 路径会尝试 `xlm-roberta.*` 而非 `roberta.*`，依然不匹配）。
// v1.1.0 自实现 XLM-RoBERTa 编码器，使用 `roberta.*` 前缀 + 正确的 position_ids。

/// XLM-RoBERTa 激活函数类型（与 BERT HiddenAct 对齐）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XlmRobertaAct {
    /// 标准 GELU（基于误差函数 erf，精度最高）
    Gelu,
    /// 近似 GELU（基于 tanh，速度快精度略低）
    GeluApproximate,
    /// ReLU（max(0, x)，最简单快速）
    Relu,
}

impl XlmRobertaAct {
    /// 前向激活
    fn forward(&self, xs: &Tensor) -> candle_core::Result<Tensor> {
        match self {
            Self::Gelu => xs.gelu_erf(),
            Self::GeluApproximate => xs.gelu(),
            Self::Relu => xs.relu(),
        }
    }
}

/// XLM-RoBERTa 配置（与 BERT Config 字段对齐，独立类型以便未来扩展）
#[derive(Debug, Clone, PartialEq)]
pub struct XlmRobertaConfig {
    pub vocab_size: usize,
    pub hidden_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub intermediate_size: usize,
    pub hidden_act: XlmRobertaAct,
    pub hidden_dropout_prob: f64,
    pub max_position_embeddings: usize,
    pub type_vocab_size: usize,
    pub initializer_range: f64,
    pub layer_norm_eps: f64,
    pub pad_token_id: usize,
}

/// 从 BERT `Config` 构造 XLM-RoBERTa 配置（用于 fallback 路径共享配置）
impl From<&Config> for XlmRobertaConfig {
    fn from(c: &Config) -> Self {
        let act = match c.hidden_act {
            candle_transformers::models::bert::HiddenAct::Gelu => XlmRobertaAct::Gelu,
            candle_transformers::models::bert::HiddenAct::GeluApproximate => {
                XlmRobertaAct::GeluApproximate
            }
            candle_transformers::models::bert::HiddenAct::Relu => XlmRobertaAct::Relu,
        };
        Self {
            vocab_size: c.vocab_size,
            hidden_size: c.hidden_size,
            num_hidden_layers: c.num_hidden_layers,
            num_attention_heads: c.num_attention_heads,
            intermediate_size: c.intermediate_size,
            hidden_act: act,
            hidden_dropout_prob: c.hidden_dropout_prob,
            max_position_embeddings: c.max_position_embeddings,
            type_vocab_size: c.type_vocab_size,
            initializer_range: c.initializer_range,
            layer_norm_eps: c.layer_norm_eps,
            pad_token_id: c.pad_token_id,
        }
    }
}

// ---------------------------------------------------------------------------
// XLM-RoBERTa 子模块（与 BERT 结构对齐，仅权重 key 前缀与 position_ids 不同）
// ---------------------------------------------------------------------------

/// Embedding 层：word + position + token_type → LayerNorm
struct XlmRobertaEmbeddings {
    word_embeddings: candle_nn::Embedding,
    position_embeddings: candle_nn::Embedding,
    token_type_embeddings: candle_nn::Embedding,
    layer_norm: candle_nn::LayerNorm,
    /// position_ids 起始偏移：XLM-R = pad_token_id + 1（默认 2）
    position_offset: u32,
}

impl XlmRobertaEmbeddings {
    fn load(vb: candle_nn::VarBuilder, config: &XlmRobertaConfig) -> candle_core::Result<Self> {
        let word_embeddings = candle_nn::embedding(
            config.vocab_size,
            config.hidden_size,
            vb.pp("word_embeddings"),
        )?;
        let position_embeddings = candle_nn::embedding(
            config.max_position_embeddings,
            config.hidden_size,
            vb.pp("position_embeddings"),
        )?;
        let token_type_embeddings = candle_nn::embedding(
            config.type_vocab_size,
            config.hidden_size,
            vb.pp("token_type_embeddings"),
        )?;
        let layer_norm = candle_nn::layer_norm(
            config.hidden_size,
            config.layer_norm_eps,
            vb.pp("LayerNorm"),
        )?;
        // XLM-R: position_ids 从 pad_token_id + 1 起算
        let position_offset = config.pad_token_id as u32 + 1;
        Ok(Self {
            word_embeddings,
            position_embeddings,
            token_type_embeddings,
            layer_norm,
            position_offset,
        })
    }

    fn forward(&self, input_ids: &Tensor) -> candle_core::Result<Tensor> {
        let (_bsize, seq_len) = input_ids.dims2()?;
        let word_emb = self.word_embeddings.forward(input_ids)?;

        // XLM-R 关键差异：position_ids = pad_token_id+1 .. pad_token_id+1+seq_len
        let start = self.position_offset;
        let position_ids: Vec<u32> = (start..start + seq_len as u32).collect();
        let position_ids = Tensor::new(position_ids.as_slice(), input_ids.device())?;
        let pos_emb = self.position_embeddings.forward(&position_ids)?;

        // token_type_ids = zeros（XLM-R 不使用 segment，但权重存在）
        let token_type_ids = input_ids.zeros_like()?;
        let tok_emb = self.token_type_embeddings.forward(&token_type_ids)?;

        let embeddings = word_emb.broadcast_add(&pos_emb)?.broadcast_add(&tok_emb)?;
        let embeddings = self.layer_norm.forward(&embeddings)?;
        Ok(embeddings)
    }
}

/// Self-Attention：query / key / value → scaled dot-product
struct XlmRobertaSelfAttention {
    query: candle_nn::Linear,
    key: candle_nn::Linear,
    value: candle_nn::Linear,
    num_heads: usize,
    head_size: usize,
}

impl XlmRobertaSelfAttention {
    fn load(vb: candle_nn::VarBuilder, config: &XlmRobertaConfig) -> candle_core::Result<Self> {
        let head_size = config.hidden_size / config.num_attention_heads;
        let all_head_size = config.num_attention_heads * head_size;
        let h = config.hidden_size;
        let query = candle_nn::linear(h, all_head_size, vb.pp("query"))?;
        let key = candle_nn::linear(h, all_head_size, vb.pp("key"))?;
        let value = candle_nn::linear(h, all_head_size, vb.pp("value"))?;
        Ok(Self {
            query,
            key,
            value,
            num_heads: config.num_attention_heads,
            head_size,
        })
    }

    fn transpose_for_scores(&self, xs: &Tensor) -> candle_core::Result<Tensor> {
        let mut shape = xs.dims().to_vec();
        shape.pop();
        shape.push(self.num_heads);
        shape.push(self.head_size);
        xs.reshape(shape.as_slice())?
            .transpose(1, 2)?
            .contiguous()
    }

    fn forward(
        &self,
        hidden: &Tensor,
        attention_mask: &Tensor,
    ) -> candle_core::Result<Tensor> {
        let q = self.transpose_for_scores(&self.query.forward(hidden)?)?;
        let k = self.transpose_for_scores(&self.key.forward(hidden)?)?;
        let v = self.transpose_for_scores(&self.value.forward(hidden)?)?;

        let scores = q.matmul(&k.t()?)?;
        let scores = (scores / (self.head_size as f64).sqrt())?;
        let scores = scores.broadcast_add(attention_mask)?;
        let probs = candle_nn::ops::softmax(&scores, candle_core::D::Minus1)?;
        let ctx = probs.matmul(&v)?;
        let ctx = ctx.transpose(1, 2)?.contiguous()?;
        ctx.flatten_from(candle_core::D::Minus2)
    }
}

/// Self-Output：dense + residual + LayerNorm
struct XlmRobertaSelfOutput {
    dense: candle_nn::Linear,
    layer_norm: candle_nn::LayerNorm,
}

impl XlmRobertaSelfOutput {
    fn load(vb: candle_nn::VarBuilder, config: &XlmRobertaConfig) -> candle_core::Result<Self> {
        let dense = candle_nn::linear(config.hidden_size, config.hidden_size, vb.pp("dense"))?;
        let layer_norm = candle_nn::layer_norm(
            config.hidden_size,
            config.layer_norm_eps,
            vb.pp("LayerNorm"),
        )?;
        Ok(Self { dense, layer_norm })
    }

    fn forward(&self, hidden: &Tensor, input: &Tensor) -> candle_core::Result<Tensor> {
        let hidden = self.dense.forward(hidden)?;
        self.layer_norm.forward(&(hidden + input)?)
    }
}

/// Attention 层：Self-Attention + Self-Output（残差 + LayerNorm）
struct XlmRobertaAttention {
    self_attention: XlmRobertaSelfAttention,
    self_output: XlmRobertaSelfOutput,
}

impl XlmRobertaAttention {
    fn load(vb: candle_nn::VarBuilder, config: &XlmRobertaConfig) -> candle_core::Result<Self> {
        let self_attention = XlmRobertaSelfAttention::load(vb.pp("self"), config)?;
        let self_output = XlmRobertaSelfOutput::load(vb.pp("output"), config)?;
        Ok(Self {
            self_attention,
            self_output,
        })
    }

    fn forward(
        &self,
        hidden: &Tensor,
        attention_mask: &Tensor,
    ) -> candle_core::Result<Tensor> {
        let attn_out = self.self_attention.forward(hidden, attention_mask)?;
        self.self_output.forward(&attn_out, hidden)
    }
}

/// Intermediate：dense + activation
struct XlmRobertaIntermediate {
    dense: candle_nn::Linear,
    act: XlmRobertaAct,
}

impl XlmRobertaIntermediate {
    fn load(vb: candle_nn::VarBuilder, config: &XlmRobertaConfig) -> candle_core::Result<Self> {
        let dense =
            candle_nn::linear(config.hidden_size, config.intermediate_size, vb.pp("dense"))?;
        Ok(Self {
            dense,
            act: config.hidden_act,
        })
    }

    fn forward(&self, hidden: &Tensor) -> candle_core::Result<Tensor> {
        self.act.forward(&self.dense.forward(hidden)?)
    }
}

/// Output：dense + residual + LayerNorm
struct XlmRobertaOutput {
    dense: candle_nn::Linear,
    layer_norm: candle_nn::LayerNorm,
}

impl XlmRobertaOutput {
    fn load(vb: candle_nn::VarBuilder, config: &XlmRobertaConfig) -> candle_core::Result<Self> {
        let dense = candle_nn::linear(config.intermediate_size, config.hidden_size, vb.pp("dense"))?;
        let layer_norm = candle_nn::layer_norm(
            config.hidden_size,
            config.layer_norm_eps,
            vb.pp("LayerNorm"),
        )?;
        Ok(Self { dense, layer_norm })
    }

    fn forward(&self, hidden: &Tensor, input: &Tensor) -> candle_core::Result<Tensor> {
        let hidden = self.dense.forward(hidden)?;
        self.layer_norm.forward(&(hidden + input)?)
    }
}

/// 单层 Transformer Encoder
struct XlmRobertaLayer {
    attention: XlmRobertaAttention,
    intermediate: XlmRobertaIntermediate,
    output: XlmRobertaOutput,
}

impl XlmRobertaLayer {
    fn load(vb: candle_nn::VarBuilder, config: &XlmRobertaConfig) -> candle_core::Result<Self> {
        let attention = XlmRobertaAttention::load(vb.pp("attention"), config)?;
        let intermediate = XlmRobertaIntermediate::load(vb.pp("intermediate"), config)?;
        let output = XlmRobertaOutput::load(vb.pp("output"), config)?;
        Ok(Self {
            attention,
            intermediate,
            output,
        })
    }

    fn forward(
        &self,
        hidden: &Tensor,
        attention_mask: &Tensor,
    ) -> candle_core::Result<Tensor> {
        let attn_out = self.attention.forward(hidden, attention_mask)?;
        let inter_out = self.intermediate.forward(&attn_out)?;
        self.output.forward(&inter_out, &attn_out)
    }
}

/// Encoder：堆叠 N 层 Transformer
struct XlmRobertaEncoder {
    layers: Vec<XlmRobertaLayer>,
}

impl XlmRobertaEncoder {
    fn load(vb: candle_nn::VarBuilder, config: &XlmRobertaConfig) -> candle_core::Result<Self> {
        let layers: Vec<XlmRobertaLayer> = (0..config.num_hidden_layers)
            .map(|i| XlmRobertaLayer::load(vb.pp(format!("layer.{i}")), config))
            .collect::<candle_core::Result<Vec<_>>>()?;
        Ok(Self { layers })
    }

    fn forward(
        &self,
        hidden: &Tensor,
        attention_mask: &Tensor,
    ) -> candle_core::Result<Tensor> {
        let mut hidden = hidden.clone();
        for layer in self.layers.iter() {
            hidden = layer.forward(&hidden, attention_mask)?;
        }
        Ok(hidden)
    }
}

/// XLM-RoBERTa 编码器（自实现，使用 `roberta.*` 权重前缀）
///
/// 与 `candle_transformers::models::bert::BertModel` 的差异：
/// 1. 权重 key 前缀为 `roberta.*`（调用方需 `vb.pp("roberta")`）
/// 2. position_ids 从 `pad_token_id + 1` 起算（XLM-R 约定）
/// 3. 不依赖 `model_type` fallback（直接使用 roberta 前缀，避免 BERT 的 fallback 逻辑误判）
pub struct XlmRobertaModel {
    embeddings: XlmRobertaEmbeddings,
    encoder: XlmRobertaEncoder,
}

impl XlmRobertaModel {
    /// 从 VarBuilder 加载 XLM-RoBERTa 权重
    ///
    /// 期望调用方传入已带 `roberta` 前缀的 VarBuilder，例如：
    /// ```ignore
    /// let model = XlmRobertaModel::load(vb.pp("roberta"), &config)?;
    /// ```
    /// 这样内部 `embeddings.*` / `encoder.layer.N.*` 会拼成 `roberta.embeddings.*` 等 key。
    pub fn load(vb: candle_nn::VarBuilder, config: &XlmRobertaConfig) -> Result<Self> {
        let embeddings = XlmRobertaEmbeddings::load(vb.pp("embeddings"), config)
            .map_err(|e| Error::internal(format!("XLM-R embeddings 加载失败: {e}")))?;
        let encoder = XlmRobertaEncoder::load(vb.pp("encoder"), config)
            .map_err(|e| Error::internal(format!("XLM-R encoder 加载失败: {e}")))?;
        Ok(Self {
            embeddings,
            encoder,
        })
    }

    /// 前向传播
    ///
    /// 输入：
    /// - `input_ids`: (batch, seq_len)
    /// - `attention_mask`: 可选 (batch, seq_len)，1=有效 / 0=padding
    ///
    /// 输出：`(batch, seq_len, hidden_size)`
    pub fn forward(&self, input_ids: &Tensor, attention_mask: Option<&Tensor>) -> Result<Tensor> {
        let embedding_output = self
            .embeddings
            .forward(input_ids)
            .map_err(|e| Error::internal(format!("XLM-R embeddings forward 失败: {e}")))?;

        let attention_mask = match attention_mask {
            Some(m) => m.clone(),
            None => input_ids
                .ones_like()
                .map_err(|e| Error::internal(format!("attention_mask 构造失败: {e}")))?,
        };
        let extended_mask = get_extended_attention_mask(&attention_mask, DType::F32)
            .map_err(|e| Error::internal(format!("extended_mask 构造失败: {e}")))?;

        let sequence_output = self
            .encoder
            .forward(&embedding_output, &extended_mask)
            .map_err(|e| Error::internal(format!("XLM-R encoder forward 失败: {e}")))?;
        Ok(sequence_output)
    }
}

/// 构造扩展 attention mask：(batch, seq_len) → (batch, 1, 1, seq_len)
///
/// padding 位置填充 f32::MIN（softmax 后趋近 0），有效位置填充 0。
fn get_extended_attention_mask(attention_mask: &Tensor, dtype: DType) -> candle_core::Result<Tensor> {
    let mask = attention_mask.unsqueeze(1)?.unsqueeze(1)?;
    let mask = mask.to_dtype(dtype)?;
    let ones = mask.ones_like()?;
    let inv = ones.broadcast_sub(&mask)?;
    inv.broadcast_mul(&Tensor::try_from(f32::MIN)?.to_device(mask.device())?)
}

/// BERT → XLM-RoBERTa 权重 key 映射
///
/// 将 `bert.` 前缀替换为 `roberta.`，其余路径保持不变。
/// 用于在 BERT fallback 路径下诊断/转换权重 key（实际加载时直接用 `vb.pp("roberta")`）。
///
/// # 示例
///
/// ```
/// use sparkfox_embedding::reranker::xlm_roberta_key_mapping;
/// assert_eq!(
///     xlm_roberta_key_mapping("bert.encoder.layer.0.attention.self.query.weight"),
///     "roberta.encoder.layer.0.attention.self.query.weight"
/// );
/// // 非 `bert.` 前缀的 key 原样返回
/// assert_eq!(
///     xlm_roberta_key_mapping("classifier.weight"),
///     "classifier.weight"
/// );
/// ```
pub fn xlm_roberta_key_mapping(bert_key: &str) -> String {
    if let Some(rest) = bert_key.strip_prefix("bert.") {
        format!("roberta.{rest}")
    } else {
        bert_key.to_string()
    }
}

/// 重排器架构类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RerankerArch {
    XlmRoberta,
    Bert,
}

/// 重排结果
///
/// 由 [`BgeReranker::rerank`] 返回，按 `score` 降序排序。
#[derive(Debug, Clone, PartialEq)]
pub struct RerankResult {
    /// 原始文档索引（在输入 `documents` 切片中的位置）
    pub index: usize,
    /// 相关性分数（sigmoid 后，范围 0.0-1.0，越大越相关）
    pub score: f32,
    /// 原始文档文本
    pub document: String,
}

/// bge-reranker-v2-m3 重排器（CrossEncoder 架构）
///
/// v1.1.0 架构修复（RISK-v1.1-04 缓解）：
/// - 优先按 XLM-RoBERTa 架构加载（`roberta.*` 权重 key，正确 position_ids 偏移）
/// - 旧 BERT 路径保留为 fallback（应对权重 key 为 `bert.*` 的非典型场景）
///
/// 加载流程（[`BgeReranker::load_from_dir`]）：
/// 1. `download_model` 下载/查找模型文件（复用 downloader）
/// 2. `verify_model_files` 强制 SHA256 校验（防供应链攻击，与 BgeEmbedder 一致）
/// 3. **先尝试 XLM-RoBERTa 加载**（[`XlmRobertaModel::load`]，`vb.pp("roberta")`）
/// 4. 若失败，**回退到 BERT 加载**（[`BertModel::load`]，`vb.pp("bert")`）
/// 5. 加载 classifier 二分类头（`vb.pp("classifier")`，两条路径共用）
///
/// 推理流程（[`BgeReranker::score_pair`]）：
/// 1. 拼接 `query [SEP] document`
/// 2. tokenize → input_ids / attention_mask
/// 3. encoder forward → (1, L, H)
/// 4. 取 [CLS] token hidden（index 0 along seq_len）→ (1, H)
/// 5. classifier Linear → logit (1, 1)
/// 6. sigmoid → score [0, 1]
pub struct BgeReranker {
    /// 编码器（XLM-RoBERTa 优先，BERT fallback）
    model: RerankerEncoder,
    /// tokenizer（与 BgeEmbedder 同款 tokenizers 库）
    tokenizer: Tokenizer,
    /// 推理设备（桌面端默认 CPU，GPU 留 v1.1+）
    device: Device,
    /// 二分类头：hidden_size → 1，输出 logit
    classifier: Linear,
}

/// 内部编码器枚举（XLM-RoBERTa 优先，BERT fallback）
///
/// 由 [`BgeReranker::load_from_dir`] 根据 weights key 自动选择：
/// - 优先 XLM-RoBERTa（`roberta.*` 权重 key）
/// - 回退 BERT（`bert.*` 权重 key）
enum RerankerEncoder {
    XlmRoberta(XlmRobertaModel),
    Bert(BertModel),
}

impl RerankerEncoder {
    /// 前向传播，返回 (batch, seq_len, hidden_size)
    ///
    /// XLM-RoBERTa 不需要 token_type_ids（内部传 zeros），BERT 需要。
    fn forward(
        &self,
        input_ids: &Tensor,
        token_type_ids: &Tensor,
        attention_mask: Option<&Tensor>,
    ) -> Result<Tensor> {
        match self {
            Self::XlmRoberta(m) => m.forward(input_ids, attention_mask),
            Self::Bert(m) => m
                .forward(input_ids, token_type_ids, attention_mask)
                .map_err(|e| Error::internal(format!("BertModel forward 失败: {e}"))),
        }
    }

    /// 返回架构类型
    fn arch(&self) -> RerankerArch {
        match self {
            Self::XlmRoberta(_) => RerankerArch::XlmRoberta,
            Self::Bert(_) => RerankerArch::Bert,
        }
    }
}

impl BgeReranker {
    /// 加载重排模型
    ///
    /// 复用 downloader 的 `download_model` + `verify_model_files`，
    /// 强制 SHA256 校验（与 [`BgeEmbedder::load`](crate::BgeEmbedder::load) 一致的安全策略）。
    ///
    /// 通常 `variant` 传 `ModelVariant::BgeRerankerV2M3`，
    /// 保留参数以便未来支持更多 reranker 变体。
    pub fn load(variant: ModelVariant) -> Result<Self> {
        let model_dir = download_model(&variant)?;
        // 【S-02 P0】强制 SHA256 校验
        verify_model_files(&model_dir, &variant)?;
        let device = Device::Cpu; // 桌面端默认 CPU，GPU 留 v1.1+
        Self::load_from_dir(&model_dir, &device)
    }

    /// 从指定目录加载模型（用于测试或自定义路径）
    ///
    /// 与 [`BgeEmbedder::load_from_dir`](crate::BgeEmbedder::load_from_dir) 对称：
    /// 不走 downloader，直接从本地目录加载，便于离线测试。
    ///
    /// v1.1.0 加载策略（XLM-RoBERTa 优先，BERT fallback）：
    /// 1. 解析 config.json（共用 BERT Config，因字段对齐）
    /// 2. mmap 加载 model.safetensors
    /// 3. **先尝试 XLM-RoBERTa 加载**：`XlmRobertaModel::load(vb.pp("roberta"), &xlm_config)`
    ///    - 若权重含 `roberta.*` key，加载成功
    /// 4. **若 XLM-R 失败，回退 BERT 加载**：`BertModel::load(vb.pp("bert"), &config)`
    ///    - 应对 `bert.*` key 的非典型场景（RISK-v1.1-04 缓解）
    /// 5. 加载 classifier（`vb.pp("classifier")`，两条路径共用）
    pub fn load_from_dir(model_dir: &PathBuf, device: &Device) -> Result<Self> {
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

        // 加载权重（mmap，unsafe）— 与 BgeEmbedder 一致的安全策略
        // SAFETY: from_mmaped_safetensors 使用 mmap，文件路径由我们控制
        // （已通过 hf-hub 下载或用户指定），不会在运行时被修改。
        let vb = {
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

        // v1.1.0：XLM-RoBERTa 优先，BERT fallback
        let xlm_config = XlmRobertaConfig::from(&config);
        let model = match XlmRobertaModel::load(vb.clone().pp("roberta"), &xlm_config) {
            Ok(m) => {
                log::info!(
                    "bge-reranker-v2-m3 按 XLM-RoBERTa 架构加载成功（roberta.* 权重 key）"
                );
                RerankerEncoder::XlmRoberta(m)
            }
            Err(xlm_err) => {
                log::warn!(
                    "XLM-RoBERTa 加载失败（{xlm_err}），回退到 BERT 加载路径（v1.0.0 兼容）"
                );
                let bert_model = BertModel::load(vb.clone().pp("bert"), &config)
                    .map_err(|e| Error::internal(format!(
                        "BERT fallback 加载也失败（XLM-R 错误: {xlm_err}; BERT 错误: {e}）"
                    )))?;
                log::info!("bge-reranker-v2-m3 按 BERT 架构加载成功（bert.* 权重 key，fallback）");
                RerankerEncoder::Bert(bert_model)
            }
        };

        // 加载二分类头 classifier: hidden_size → 1
        // bge-reranker-v2-m3 的 classifier 权重在 `classifier.weight` / `classifier.bias`
        let hidden_size = config.hidden_size;
        let classifier = linear(hidden_size, 1, vb.pp("classifier"))
            .map_err(|e| Error::internal(format!("classifier 加载失败: {e}")))?;

        Ok(Self {
            model,
            tokenizer,
            device: device.clone(),
            classifier,
        })
    }

    /// 返回当前加载的架构类型（XLM-RoBERTa 或 BERT）
    ///
    /// 用于诊断与监控：若生产环境报告 `Bert`，说明权重 key 不匹配，
    /// 需检查模型文件是否正确下载。
    pub fn arch_type(&self) -> RerankerArch {
        self.model.arch()
    }

    /// 对单个 (query, document) 对打分
    ///
    /// 返回 sigmoid 后的相关性分数（范围 0.0-1.0，越大越相关）。
    ///
    /// 流程：
    /// 1. 拼接 `query [SEP] document`（tokenizer 自动添加 [CLS] / [SEP]）
    /// 2. BERT forward → (1, L, H)
    /// 3. 取 [CLS] token hidden → (1, H)
    /// 4. classifier Linear → logit (1, 1)
    /// 5. sigmoid → score [0, 1]
    fn score_pair(&self, query: &str, document: &str) -> Result<f32> {
        // CrossEncoder 输入：[CLS] query [SEP] document [SEP]
        // tokenizer 的 add_special_tokens=true 会自动添加 [CLS] 和 [SEP]
        let text = format!("{query} [SEP] {document}");
        let tokens = self
            .tokenizer
            .encode(text.as_str(), true)
            .map_err(|e| Error::internal(format!("tokenize 失败: {e}")))?;

        let ids = tokens.get_ids();
        let input_ids = Tensor::from_slice(ids as &[u32], (1, ids.len()), &self.device)
            .map_err(|e| Error::internal(format!("input_ids 构造失败: {e}")))?;

        let token_type_ids = input_ids
            .zeros_like()
            .map_err(|e| Error::internal(format!("token_type_ids 构造失败: {e}")))?;

        let attention_mask = input_ids
            .ones_like()
            .map_err(|e| Error::internal(format!("attention_mask 构造失败: {e}")))?;

        // forward: (1, L, H)
        let output = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask))
            .map_err(|e| Error::internal(format!("forward 失败: {e}")))?;

        // 取 [CLS] token（seq_len 维度 index 0）：(1, L, H) → (1, 1, H) → (1, H)
        let cls_hidden = output
            .narrow(1, 0, 1)
            .map_err(|e| Error::internal(format!("narrow [CLS] 失败: {e}")))?
            .squeeze(1)
            .map_err(|e| Error::internal(format!("squeeze seq_len 失败: {e}")))?;
        // cls_hidden: (1, H)

        // classifier: (1, H) → (1, 1)
        let logit = self
            .classifier
            .forward(&cls_hidden)
            .map_err(|e| Error::internal(format!("classifier forward 失败: {e}")))?;
        // logit: (1, 1)

        // 取标量 logit：(1, 1) → (1,) → f32
        let logit_vec = logit
            .squeeze(0)
            .map_err(|e| Error::internal(format!("squeeze batch 失败: {e}")))?
            .to_vec1::<f32>()
            .map_err(|e| Error::internal(format!("to_vec1 失败: {e}")))?;
        let logit_val = logit_vec[0];

        // sigmoid → [0, 1]
        Ok(sigmoid(logit_val))
    }

    /// 批量重排：对 (query, doc) 对打分并按分数降序排序
    ///
    /// 返回所有文档的 [`RerankResult`]（按分数降序）。
    /// 若需 top-K 截断，使用 [`rerank_top_k`](Self::rerank_top_k)。
    ///
    /// 单条打分失败时该文档分数记为 0.0（不中断整体流程），
    /// 与 spec 1.0 第 1814 行 `unwrap_or(0.0)` 行为一致。
    pub fn rerank(&self, query: &str, documents: &[String]) -> Result<Vec<RerankResult>> {
        let mut results: Vec<RerankResult> = documents
            .iter()
            .enumerate()
            .map(|(i, doc)| {
                let score = self.score_pair(query, doc).unwrap_or(0.0);
                RerankResult {
                    index: i,
                    score,
                    document: doc.clone(),
                }
            })
            .collect();

        // 按分数降序排序（NaN 安全：partial_cmp + unwrap_or(Equal)）
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results)
    }

    /// 批量重排 + top-K 截断
    ///
    /// 等价于 [`rerank`](Self::rerank) 后取前 `top_k` 条。
    /// 若 `top_k >= documents.len()`，返回全部结果。
    pub fn rerank_top_k(
        &self,
        query: &str,
        documents: &[String],
        top_k: usize,
    ) -> Result<Vec<RerankResult>> {
        let mut results = self.rerank(query, documents)?;
        results.truncate(top_k);
        Ok(results)
    }

    /// 兼容 spec 1.0 Task 5.2 的接口：返回 (index, score) 元组
    ///
    /// 与 [`rerank_top_k`](Self::rerank_top_k) 等价，但返回简化的元组形式，
    /// 便于 `sparkfox-knowledge::RagEngine::rerank_search` 集成
    /// （spec 1.0 第 1812-1819 行 / 1856-1862 行）。
    pub fn rerank_pairs(
        &self,
        query: &str,
        documents: &[String],
        top_k: usize,
    ) -> Result<Vec<(usize, f32)>> {
        let results = self.rerank_top_k(query, documents, top_k)?;
        Ok(results.into_iter().map(|r| (r.index, r.score)).collect())
    }
}

/// sigmoid 函数：1 / (1 + e^(-x))
///
/// 将 logit 映射到 [0, 1] 区间，作为相关性概率。
/// 对大正值趋近 1，大负值趋近 0，x=0 时为 0.5。
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigmoid_zero_is_half() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sigmoid_positive_approaches_one() {
        assert!(sigmoid(10.0) > 0.999);
        assert!((sigmoid(10.0) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn sigmoid_negative_approaches_zero() {
        assert!(sigmoid(-10.0) < 1e-3);
        assert!((sigmoid(-10.0) - 0.0).abs() < 1e-3);
    }

    #[test]
    fn sigmoid_monotonic_increasing() {
        let s1 = sigmoid(-1.0);
        let s2 = sigmoid(0.0);
        let s3 = sigmoid(1.0);
        assert!(s1 < s2, "sigmoid 应单调递增: {s1} < {s2} 失败");
        assert!(s2 < s3, "sigmoid 应单调递增: {s2} < {s3} 失败");
    }

    #[test]
    fn sigmoid_output_in_unit_range() {
        for x in [-100.0, -10.0, -1.0, 0.0, 1.0, 10.0, 100.0] {
            let s = sigmoid(x);
            assert!(s >= 0.0 && s <= 1.0, "sigmoid({x}) = {s} 超出 [0,1]");
        }
    }

    #[test]
    fn rerank_result_sort_descending() {
        // 构造无序的 RerankResult 列表，按 score 降序排序
        let mut results = vec![
            RerankResult {
                index: 0,
                score: 0.3,
                document: "doc0".to_string(),
            },
            RerankResult {
                index: 1,
                score: 0.9,
                document: "doc1".to_string(),
            },
            RerankResult {
                index: 2,
                score: 0.5,
                document: "doc2".to_string(),
            },
        ];
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        assert_eq!(results[0].index, 1);
        assert!((results[0].score - 0.9).abs() < 1e-6);
        assert_eq!(results[1].index, 2);
        assert_eq!(results[2].index, 0);
    }

    #[test]
    fn rerank_result_preserves_document() {
        let r = RerankResult {
            index: 5,
            score: 0.8,
            document: "hello world".to_string(),
        };
        assert_eq!(r.index, 5);
        assert_eq!(r.document, "hello world");
        assert!((r.score - 0.8).abs() < 1e-6);
    }

    #[test]
    fn rerank_top_k_truncation_logic() {
        // 模拟 rerank_top_k 的截断逻辑（不加载真实模型）
        let mut results: Vec<RerankResult> = (0..10)
            .map(|i| RerankResult {
                index: i,
                score: i as f32 * 0.1,
                document: format!("doc{i}"),
            })
            .collect();
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(3);
        assert_eq!(results.len(), 3);
        // 降序：index 9 (0.9), 8 (0.8), 7 (0.7)
        assert_eq!(results[0].index, 9);
        assert_eq!(results[1].index, 8);
        assert_eq!(results[2].index, 7);
    }

    #[test]
    fn rerank_pairs_returns_tuples() {
        // 验证 rerank_pairs 的元组转换逻辑
        let results = vec![
            RerankResult {
                index: 2,
                score: 0.9,
                document: "doc2".to_string(),
            },
            RerankResult {
                index: 0,
                score: 0.5,
                document: "doc0".to_string(),
            },
        ];
        let pairs: Vec<(usize, f32)> =
            results.into_iter().map(|r| (r.index, r.score)).collect();
        assert_eq!(pairs, vec![(2, 0.9), (0, 0.5)]);
    }

    #[test]
    fn rerank_result_sort_with_nan_safety() {
        // NaN 安全性：partial_cmp 返回 None 时回退为 Equal
        let mut results = vec![
            RerankResult {
                index: 0,
                score: f32::NAN,
                document: "nan".to_string(),
            },
            RerankResult {
                index: 1,
                score: 0.5,
                document: "half".to_string(),
            },
        ];
        // 不应 panic
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        assert_eq!(results.len(), 2);
    }

    /// 需要真实模型文件的集成测试（560MB bge-reranker-v2-m3）
    ///
    /// 运行方式：
    /// ```bash
    /// cargo test -p sparkfox-embedding --lib reranker_load_and_score -- --ignored
    /// ```
    ///
    /// 前置条件：模型已下载到 `SPARKFOX_MODELS_DIR` 或 `cache_dir()`
    #[test]
    #[ignore = "需要 560MB 真实模型文件"]
    fn reranker_load_and_score() {
        let reranker = BgeReranker::load(ModelVariant::BgeRerankerV2M3)
            .expect("加载 bge-reranker-v2-m3 失败");
        let docs = vec![
            "SparkFox 是一个桌面端 AI Agent 应用".to_string(),
            "今天天气很好".to_string(),
            "Rust 是一门系统编程语言".to_string(),
        ];
        let results = reranker
            .rerank("SparkFox 是什么", &docs)
            .expect("rerank 失败");
        assert_eq!(results.len(), 3);
        // 分数应在 [0, 1] 区间
        for r in &results {
            assert!(
                r.score >= 0.0 && r.score <= 1.0,
                "score out of range: {}",
                r.score
            );
        }
        // 最相关的应是 doc0（与 query 主题一致）
        println!("rerank results:");
        for r in &results {
            println!("  index={} score={:.4} doc={:?}", r.index, r.score, r.document);
        }
    }
}
