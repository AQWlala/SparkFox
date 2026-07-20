//! Sub-Step 11.6.1 — HnswIndex 集成测试（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 11.6.1）
//! 验证 [`HnswIndex`](sparkfox_knowledge::index::hnsw_index::HnswIndex) 基于
//! `hnsw_rs`（纯 Rust HNSW 实现）的 6 项核心能力：
//! 1. 新建空索引（len=0, is_empty=true）
//! 2. 单条向量插入（len=1）
//! 3. 批量向量插入（10 条 → len=10）
//! 4. kNN 检索返回 k 个结果
//! 5. kNN 检索结果按距离升序（最近在前）
//! 6. 持久化保存 / 加载（数据完整）
//!
//! ## 测试 fixture
//! - 128 维向量（PoC，实际生产用 768/1024 维）
//! - 10 条正交向量（mock entity_id: `ent-0` 到 `ent-9`）
//!   - `ent-i` → vec[i]=1.0，其余维度 0（正交，cosine 距离 = 1）
//! - 查询向量 = `ent-0` 的向量（应返回自身，距离 ≈ 0）
//!
//! ## 双引擎策略（spec §三 11.6.1）
//! `sparkfox-knowledge::index::HnswIndex` 作为 `sqlite-vec` 的补充引擎，
//! 适用于 >= 1k 向量的快速 kNN 检索场景。本测试使用 `hnsw_rs` 真实 HNSW 实现
//! （区别于 `sparkfox-store::vector_index::hnsw::HnswIndex` 的暴力扫描占位实现）。
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use sparkfox_knowledge::index::hnsw_index::HnswIndex;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// 测试 fixture
// ---------------------------------------------------------------------------

/// 向量维度（PoC 用 128 维，实际生产用 768/1024 维）
const DIM: usize = 128;

/// 测试向量数量
const N_VECTORS: usize = 10;

/// 构造 10 条 128 维正交向量（mock entity_id: `ent-0` 到 `ent-9`）
///
/// `ent-i` → vec[i]=1.0，其余维度 0。两两正交，cosine 距离 = 1（非自身）。
/// 查询 `ent-0` 的向量时，应返回自身（距离=0）+ 其余 9 个（距离=1）。
fn make_orthogonal_vectors() -> Vec<(Vec<f32>, String)> {
    let mut out = Vec::with_capacity(N_VECTORS);
    for i in 0..N_VECTORS {
        let mut v = vec![0.0f32; DIM];
        v[i] = 1.0;
        out.push((v, format!("ent-{}", i)));
    }
    out
}

/// 构造查询向量 = `ent-0` 的向量（vec[0]=1.0，其余 0）
fn make_query_vec() -> Vec<f32> {
    let mut v = vec![0.0f32; DIM];
    v[0] = 1.0;
    v
}

// ---------------------------------------------------------------------------
// 6 个测试（spec §三 11.6.1 验收指标）
// ---------------------------------------------------------------------------

/// 验收指标 1：新建空索引
///
/// `HnswIndex::new(max_elements, dim)` 创建空索引：
/// - `len()` 应返回 0
/// - `is_empty()` 应返回 true
#[test]
fn test_hnsw_index_create_empty() {
    let index = HnswIndex::new(100, DIM);
    assert_eq!(index.len(), 0, "新建索引 len 应为 0");
    assert!(index.is_empty(), "新建索引 is_empty 应为 true");
}

/// 验收指标 2：插入单条向量
///
/// 插入 1 条 128 维向量后，`len()` 应为 1，`is_empty()` 应为 false。
#[test]
fn test_hnsw_index_insert_single() {
    let mut index = HnswIndex::new(100, DIM);
    let vec = make_orthogonal_vectors().into_iter().next().unwrap();
    index
        .insert(&vec.0, &vec.1)
        .expect("insert 单条向量失败");
    assert_eq!(index.len(), 1, "插入 1 条后 len 应为 1");
    assert!(!index.is_empty(), "插入 1 条后 is_empty 应为 false");
}

/// 验收指标 3：批量插入 10 条向量
///
/// `insert_batch` 一次性插入 10 条 128 维向量，`len()` 应为 10。
#[test]
fn test_hnsw_index_insert_batch() {
    let mut index = HnswIndex::new(100, DIM);
    let vectors = make_orthogonal_vectors();
    let batch: Vec<(&[f32], &str)> = vectors
        .iter()
        .map(|(v, id)| (v.as_slice(), id.as_str()))
        .collect();
    index
        .insert_batch(&batch)
        .expect("insert_batch 10 条向量失败");
    assert_eq!(index.len(), 10, "批量插入 10 条后 len 应为 10");
}

/// 验收指标 4：kNN 检索返回 k 个结果
///
/// 插入 10 条向量后，检索 top_k=5，应返回 5 个 `(entity_id, distance)` 元组。
#[test]
fn test_hnsw_index_search_returns_k_results() {
    let mut index = HnswIndex::new(100, DIM);
    let vectors = make_orthogonal_vectors();
    let batch: Vec<(&[f32], &str)> = vectors
        .iter()
        .map(|(v, id)| (v.as_slice(), id.as_str()))
        .collect();
    index.insert_batch(&batch).expect("insert_batch 失败");

    let query = make_query_vec();
    let results = index.search(&query, 5).expect("search top-5 失败");

    assert_eq!(results.len(), 5, "top_k=5 应返回 5 个结果，实际: {:?}", results);
}

/// 验收指标 5：kNN 检索结果按距离升序（最近在前）
///
/// 检索 top_k=10，断言：
/// - 第一个结果为 `ent-0`（自身，距离 ≈ 0）
/// - 结果按 distance 升序排列（前一个 <= 后一个）
#[test]
fn test_hnsw_index_search_returns_nearest_first() {
    let mut index = HnswIndex::new(100, DIM);
    let vectors = make_orthogonal_vectors();
    let batch: Vec<(&[f32], &str)> = vectors
        .iter()
        .map(|(v, id)| (v.as_slice(), id.as_str()))
        .collect();
    index.insert_batch(&batch).expect("insert_batch 失败");

    let query = make_query_vec();
    let results = index.search(&query, 10).expect("search top-10 失败");

    // 结果应非空
    assert!(!results.is_empty(), "结果应非空");

    // 第一个结果应为 ent-0（自身，距离 ≈ 0）
    assert_eq!(
        results[0].0, "ent-0",
        "最近邻应为 ent-0（自身），实际: {:?}",
        results[0]
    );
    // 距离应接近 0（HNSW 浮点误差容差 1e-5）
    assert!(
        results[0].1.abs() < 1e-5,
        "ent-0 自身距离应 ≈ 0，实际: {}",
        results[0].1
    );

    // 结果应按 distance 升序（最近在前）
    for i in 1..results.len() {
        assert!(
            results[i - 1].1 <= results[i].1 + 1e-6,
            "结果应按距离升序，但 results[{}].distance={} > results[{}].distance={}",
            i - 1,
            results[i - 1].1,
            i,
            results[i].1
        );
    }
}

/// 验收指标 6：保存后加载，索引数据完整
///
/// 插入 10 条向量 → save → drop → load → search：
/// - 加载后 `len()` 与原始一致（10）
/// - 检索 top-5 仍返回 5 个结果
/// - 第一个结果仍为 `ent-0`（自身，距离 ≈ 0）
#[test]
fn test_hnsw_index_save_and_load() {
    let tmp = TempDir::new().expect("tempdir 创建失败");
    let path = tmp.path().join("hnsw_index");

    // 写入阶段：插入 10 条向量并 save
    {
        let mut index = HnswIndex::new(100, DIM);
        let vectors = make_orthogonal_vectors();
        let batch: Vec<(&[f32], &str)> = vectors
            .iter()
            .map(|(v, id)| (v.as_slice(), id.as_str()))
            .collect();
        index.insert_batch(&batch).expect("insert_batch 失败");
        assert_eq!(index.len(), 10, "save 前 len 应为 10");
        index.save(&path).expect("save 失败");
    }

    // 加载阶段：从磁盘重新加载
    let loaded = HnswIndex::load(&path).expect("load 失败");
    assert_eq!(loaded.len(), 10, "加载后 len 应为 10");

    // 加载后检索应正常工作
    let query = make_query_vec();
    let results = loaded.search(&query, 5).expect("加载后 search 失败");
    assert_eq!(results.len(), 5, "加载后 top-5 应返回 5 个结果");

    // 第一个结果应为 ent-0（自身，距离 ≈ 0）
    assert_eq!(
        results[0].0, "ent-0",
        "加载后最近邻应为 ent-0（自身），实际: {:?}",
        results[0]
    );
    assert!(
        results[0].1.abs() < 1e-5,
        "加载后 ent-0 自身距离应 ≈ 0，实际: {}",
        results[0].1
    );
}
