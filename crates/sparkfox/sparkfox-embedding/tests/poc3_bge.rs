//! PoC-3 bge Rust 推理性 + 与 Python 一致性（严格 cosine > 0.99）
//!
//! 【运行前提】
//! 1. 模型已下载：首次运行 `BgeEmbedder::load` 会自动通过 hf-hub 下载 bge-small-zh-v1.5（120MB）
//! 2. Python baseline 已生成：需要 Python 环境 + sentence-transformers
//!    ```bash
//!    pip install sentence-transformers
//!    python -c "
//!    from sentence_transformers import SentenceTransformer
//!    import json
//!    model = SentenceTransformer('BAAI/bge-small-zh-v1.5')
//!    texts = ['测试文本 ' + str(i) for i in range(100)]
//!    vecs = model.encode(texts).tolist()
//!    data = list(zip(texts, vecs))
//!    json.dump(data, open('tests/baseline_small.json', 'w'))
//!    "
//!    ```
//!
//! 【运行方式】
//! ```bash
//! cargo test -p sparkfox-embedding --test poc3_bge -- --ignored --nocapture
//! ```
//!
//! 【验收门槛】（用户决策 B：严格 cosine > 0.99）
//! - 单条嵌入 < 50ms
//! - 1000 条批量 < 30s
//! - 与 Python sentence-transformers cosine > 0.99（每条）

#![forbid(unsafe_code)]

use std::time::Instant;

use sparkfox_embedding::{find_local_model_dir, BgeEmbedder, ModelVariant};

/// 诊断测试（不忽略）：打印当前 cwd 和 find_local_model_dir 结果
#[test]
fn diag_local_model_dir() {
    let cwd = std::env::current_dir().unwrap();
    let sparkfox_models_dir = std::env::var("SPARKFOX_MODELS_DIR").unwrap_or_default();
    eprintln!("[diag] cwd = {}", cwd.display());
    eprintln!("[diag] SPARKFOX_MODELS_DIR = {sparkfox_models_dir}");
    let found = find_local_model_dir(&ModelVariant::BgeSmallZh);
    eprintln!("[diag] find_local_model_dir = {:?}", found);
}

/// PoC-3.1: 单条嵌入延迟 < 50ms（预热后）
#[test]
#[ignore]
fn poc3_single_embedding_under_50ms() {
    let embedder = BgeEmbedder::load(ModelVariant::BgeSmallZh).expect("加载模型");
    // 预热（首次 forward 会初始化内部缓存）
    let _ = embedder.embed("预热");

    let t = Instant::now();
    let v = embedder.embed("你好世界，这是一个测试").expect("嵌入");
    let elapsed = t.elapsed();

    assert_eq!(v.len(), 512, "bge-small-zh 维度应为 512");
    assert!(
        elapsed.as_millis() < 50,
        "单条嵌入 {elapsed:?} 超过 50ms 门槛"
    );
    eprintln!("✅ PoC-3.1 单条嵌入: {}ms", elapsed.as_millis());
}

/// PoC-3.2: 1000 条批量嵌入 < 30s
#[test]
#[ignore]
fn poc3_batch_1000_under_30s() {
    let embedder = BgeEmbedder::load(ModelVariant::BgeSmallZh).expect("加载模型");
    let texts: Vec<String> = (0..1000).map(|i| format!("测试文本 {i}")).collect();
    let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

    let t = Instant::now();
    let vecs = embedder.embed_batch(&refs).expect("批量嵌入");
    let elapsed = t.elapsed();

    assert_eq!(vecs.len(), 1000);
    assert_eq!(vecs[0].len(), 512);
    assert!(
        elapsed.as_secs() < 30,
        "1000 条批量 {elapsed:?} 超过 30s 门槛"
    );
    eprintln!("✅ PoC-3.2 1000 条批量: {:.2}s", elapsed.as_secs_f64());
}

/// PoC-3.3: 与 Python sentence-transformers cosine > 0.99（严格门槛）
///
/// 需要 `tests/baseline_small.json` 已生成（见文件头说明）。
#[test]
#[ignore]
fn poc3_consistency_with_python_strict_099() {
    let baseline_path = "tests/baseline_small.json";
    let baseline = load_python_baseline(baseline_path);

    let embedder = BgeEmbedder::load(ModelVariant::BgeSmallZh).expect("加载模型");

    let mut min_cosine = 1.0f32;
    let mut max_cosine = -1.0f32;
    let sample_size = baseline.len().min(100);

    for (text, expected) in baseline.iter().take(sample_size) {
        let actual = embedder.embed(text).expect("嵌入");
        let cos = cosine_similarity(&actual, expected);
        assert!(
            cos > 0.99,
            "cosine {cos:.6} < 0.99 (text: {text})\n实际向量前5: {:?}\n期望向量前5: {:?}",
            &actual[..5.min(actual.len())],
            &expected[..5.min(expected.len())]
        );
        min_cosine = min_cosine.min(cos);
        max_cosine = max_cosine.max(cos);
    }

    eprintln!(
        "✅ PoC-3.3 一致性: 样本 {sample_size}, min cosine = {min_cosine:.6}, max cosine = {max_cosine:.6}"
    );
}

/// PoC-3.4: 归一化验证 — 每条嵌入 L2 norm ≈ 1.0
#[test]
#[ignore]
fn poc3_embeddings_are_l2_normalized() {
    let embedder = BgeEmbedder::load(ModelVariant::BgeSmallZh).expect("加载模型");
    let samples = ["你好", "这是一个测试", "SparkFox 知识库 RAG 引擎", "你好世界"];

    for text in &samples {
        let v = embedder.embed(text).expect("嵌入");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-4,
            "L2 norm = {norm:.6}，应为 1.0（text: {text}）"
        );
    }
    eprintln!("✅ PoC-3.4 归一化: 所有样本 L2 norm ≈ 1.0");
}

// ---------- 辅助函数 ----------

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

fn load_python_baseline(path: &str) -> Vec<(String, Vec<f32>)> {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("读取 baseline 失败 {path}: {e}\n请先运行 Python 生成 baseline（见文件头说明）"));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("解析 baseline 失败: {e}"))
}
