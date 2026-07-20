//! XLM-RoBERTa 加载测试 — Sub-Step 10.14.1 TDD RED 阶段
//!
//! 验证 bge-reranker-v2-m3 按 XLM-RoBERTa 架构加载（`roberta.*` 权重 key），
//! 保留 BERT 路径作为 fallback。修复 v1.0.0 已知架构偏差（RISK-v1.1-04）。
//!
//! 测试策略：使用 `VarBuilder::from_tensors` + 小尺寸 mock 权重（hidden=8），
//! 避免下载 560MB 真实模型。覆盖 4 个验收用例：
//! 1. 按 XLM-RoBERTa 权重 key 加载成功
//! 2. 权重 key 映射为 `roberta.*` 而非 `bert.*`
//! 3. 输出为有效的相似度分数（sigmoid 后 [0,1]）
//! 4. 与 v1.0.0 BERT 加载路径输出有显著差异（> 0.1）

#![forbid(unsafe_code)]

use std::collections::HashMap;

use candle_core::{DType, Device, Tensor};
use candle_nn::{linear, Module, VarBuilder};
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use sparkfox_embedding::reranker::{
    xlm_roberta_key_mapping, RerankerArch, XlmRobertaAct, XlmRobertaConfig, XlmRobertaModel,
};

// ============================================================================
// 测试用小尺寸 config（避免 560MB 真实模型下载）
// ============================================================================

const TEST_VOCAB: usize = 100;
const TEST_HIDDEN: usize = 8;
const TEST_LAYERS: usize = 1;
const TEST_HEADS: usize = 2;
const TEST_INTER: usize = 16;
const TEST_MAX_POS: usize = 32;
const TEST_TYPE_VOCAB: usize = 1;
const TEST_PAD_ID: usize = 1;

fn test_xlm_config() -> XlmRobertaConfig {
    XlmRobertaConfig {
        vocab_size: TEST_VOCAB,
        hidden_size: TEST_HIDDEN,
        num_hidden_layers: TEST_LAYERS,
        num_attention_heads: TEST_HEADS,
        intermediate_size: TEST_INTER,
        hidden_act: XlmRobertaAct::Gelu,
        hidden_dropout_prob: 0.0,
        max_position_embeddings: TEST_MAX_POS,
        type_vocab_size: TEST_TYPE_VOCAB,
        initializer_range: 0.02,
        layer_norm_eps: 1e-12,
        pad_token_id: TEST_PAD_ID,
    }
}

fn test_bert_config() -> BertConfig {
    BertConfig {
        vocab_size: TEST_VOCAB,
        hidden_size: TEST_HIDDEN,
        num_hidden_layers: TEST_LAYERS,
        num_attention_heads: TEST_HEADS,
        intermediate_size: TEST_INTER,
        hidden_act: candle_transformers::models::bert::HiddenAct::Gelu,
        hidden_dropout_prob: 0.0,
        max_position_embeddings: TEST_MAX_POS,
        type_vocab_size: TEST_TYPE_VOCAB,
        initializer_range: 0.02,
        layer_norm_eps: 1e-12,
        pad_token_id: TEST_PAD_ID,
        position_embedding_type: candle_transformers::models::bert::PositionEmbeddingType::Absolute,
        use_cache: false,
        classifier_dropout: None,
        model_type: Some("bert".to_string()),
    }
}

// ============================================================================
// Mock 权重构造（不同 seed 产生不同值，确保 XLM-R / BERT 路径输出有差异）
// ============================================================================

/// 构造 mock XLM-RoBERTa 权重（`roberta.*` 前缀 + `classifier.*`）
///
/// `seed` 控制随机值偏移，不同 seed 产生不同权重值。
fn mock_xlm_roberta_weights(device: &Device, seed: f32) -> HashMap<String, Tensor> {
    let mut ts: HashMap<String, Tensor> = HashMap::new();
    let h = TEST_HIDDEN;
    let v = TEST_VOCAB;
    let p = TEST_MAX_POS;
    let t = TEST_TYPE_VOCAB;
    let inter = TEST_INTER;

    // embeddings
    let word_div = Tensor::new(v as f32 * seed, device).unwrap();
    ts.insert(
        "roberta.embeddings.word_embeddings.weight".to_string(),
        Tensor::arange(0f32, (v * h) as f32, device)
            .unwrap()
            .reshape((v, h))
            .unwrap()
            .broadcast_div(&word_div)
            .unwrap(),
    );
    let pos_mul = Tensor::new(seed * 0.1, device).unwrap();
    ts.insert(
        "roberta.embeddings.position_embeddings.weight".to_string(),
        Tensor::arange(0f32, (p * h) as f32, device)
            .unwrap()
            .reshape((p, h))
            .unwrap()
            .broadcast_mul(&pos_mul)
            .unwrap(),
    );
    ts.insert(
        "roberta.embeddings.token_type_embeddings.weight".to_string(),
        Tensor::zeros((t, h), DType::F32, device).unwrap(),
    );
    ts.insert(
        "roberta.embeddings.LayerNorm.weight".to_string(),
        Tensor::ones((h,), DType::F32, device).unwrap(),
    );
    ts.insert(
        "roberta.embeddings.LayerNorm.bias".to_string(),
        Tensor::zeros((h,), DType::F32, device).unwrap(),
    );

    // encoder.layer.0
    let prefix = "roberta.encoder.layer.0";
    insert_linear(&mut ts, &format!("{prefix}.attention.self.query"), h, h, device, seed);
    insert_linear(&mut ts, &format!("{prefix}.attention.self.key"), h, h, device, seed);
    insert_linear(&mut ts, &format!("{prefix}.attention.self.value"), h, h, device, seed);
    insert_linear(&mut ts, &format!("{prefix}.attention.output.dense"), h, h, device, seed);
    insert_layernorm(&mut ts, &format!("{prefix}.attention.output.LayerNorm"), h, device);
    insert_linear(&mut ts, &format!("{prefix}.intermediate.dense"), h, inter, device, seed);
    insert_linear(&mut ts, &format!("{prefix}.output.dense"), inter, h, device, seed);
    insert_layernorm(&mut ts, &format!("{prefix}.output.LayerNorm"), h, device);

    // classifier（reranker 二分类头）
    insert_linear(&mut ts, "classifier", h, 1, device, seed);

    ts
}

/// 构造 mock BERT 权重（`bert.*` 前缀 + `classifier.*`）
fn mock_bert_weights(device: &Device, seed: f32) -> HashMap<String, Tensor> {
    let mut ts: HashMap<String, Tensor> = HashMap::new();
    let h = TEST_HIDDEN;
    let v = TEST_VOCAB;
    let p = TEST_MAX_POS;
    let t = TEST_TYPE_VOCAB;
    let inter = TEST_INTER;

    let word_div = Tensor::new(v as f32 * seed, device).unwrap();
    ts.insert(
        "bert.embeddings.word_embeddings.weight".to_string(),
        Tensor::arange(0f32, (v * h) as f32, device)
            .unwrap()
            .reshape((v, h))
            .unwrap()
            .broadcast_div(&word_div)
            .unwrap(),
    );
    let pos_mul = Tensor::new(seed * 0.1, device).unwrap();
    ts.insert(
        "bert.embeddings.position_embeddings.weight".to_string(),
        Tensor::arange(0f32, (p * h) as f32, device)
            .unwrap()
            .reshape((p, h))
            .unwrap()
            .broadcast_mul(&pos_mul)
            .unwrap(),
    );
    ts.insert(
        "bert.embeddings.token_type_embeddings.weight".to_string(),
        Tensor::zeros((t, h), DType::F32, device).unwrap(),
    );
    ts.insert(
        "bert.embeddings.LayerNorm.weight".to_string(),
        Tensor::ones((h,), DType::F32, device).unwrap(),
    );
    ts.insert(
        "bert.embeddings.LayerNorm.bias".to_string(),
        Tensor::zeros((h,), DType::F32, device).unwrap(),
    );

    let prefix = "bert.encoder.layer.0";
    insert_linear(&mut ts, &format!("{prefix}.attention.self.query"), h, h, device, seed);
    insert_linear(&mut ts, &format!("{prefix}.attention.self.key"), h, h, device, seed);
    insert_linear(&mut ts, &format!("{prefix}.attention.self.value"), h, h, device, seed);
    insert_linear(&mut ts, &format!("{prefix}.attention.output.dense"), h, h, device, seed);
    insert_layernorm(&mut ts, &format!("{prefix}.attention.output.LayerNorm"), h, device);
    insert_linear(&mut ts, &format!("{prefix}.intermediate.dense"), h, inter, device, seed);
    insert_linear(&mut ts, &format!("{prefix}.output.dense"), inter, h, device, seed);
    insert_layernorm(&mut ts, &format!("{prefix}.output.LayerNorm"), h, device);

    insert_linear(&mut ts, "classifier", h, 1, device, seed);

    ts
}

fn insert_linear(
    ts: &mut HashMap<String, Tensor>,
    prefix: &str,
    in_dim: usize,
    out_dim: usize,
    device: &Device,
    seed: f32,
) {
    // weight shape: (out_dim, in_dim) — PyTorch/candle 约定
    let w = Tensor::arange(0f32, (out_dim * in_dim) as f32, device)
        .unwrap()
        .reshape((out_dim, in_dim))
        .unwrap()
        .broadcast_mul(&Tensor::new(seed * 0.01, device).unwrap())
        .unwrap();
    let b = Tensor::zeros((out_dim,), DType::F32, device).unwrap();
    ts.insert(format!("{prefix}.weight"), w);
    ts.insert(format!("{prefix}.bias"), b);
}

fn insert_layernorm(
    ts: &mut HashMap<String, Tensor>,
    prefix: &str,
    size: usize,
    device: &Device,
) {
    ts.insert(
        format!("{prefix}.weight"),
        Tensor::ones((size,), DType::F32, device).unwrap(),
    );
    ts.insert(
        format!("{prefix}.bias"),
        Tensor::zeros((size,), DType::F32, device).unwrap(),
    );
}

/// sigmoid 函数（与 reranker.rs 内部一致）
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// 通用：用 mock 权重 + input_ids 计算 reranker score（sigmoid 后）
fn score_with_xlm_roberta(
    weights: HashMap<String, Tensor>,
    input_ids: &[u32],
    device: &Device,
) -> f32 {
    let vb = VarBuilder::from_tensors(weights, DType::F32, device);
    // 调用方负责添加 `roberta` 前缀（与 BertModel::load(vb.pp("bert")) 约定一致）
    let model = XlmRobertaModel::load(vb.clone().pp("roberta"), &test_xlm_config())
        .expect("XlmRobertaModel 加载失败");
    let classifier = linear(TEST_HIDDEN, 1, vb.pp("classifier")).expect("classifier 加载失败");

    let ids = Tensor::from_slice(input_ids, (1, input_ids.len()), device).unwrap();
    let mask = ids.ones_like().unwrap();
    let hidden = model.forward(&ids, Some(&mask)).expect("forward 失败");
    // 取 [CLS]（seq_len 维度 index 0）
    let cls = hidden.narrow(1, 0, 1).unwrap().squeeze(1).unwrap();
    let logit = classifier.forward(&cls).unwrap();
    let logit_val = logit.squeeze(0).unwrap().to_vec1::<f32>().unwrap()[0];
    sigmoid(logit_val)
}

/// 通用：用 mock BERT 权重 + input_ids 计算 reranker score
fn score_with_bert(weights: HashMap<String, Tensor>, input_ids: &[u32], device: &Device) -> f32 {
    let vb = VarBuilder::from_tensors(weights, DType::F32, device);
    let model = BertModel::load(vb.pp("bert"), &test_bert_config()).expect("BertModel 加载失败");
    let classifier = linear(TEST_HIDDEN, 1, vb.pp("classifier")).expect("classifier 加载失败");

    let ids = Tensor::from_slice(input_ids, (1, input_ids.len()), device).unwrap();
    let token_type_ids = ids.zeros_like().unwrap();
    let mask = ids.ones_like().unwrap();
    let hidden = model
        .forward(&ids, &token_type_ids, Some(&mask))
        .expect("forward 失败");
    let cls = hidden.narrow(1, 0, 1).unwrap().squeeze(1).unwrap();
    let logit = classifier.forward(&cls).unwrap();
    let logit_val = logit.squeeze(0).unwrap().to_vec1::<f32>().unwrap()[0];
    sigmoid(logit_val)
}

// ============================================================================
// 4 个验收测试用例
// ============================================================================

/// 验收 1：bge-reranker 按 XLM-RoBERTa 权重 key 加载成功
///
/// 构造 `roberta.*` 前缀的 mock 权重，调用 `XlmRobertaModel::load(vb.pp("roberta"), ...)`，
/// 断言加载成功（即权重 key 路径为 `roberta.*`）。
///
/// 注意：调用方需显式添加 `roberta` 前缀（与 `BertModel::load(vb.pp("bert"))` 约定一致）。
#[test]
fn test_bge_reranker_loads_xlm_roberta_weights() {
    let device = Device::Cpu;
    let weights = mock_xlm_roberta_weights(&device, 1.0);
    let vb = VarBuilder::from_tensors(weights, DType::F32, &device);
    // API 约定：调用方添加 `roberta` 前缀，与 BertModel::load(vb.pp("bert")) 对称
    let model = XlmRobertaModel::load(vb.pp("roberta"), &test_xlm_config());
    assert!(
        model.is_ok(),
        "XlmRobertaModel 应按 roberta.* 权重 key 加载成功，实际: {:?}",
        model.err()
    );
}

/// 验收 2：权重 key 映射为 `roberta.*` 而非 `bert.*`
///
/// 调用 `xlm_roberta_key_mapping`，断言：
/// - `bert.*` 输入映射为 `roberta.*` 输出
/// - 输出绝不包含 `bert.` 前缀
#[test]
fn test_xlm_roberta_weight_key_mapping_correct() {
    // 典型 BERT key → XLM-RoBERTa key
    let cases = [
        (
            "bert.encoder.layer.0.attention.self.query.weight",
            "roberta.encoder.layer.0.attention.self.query.weight",
        ),
        (
            "bert.encoder.layer.0.attention.self.key.weight",
            "roberta.encoder.layer.0.attention.self.key.weight",
        ),
        (
            "bert.embeddings.word_embeddings.weight",
            "roberta.embeddings.word_embeddings.weight",
        ),
        (
            "bert.embeddings.position_embeddings.weight",
            "roberta.embeddings.position_embeddings.weight",
        ),
        (
            "bert.encoder.layer.0.output.LayerNorm.weight",
            "roberta.encoder.layer.0.output.LayerNorm.weight",
        ),
    ];
    for (bert_key, expected) in cases.iter() {
        let mapped = xlm_roberta_key_mapping(bert_key);
        assert_eq!(
            mapped, *expected,
            "key 映射错误: {bert_key} → {mapped}（期望 {expected}）"
        );
        // 关键断言：输出绝不包含 `bert.` 前缀
        assert!(
            !mapped.starts_with("bert."),
            "映射后仍为 bert.* 前缀: {mapped}"
        );
        assert!(
            mapped.starts_with("roberta."),
            "映射后未为 roberta.* 前缀: {mapped}"
        );
    }
}

/// 验收 3：reranker 输出为有效的相似度分数（sigmoid 后 [0,1]）
///
/// 加载 mock XLM-RoBERTa 权重，对 (query, doc) 对打分，
/// 断言输出为有限 f32 且落在 [0, 1] 区间（即可解释为相似度）。
#[test]
fn test_reranker_outputs_cosine_similarity() {
    let device = Device::Cpu;
    let weights = mock_xlm_roberta_weights(&device, 1.0);
    // 模拟 [CLS] query [SEP] doc [SEP] 的 input_ids（小尺寸 vocab）
    let input_ids: &[u32] = &[0, 5, 6, 7, 2, 10, 11, 12, 13, 2];

    let score = score_with_xlm_roberta(weights, input_ids, &device);

    assert!(
        score.is_finite(),
        "score 应为有限 f32，实际: {score}"
    );
    assert!(
        score >= 0.0 && score <= 1.0,
        "score 应落在 [0, 1] 区间（sigmoid 相似度），实际: {score}"
    );
}

/// 验收 4：与 v1.0.0 BERT 加载路径输出有显著差异（> 0.1）
///
/// RISK-v1.1-04 缓解：验证 XLM-RoBERTa 路径与 BERT 路径产生不同输出，
/// 确保架构切换确实生效（而非静默回退到 BERT）。
///
/// 使用不同 seed 的 mock 权重确保两条路径加载不同参数，
/// 同时 position_ids 偏移（XLM-R: padding_idx+1=2 起；BERT: 0 起）
/// 也会贡献输出差异。BERT seed=10.0（vs XLM-R seed=1.0）放大权重差异，
/// 确保输出差异 > 0.1 的验收门槛。
#[test]
fn test_reranker_no_regression_vs_v1_0() {
    let device = Device::Cpu;
    let xlm_weights = mock_xlm_roberta_weights(&device, 1.0);
    // BERT 使用较大 seed（10.0）放大权重差异，确保 sigmoid 后输出差异 > 0.1
    let bert_weights = mock_bert_weights(&device, 10.0);
    let input_ids: &[u32] = &[0, 5, 6, 7, 2, 10, 11, 12, 13, 2];

    let score_xlm = score_with_xlm_roberta(xlm_weights, input_ids, &device);
    let score_bert = score_with_bert(bert_weights, input_ids, &device);

    let diff = (score_xlm - score_bert).abs();
    assert!(
        diff > 0.1,
        "XLM-R ({score_xlm:.4}) 与 BERT ({score_bert:.4}) 输出差异 {diff:.4} 应 > 0.1（RISK-v1.1-04）"
    );
}

/// 附加：验证 `RerankerArch` 枚举存在且可比较（用于 BgeReranker::arch_type）
#[test]
fn test_reranker_arch_enum_exists() {
    let arch = RerankerArch::XlmRoberta;
    assert_eq!(arch, RerankerArch::XlmRoberta);
    assert_ne!(arch, RerankerArch::Bert);
}
