//! Sub-Step 11.1.2 — MULTI Step3/Step4 真实实施（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 11.1.2）
//! 1. Step3 实体向量检索返回 Top-K entities（HnswIndex，top_k=3 → 3 个）
//! 2. Step3 验证使用 HnswIndex（thought_process 含 "HnswIndex"）
//! 3. Step4 通过 entities 检索 events（2 个 entity_ids → 2 个 event_ids）
//! 4. Step4 JOIN event_entity_relation（candidates 含预期 event_ids）
//! 5. Step3+Step4 pipeline 返回中间状态（entity_ids 非空 + candidates 非空 + thought_process 含 Step3 和 Step4）
//!
//! ## 循环依赖规避
//! `sparkfox-store` 已依赖 `sparkfox-knowledge`（用于 `ALL_SAG_DDL`），反向依赖会形成循环。
//! 因此 [`multi_step::Step3VectorIndex`] trait 在 `sparkfox-knowledge` 本地定义，
//! 测试中通过 `HnswIndexAdapter` 桥接 `sparkfox_store::vector_index::hnsw::HnswIndex`。
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::multi_step::{
    step3_vector_search_with_index, step4_event_search_with_conn, MultiState, Step3VectorIndex,
};
// 集成测试方可依赖 sparkfox-store（dev-dependencies，不参与 lib 编译，无循环依赖）
use sparkfox_store::vector_index::hnsw::HnswIndex;
use sparkfox_store::vector_index::VectorIndex;

// ---------------------------------------------------------------------------
// HnswIndexAdapter：桥接 sparkfox_store::HnswIndex → Step3VectorIndex
// ---------------------------------------------------------------------------

/// 桥接 `HnswIndex` 到本地 [`Step3VectorIndex`] trait
///
/// 生产代码（`sparkfox-knowledge`）不能依赖 `sparkfox-store`（循环依赖），
/// 故 [`Step3VectorIndex`] trait 在 `sparkfox-knowledge` 本地定义。
/// 测试中通过此 adapter 桥接具体 `HnswIndex` 实现。
struct HnswIndexAdapter<'a> {
    inner: &'a HnswIndex,
}

impl<'a> HnswIndexAdapter<'a> {
    fn new(inner: &'a HnswIndex) -> Self {
        Self { inner }
    }
}

impl<'a> Step3VectorIndex for HnswIndexAdapter<'a> {
    fn search_top_k(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        self.inner
            .search(query, k, None)
            .unwrap_or_default()
            .into_iter()
            .map(|m| (m.id, m.score))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// 测试 fixture
// ---------------------------------------------------------------------------

/// 构造含 5 个 entity 向量的 HnswIndex（384 维，向量正交可区分）
///
/// 插入 5 个 entity（id=`ent-1`..`ent-5`），每个向量在唯一维度上取 1.0：
/// - `ent-1` → vec[0]=1.0
/// - `ent-2` → vec[1]=1.0
/// - `ent-3` → vec[2]=1.0
/// - `ent-4` → vec[3]=1.0
/// - `ent-5` → vec[4]=1.0
///
/// 测试 query 时按维度权重 `[1.0, 0.8, 0.6, 0.4, 0.2, 0, ...]` 可确定性返回
/// `[ent-1, ent-2, ent-3]` 作为 Top-3。
fn setup_hnsw_index_with_5_vectors() -> HnswIndex {
    let index = HnswIndex::new(384).expect("创建 HnswIndex 失败");
    for i in 1..=5u32 {
        let mut vec = vec![0.0f32; 384];
        vec[(i - 1) as usize] = 1.0;
        index
            .insert(&format!("ent-{}", i), &vec)
            .expect("insert 失败");
    }
    index
}

/// 构造确定性 query_vec：与 ent-1..ent-5 的 cosine 相似度依次递减
///
/// 归一化后查询，Top-3 应稳定返回 `[ent-1, ent-2, ent-3]`。
fn make_query_vec_matching_top3() -> Vec<f32> {
    let mut vec = vec![0.0f32; 384];
    vec[0] = 1.0;
    vec[1] = 0.8;
    vec[2] = 0.6;
    vec[3] = 0.4;
    vec[4] = 0.2;
    vec
}

/// 构造含 5 个 entity / 2 个 event / 4 条 event_entity_relation 的 SAG DB
///
/// entity 表（5 条，对应 HnswIndex 的 ent-1..ent-5）：
/// - ent-1: 张三（PERSON）
/// - ent-2: 北京（LOCATION）
/// - ent-3: 腾讯（ORGANIZATION）
/// - ent-4: 上海（LOCATION）
/// - ent-5: 阿里（ORGANIZATION）
///
/// knowledge_event 表（2 条）：
/// - evt-1: 张三去了北京
/// - evt-2: 腾讯在北京
///
/// event_entity_relation 表（4 条，构建 evt-1↔(ent-1,ent-2)、evt-2↔(ent-2,ent-3) 的关联）：
fn setup_sag_db_with_events() -> Connection {
    let conn = Connection::open_in_memory().expect("open_in_memory 失败");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("PRAGMA foreign_keys 失败");
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).expect("DDL 执行失败");
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES)
        .expect("INSERT_DEFAULT_ENTITY_TYPES 失败");

    // 插入 5 个 entity（entity_type_id 引用 INSERT_DEFAULT_ENTITY_TYPES 预填的 11 种默认类型）
    // + 2 个 event（kb_id / doc_id / title / summary / content / created_time / updated_time 均为 NOT NULL）
    // + 4 条 event_entity_relation（id / event_id / entity_id / created_time 均为 NOT NULL）
    //
    // 使用 execute_batch 一次性插入，无需逐条传 params（rusqlite 0.32 execute 必须传 params 参数）
    conn.execute_batch(
        "INSERT INTO entity (id, name, normalized_name, entity_type_id, created_time, updated_time) \
         VALUES ('ent-1', '张三', '张三', 'default_person', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'); \
         INSERT INTO entity (id, name, normalized_name, entity_type_id, created_time, updated_time) \
         VALUES ('ent-2', '北京', '北京', 'default_location', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'); \
         INSERT INTO entity (id, name, normalized_name, entity_type_id, created_time, updated_time) \
         VALUES ('ent-3', '腾讯', '腾讯', 'default_organization', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'); \
         INSERT INTO entity (id, name, normalized_name, entity_type_id, created_time, updated_time) \
         VALUES ('ent-4', '上海', '上海', 'default_location', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'); \
         INSERT INTO entity (id, name, normalized_name, entity_type_id, created_time, updated_time) \
         VALUES ('ent-5', '阿里', '阿里', 'default_organization', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'); \
         INSERT INTO knowledge_event (id, kb_id, doc_id, chunk_id, title, summary, content, created_time, updated_time) \
         VALUES ('evt-1', 'kb-1', 'doc-1', NULL, '张三去了北京', '张三前往北京的行程', '张三于 2026 年 7 月前往北京出差', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'); \
         INSERT INTO knowledge_event (id, kb_id, doc_id, chunk_id, title, summary, content, created_time, updated_time) \
         VALUES ('evt-2', 'kb-1', 'doc-2', NULL, '腾讯在北京', '腾讯北京分公司', '腾讯在北京设有分公司', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'); \
         INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
         VALUES ('rel-1', 'evt-1', 'ent-1', '2026-07-20T00:00:00Z'); \
         INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
         VALUES ('rel-2', 'evt-1', 'ent-2', '2026-07-20T00:00:00Z'); \
         INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
         VALUES ('rel-3', 'evt-2', 'ent-2', '2026-07-20T00:00:00Z'); \
         INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
         VALUES ('rel-4', 'evt-2', 'ent-3', '2026-07-20T00:00:00Z');",
    )
    .expect("insert 业务数据失败");

    conn
}

// ---------------------------------------------------------------------------
// 5 个测试（spec §三 11.1.2 要求）
// ---------------------------------------------------------------------------

/// 验收指标 1：Step3 实体向量检索返回 Top-K entities
///
/// 构造 HnswIndex 插入 5 个向量，检索 top_k=3，应返回 3 个 entity_ids。
#[test]
fn test_step3_entity_vector_search_returns_top_k() {
    let index = setup_hnsw_index_with_5_vectors();
    let adapter = HnswIndexAdapter::new(&index);

    let mut state = MultiState::new("测试 query");
    state.query_vec = make_query_vec_matching_top3();

    let state = step3_vector_search_with_index(state, &adapter, 3);

    assert_eq!(
        state.entity_ids.len(),
        3,
        "top_k=3 应返回 3 个 entity_ids，实际: {:?}",
        state.entity_ids
    );
}

/// 验收指标 2：Step3 使用 HnswIndex
///
/// 断言 entity_ids 非空 + thought_process 含 "HnswIndex"。
#[test]
fn test_step3_uses_hnsw_index() {
    let index = setup_hnsw_index_with_5_vectors();
    let adapter = HnswIndexAdapter::new(&index);

    let mut state = MultiState::new("测试 query");
    state.query_vec = make_query_vec_matching_top3();

    let state = step3_vector_search_with_index(state, &adapter, 3);

    assert!(
        !state.entity_ids.is_empty(),
        "entity_ids 应非空（使用 HnswIndex 检索）"
    );
    assert!(
        state
            .thought_process
            .iter()
            .any(|s| s.contains("HnswIndex")),
        "thought_process 应含 \"HnswIndex\"，实际: {:?}",
        state.thought_process
    );
}

/// 验收指标 3：Step4 通过 entities 检索 events
///
/// 构造 SAG DB（3 entity + 2 event + 4 event_entity_relation），传入 2 个 entity_ids
/// （ent-1, ent-2），应返回 2 个 event_ids（evt-1, evt-2）。
#[test]
fn test_step4_event_retrieval_by_entities() {
    let conn = setup_sag_db_with_events();

    let mut state = MultiState::new("测试 query");
    // 传入 2 个 entity_ids（ent-1, ent-2）
    // ent-1 → evt-1（rel-1）
    // ent-2 → evt-1（rel-2）+ evt-2（rel-3）
    // 去重后 candidates = {evt-1, evt-2}
    state.entity_ids = vec!["ent-1".to_string(), "ent-2".to_string()];

    let state = step4_event_search_with_conn(state, &conn);

    assert_eq!(
        state.candidates.len(),
        2,
        "2 个 entity_ids 应返回 2 个 event_ids（evt-1 + evt-2），实际: {:?}",
        state.candidates
    );
}

/// 验收指标 4：Step4 JOIN event_entity_relation
///
/// 断言 candidates 含预期的 event_ids（evt-1, evt-2）。
#[test]
fn test_step4_joins_event_entity_relation() {
    let conn = setup_sag_db_with_events();

    let mut state = MultiState::new("测试 query");
    // 传入 ent-2 + ent-3：
    // ent-2 → evt-1（rel-2）+ evt-2（rel-3）
    // ent-3 → evt-2（rel-4）
    // 去重后 candidates = {evt-1, evt-2}
    state.entity_ids = vec!["ent-2".to_string(), "ent-3".to_string()];

    let state = step4_event_search_with_conn(state, &conn);

    assert!(
        state.candidates.contains(&"evt-1".to_string()),
        "candidates 应含 evt-1（ent-2 → rel-2 → evt-1），实际: {:?}",
        state.candidates
    );
    assert!(
        state.candidates.contains(&"evt-2".to_string()),
        "candidates 应含 evt-2（ent-2 → rel-3 → evt-2 / ent-3 → rel-4 → evt-2），实际: {:?}",
        state.candidates
    );
}

/// 验收指标 5：Step3+Step4 pipeline 返回中间状态
///
/// 完整跑 Step3 + Step4：entity_ids 非空 + candidates 非空 + thought_process 含 Step3 和 Step4。
#[test]
fn test_step3_step4_pipeline_returns_intermediate_state() {
    let index = setup_hnsw_index_with_5_vectors();
    let adapter = HnswIndexAdapter::new(&index);
    let conn = setup_sag_db_with_events();

    let mut state = MultiState::new("测试 query");
    state.query_vec = make_query_vec_matching_top3();

    // Step3：HnswIndex 检索，返回 Top-3 entity_ids（应为 ent-1, ent-2, ent-3）
    let state = step3_vector_search_with_index(state, &adapter, 3);
    assert!(
        !state.entity_ids.is_empty(),
        "Step3 后 entity_ids 应非空"
    );

    // Step4：JOIN event_entity_relation，返回候选 event_ids
    let state = step4_event_search_with_conn(state, &conn);
    assert!(
        !state.candidates.is_empty(),
        "Step4 后 candidates 应非空（Top-3 entity_ids 中 ent-1/ent-2/ent-3 均有关联 event）"
    );

    // thought_process 应同时含 Step3 和 Step4 记录
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step3")),
        "thought_process 应含 Step3 记录，实际: {:?}",
        state.thought_process
    );
    assert!(
        state.thought_process.iter().any(|s| s.contains("Step4")),
        "thought_process 应含 Step4 记录，实际: {:?}",
        state.thought_process
    );
}
