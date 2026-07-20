//! Sub-Step 10.13.1 — VectorIndex trait 抽象 集成测试
//!
//! 验证 [`VectorIndex`] trait 可作为 trait object 使用，并验证
//! [`SqliteVecIndex`](sparkfox_store::vector_index::sqlite_vec::SqliteVecIndex)
//! 后端的 insert / search / delete / len / backend_name 行为，
//! 以及 [`auto_select`](sparkfox_store::vector_index::auto_select) 函数的规模分支选择。
//!
//! TDD-RED：先于实现编写（实际 v1.0.0 已有实现，本测试集用于 v1.1.0 Sub-Step 10.13.1 验收）。
//! TDD-GREEN：所有用例应一次性通过。
//! TDD-REFACTOR：清理后再次运行本文件，仍应全绿。

#![forbid(unsafe_code)]

use sparkfox_store::vector_index::{
    auto_select, sqlite_vec::SqliteVecIndex, VectorIndex, VectorMatch,
};

/// 1. 验证 `Box<dyn VectorIndex>` 可作为 trait object 编译通过。
///
/// 这本质上是一个编译期测试：如果 trait 不满足 object safety（例如含泛型方法），
/// `Box<dyn VectorIndex>` 将无法构造，编译失败。
#[test]
fn test_vector_index_trait_can_be_object() {
    let idx: Box<dyn VectorIndex> = Box::new(SqliteVecIndex::new(4).expect("new SqliteVecIndex"));
    // 调用一个方法以确保 trait object 真正可用（不只是编译通过）
    assert_eq!(idx.len(), 0);
    assert!(idx.is_empty());
    assert_eq!(idx.backend_name(), "sqlite-vec");
}

/// 2. 插入 10 个向量，搜索 top-5，验证返回结果数量与排序。
///
/// 设计 10 条 4 维向量，确保 cosine 严格递减（避免 scale-invariance 导致的并列）：
/// - v0=[1,0,0,0]         → cosine = 1.0（与 query 完全同向）
/// - v1=[0.9,0.1,0,0]     → cosine ≈ 0.9939
/// - v2=[0.8,0,0.2,0]     → cosine ≈ 0.9701
/// - v3=[0.7,0,0,0.3]     → cosine ≈ 0.9189
/// - v4=[0.6,0.4,0,0]     → cosine ≈ 0.8321
/// - v5..v9 与 query 正交   → cosine = 0
///
/// 注意：cosine 相似度具有 scale-invariance，[0.96,0,0,0] 与 [1,0,0,0] 的 cosine 同为 1.0，
/// 故 v4 必须在非首维有非零分量，才能保证 cosine 严格小于 v0。
#[test]
fn test_sqlite_vec_insert_and_search() {
    let idx = SqliteVecIndex::new(4).expect("new SqliteVecIndex");

    // 前 5 条：与 query=[1,0,0,0] cosine 严格递减
    idx.insert("v0", &[1.0, 0.0, 0.0, 0.0]).expect("insert v0");
    idx.insert("v1", &[0.9, 0.1, 0.0, 0.0]).expect("insert v1");
    idx.insert("v2", &[0.8, 0.0, 0.2, 0.0]).expect("insert v2");
    idx.insert("v3", &[0.7, 0.0, 0.0, 0.3]).expect("insert v3");
    idx.insert("v4", &[0.6, 0.4, 0.0, 0.0]).expect("insert v4");
    // 后 5 条：与 query 正交（cosine = 0）
    idx.insert("v5", &[0.0, 1.0, 0.0, 0.0]).expect("insert v5");
    idx.insert("v6", &[0.0, 0.0, 1.0, 0.0]).expect("insert v6");
    idx.insert("v7", &[0.0, 0.0, 0.0, 1.0]).expect("insert v7");
    idx.insert("v8", &[0.0, 1.0, 1.0, 0.0]).expect("insert v8");
    idx.insert("v9", &[0.0, 0.0, 1.0, 1.0]).expect("insert v9");

    assert_eq!(idx.len(), 10, "插入 10 条后 len 应为 10");

    let hits: Vec<VectorMatch> = idx.search(&[1.0, 0.0, 0.0, 0.0], 5, None).expect("search top-5");
    assert_eq!(hits.len(), 5, "top-5 应返回 5 条结果");

    // 前 5 条 cosine 严格大于后 5 条，应全部命中
    let hit_ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
    for expected in ["v0", "v1", "v2", "v3", "v4"] {
        assert!(
            hit_ids.contains(&expected),
            "expected {expected} in top-5 hits, got {:?}",
            hit_ids
        );
    }

    // v0 cosine=1.0 严格最大，应排在首位
    assert_eq!(hits[0].id, "v0", "v0 cosine=1.0 应排在首位");
    assert!(
        hits[0].score > 0.99,
        "v0 score 应接近 1.0，实际 {}",
        hits[0].score
    );

    // 验证排序严格递减（cosine 越大越靠前）
    for i in 1..hits.len() {
        assert!(
            hits[i - 1].score >= hits[i].score,
            "hits 应按 cosine 降序排列，但 hits[{}].score={} < hits[{}].score={}",
            i - 1,
            hits[i - 1].score,
            i,
            hits[i].score
        );
    }

    // 后 5 条不应出现在 top-5 中（cosine = 0 排在末尾被截断）
    for excluded in ["v5", "v6", "v7", "v8", "v9"] {
        assert!(
            !hit_ids.contains(&excluded),
            "{excluded} cosine=0 不应出现在 top-5 中，got {:?}",
            hit_ids
        );
    }
}

/// 3. 插入后删除，验证搜索结果不再包含被删除的 id。
#[test]
fn test_sqlite_vec_delete() {
    let idx = SqliteVecIndex::new(3).expect("new SqliteVecIndex");
    idx.insert("a", &[1.0, 0.0, 0.0]).expect("insert a");
    idx.insert("b", &[0.9, 0.1, 0.0]).expect("insert b");
    idx.insert("c", &[0.0, 1.0, 0.0]).expect("insert c");
    assert_eq!(idx.len(), 3);

    // 删除 a（与 query=[1,0,0] 最相似的那条）
    idx.delete("a").expect("delete a");
    assert_eq!(idx.len(), 2, "删除后 len 应为 2");

    // 搜索 query=[1,0,0]，a 不应出现
    let hits = idx.search(&[1.0, 0.0, 0.0], 10, None).expect("search after delete");
    assert!(
        hits.iter().all(|h| h.id != "a"),
        "已删除的 a 不应出现在搜索结果中，got {:?}",
        hits.iter().map(|h| &h.id).collect::<Vec<_>>()
    );
    // b 仍应存在
    assert!(
        hits.iter().any(|h| h.id == "b"),
        "b 应仍出现在搜索结果中"
    );
}

/// 4. 插入 5 个向量，验证 `len()` 返回 5。
#[test]
fn test_sqlite_vec_len() {
    let idx = SqliteVecIndex::new(2).expect("new SqliteVecIndex");
    assert_eq!(idx.len(), 0, "新建后 len 应为 0");
    assert!(idx.is_empty(), "新建后 is_empty 应为 true");

    for i in 0..5 {
        let id = format!("v{i}");
        idx.insert(&id, &[i as f32, (i + 1) as f32]).expect("insert");
    }
    assert_eq!(idx.len(), 5, "插入 5 条后 len 应为 5");
    assert!(!idx.is_empty(), "插入后 is_empty 应为 false");

    // 重复插入（同 id）应为 upsert 语义，len 不增加
    idx.insert("v0", &[100.0, 200.0]).expect("upsert v0");
    assert_eq!(idx.len(), 5, "upsert 后 len 仍应为 5");
}

/// 5. 验证 `backend_name()` 返回 `"sqlite-vec"`。
#[test]
fn test_vector_index_backend_name() {
    let idx = SqliteVecIndex::new(8).expect("new SqliteVecIndex");
    assert_eq!(idx.backend_name(), "sqlite-vec");

    // 通过 trait object 也应返回相同值
    let boxed: Box<dyn VectorIndex> = Box::new(idx);
    assert_eq!(boxed.backend_name(), "sqlite-vec");
}

/// 6. 验证 `auto_select(500, 768)` 返回 sqlite-vec 后端（500 < 1000 阈值）。
#[test]
fn test_auto_select_small_size_returns_sqlite_vec() {
    let backend = auto_select(500, 768).expect("auto_select(500, 768)");
    assert_eq!(
        backend.backend_name(),
        "sqlite-vec",
        "size=500 < 1000 阈值，应选 sqlite-vec 后端"
    );

    // 边界：999 仍应为 sqlite-vec
    let boundary = auto_select(999, 768).expect("auto_select(999, 768)");
    assert_eq!(
        boundary.backend_name(),
        "sqlite-vec",
        "size=999 仍 < 1000，应为 sqlite-vec"
    );

    // 确保返回的是可用的 trait object（可调用方法）
    assert_eq!(backend.len(), 0);
    assert!(backend.is_empty());
}
