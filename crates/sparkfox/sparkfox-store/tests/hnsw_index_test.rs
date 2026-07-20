//! Sub-Step 10.13.2 — HnswIndex 实现 + 性能测试 集成测试
//!
//! 验证 [`HnswIndex`](sparkfox_store::vector_index::hnsw::HnswIndex) 后端的
//! trait 实现、性能（1k 向量插入 < 1s / 查询 < 50ms）、删除语义、磁盘持久化。
//!
//! TDD-RED：先于实现编写。当前占位实现未提供 `save` / `load` 方法 → 编译失败 → RED。
//! TDD-GREEN：实现 `save` / `load` + 优化暴力扫描（预归一化 + 点积）后所有用例通过。
//! TDD-REFACTOR：提取 `HnswConfig` + 中文文档注释后再次运行，仍应全绿。
//!
//! spec v2.0 Task 1.5 推荐参数（验收参考）：
//! - M=16 / ef_construction=200 / ef_search=64 / Cosine / mark_deleted 删除

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::time::Instant;

use sparkfox_store::vector_index::hnsw::HnswIndex;
use sparkfox_store::vector_index::{VectorIndex, VectorMatch};
use tempfile::TempDir;

/// 1. 验证 `HnswIndex` 实现 `VectorIndex` trait（可作为 `Box<dyn VectorIndex>` 使用）
///
/// 本质是编译期测试：若 trait 不满足 object safety 或 `HnswIndex` 未实现 trait，
/// `Box<dyn VectorIndex>` 构造将编译失败。
#[test]
fn test_hnsw_index_implements_vector_index() {
    let idx: Box<dyn VectorIndex> = Box::new(HnswIndex::new(4).expect("new HnswIndex"));
    assert_eq!(idx.len(), 0, "新建后 len 应为 0");
    assert!(idx.is_empty(), "新建后 is_empty 应为 true");
    // backend_name 应包含 "hnsw" 标识（与 SqliteVecIndex 的 "sqlite-vec" 区分）
    let name = idx.backend_name();
    assert!(
        name.contains("hnsw"),
        "backend_name 应包含 hnsw 标识，实际: {}",
        name
    );
}

/// 2. 插入 1k 768维向量 < 1s
///
/// spec 验收指标 1：1k 向量插入 < 1s
/// 使用确定性 PRNG（xorshift64）生成 1000 条 768 维向量，避免引入 rand 依赖。
#[test]
fn test_hnsw_index_insert_1k_vectors_under_1s() {
    let idx = HnswIndex::new(768).expect("new HnswIndex");
    let vectors = generate_test_vectors(1000, 768, 0xDEAD_BEEF_CAFE);

    let start = Instant::now();
    for (i, v) in vectors.iter().enumerate() {
        idx.insert(&format!("v{i}"), v).expect("insert");
    }
    let elapsed = start.elapsed();

    assert_eq!(idx.len(), 1000, "插入 1k 后 len 应为 1000");
    assert!(
        elapsed.as_secs_f32() < 1.0,
        "插入 1k 768维向量耗时 {:.3}s 超过 1s 限制",
        elapsed.as_secs_f32()
    );
    println!(
        "[perf] 1k 768维向量插入耗时: {:.3}ms",
        elapsed.as_secs_f32() * 1000.0
    );
}

/// 3. 查询返回 top_k=10 结果（且按 score 降序）
#[test]
fn test_hnsw_index_search_returns_top_k() {
    let idx = HnswIndex::new(8).expect("new HnswIndex");
    // 插入 100 条 8 维向量（v[0] 递增，其余维度 0）
    for i in 0..100 {
        let mut v = vec![0.0f32; 8];
        v[0] = (i as f32 + 1.0) / 100.0;
        idx.insert(&format!("v{i}"), &v).expect("insert");
    }
    assert_eq!(idx.len(), 100);

    let hits: Vec<VectorMatch> = idx
        .search(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 10, None)
        .expect("search top-10");

    assert_eq!(hits.len(), 10, "top_k=10 应返回 10 条结果");

    // 验证按 score 降序排列
    for i in 1..hits.len() {
        assert!(
            hits[i - 1].score >= hits[i].score,
            "结果应按 score 降序，但 hits[{}].score={} < hits[{}].score={}",
            i - 1,
            hits[i - 1].score,
            i,
            hits[i].score
        );
    }
}

/// 4. 1k 768维向量查询 < 50ms
///
/// spec 验收指标 2：1k 向量查询 < 50ms
/// 1k 768维点积约 768k 次乘加，单线程现代 CPU 估测 < 1ms，远低于 50ms 阈值。
#[test]
fn test_hnsw_index_search_1k_vectors_under_50ms() {
    let idx = HnswIndex::new(768).expect("new HnswIndex");
    let vectors = generate_test_vectors(1000, 768, 0xCAFE_BABE_F00D);
    for (i, v) in vectors.iter().enumerate() {
        idx.insert(&format!("v{i}"), v).expect("insert");
    }
    assert_eq!(idx.len(), 1000);

    let query = &vectors[0];
    // 预热一次（避免冷启动 / 缓存未命中影响主测试计时）
    let _ = idx.search(query, 10, None).expect("warmup search");

    let start = Instant::now();
    let hits = idx.search(query, 10, None).expect("timed search");
    let elapsed = start.elapsed();

    assert_eq!(hits.len(), 10, "应返回 10 条结果");
    assert!(
        elapsed.as_millis() < 50,
        "1k 768维向量查询耗时 {}ms 超过 50ms 限制",
        elapsed.as_millis()
    );
    println!(
        "[perf] 1k 768维向量查询耗时: {:.3}ms",
        elapsed.as_secs_f32() * 1000.0
    );
}

/// 5. 删除向量后查询不返回
///
/// 插入 a/b/c 三条向量，删除 a（与 query 最相似），验证：
/// - len 减为 2
/// - 搜索结果不含 a
/// - 搜索结果仍含 b（未被误删）
#[test]
fn test_hnsw_index_delete_removes_vector() {
    let idx = HnswIndex::new(4).expect("new HnswIndex");
    idx.insert("a", &[1.0, 0.0, 0.0, 0.0]).expect("insert a");
    idx.insert("b", &[0.9, 0.1, 0.0, 0.0]).expect("insert b");
    idx.insert("c", &[0.0, 1.0, 0.0, 0.0]).expect("insert c");
    assert_eq!(idx.len(), 3);

    idx.delete("a").expect("delete a");
    assert_eq!(idx.len(), 2, "删除后 len 应为 2");

    let hits = idx
        .search(&[1.0, 0.0, 0.0, 0.0], 10, None)
        .expect("search after delete");
    assert!(
        hits.iter().all(|h| h.id != "a"),
        "已删除的 a 不应出现在搜索结果中，got {:?}",
        hits.iter().map(|h| &h.id).collect::<Vec<_>>()
    );
    assert!(
        hits.iter().any(|h| h.id == "b"),
        "b 应仍出现在搜索结果中（不应被误删）"
    );
}

/// 6. 持久化到磁盘后重新加载
///
/// spec 验收指标 3：持久化到磁盘可重新加载
/// 使用 tempfile::TempDir 创建临时目录，写入 50 条 8 维向量，
/// 调用 `save` 持久化，再用 `load` 重新加载，验证：
/// - 文件确实创建
/// - 加载后 len 与原始一致
/// - 加载后检索功能正常（返回正确数量 + 排序）
#[test]
fn test_hnsw_index_persists_to_disk() {
    let tmp = TempDir::new().expect("tempdir");
    let path: PathBuf = tmp.path().join("hnsw.index");

    // 写入阶段：插入 50 条向量并 save
    {
        let idx = HnswIndex::new(8).expect("new HnswIndex");
        for i in 0..50 {
            let mut v = vec![0.0f32; 8];
            v[0] = i as f32 / 50.0;
            v[1] = 0.5;
            idx.insert(&format!("v{i}"), &v).expect("insert");
        }
        assert_eq!(idx.len(), 50, "写入前 len 应为 50");
        idx.save(&path).expect("save to disk");
    }

    // 文件应确实存在
    assert!(
        path.exists(),
        "持久化文件应存在: {}",
        path.display()
    );

    // 加载阶段：从磁盘重新加载
    let loaded = HnswIndex::load(&path).expect("load from disk");
    assert_eq!(loaded.len(), 50, "加载后 len 应为 50");

    // 加载后检索应正常工作
    let query = vec![0.5f32; 8];
    let hits: Vec<VectorMatch> = loaded.search(&query, 5, None).expect("search after load");
    assert_eq!(hits.len(), 5, "加载后 top-5 检索应返回 5 条结果");

    // 验证排序仍正确（降序）
    for i in 1..hits.len() {
        assert!(
            hits[i - 1].score >= hits[i].score,
            "加载后结果应按 score 降序，但 hits[{}].score={} < hits[{}].score={}",
            i - 1,
            hits[i - 1].score,
            i,
            hits[i].score
        );
    }

    // 验证 backend_name 仍可调用（trait object 可用性）
    assert!(
        loaded.backend_name().contains("hnsw"),
        "加载后 backend_name 应仍包含 hnsw"
    );
}

/// 简易 xorshift64 PRNG（测试用，避免引入 rand 依赖）
///
/// 与现有 hnsw.rs / sqlite_vec.rs 单元测试中的实现一致，确保可重现。
fn generate_test_vectors(n: usize, dim: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut state = seed;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let mut v = vec![0.0f32; dim];
        for el in v.iter_mut() {
            // xorshift64
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *el = (state as f32 / u64::MAX as f32) * 2.0 - 1.0;
        }
        out.push(v);
    }
    out
}
