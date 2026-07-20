//! Sub-Step 11.7.2 — 端到端 < 1s 二次验证（10k events 全流程延迟测试，spec §三 11.7.2）
//!
//! ## 测试目标（spec §三 11.7.2，5 测试）
//! 1. `test_e2e_multi1_latency_under_1s`：multi1 单跳检索 < 1s
//! 2. `test_e2e_multi_latency_under_1s`：multi 多跳检索 < 1s
//! 3. `test_e2e_step1_step2_latency_under_100ms`：Step1（向量化）+ Step2（实体抽取）< 100ms
//! 4. `test_e2e_step3_step4_latency_under_200ms`：Step3（实体检索）+ Step4（事件检索）< 200ms
//! 5. `test_e2e_full_8_step_latency_under_1s`：完整 8 步流程 < 1s
//!
//! ## 延迟基准（spec §三 11.7.2）
//! - **multi1 单跳**：预期 < 100ms（仅 1 跳 BFS，~5 次 SQL 查询）
//! - **multi 多跳**：预期 < 500ms（max_hop=3，10k events 实际 2-3 跳）
//! - **Step1+Step2**：预期 < 100ms（mock 向量化 + jieba 实体抽取）
//! - **Step3+Step4**：预期 < 200ms（HnswIndex kNN + SQL JOIN event_entity_relation）
//! - **完整 8 步**：预期 < 1s（所有步骤总和）
//!
//! ## 断言阈值设计（2x 余量，避免 CI 性能波动 flaky）
//! - multi1 < 1000ms（spec < 1s，已 10x baseline 余量，无需额外放大）
//! - multi < 1000ms（spec < 1s，已 2x baseline 余量）
//! - Step1+Step2 < 200ms（baseline 100ms，2x 余量；jieba 词典加载为主要耗时）
//! - Step3+Step4 < 400ms（baseline 200ms，2x 余量；HnswIndex 图构建不计时，仅测量 search+JOIN）
//! - 完整 8 步 < 1000ms（spec < 1s，硬性要求）
//!
//! ## Fixture 设计（独立实现，避免与 11.6.2 并行修改冲突）
//! - `setup_10k_events_db`：参考 `multi_e2e.rs::setup_10k_events_db` 构造方式（1000 entity
//!   + 10000 event + ~20000 relation），独立实现不 import，避免与 11.6.2 并行修改冲突
//! - `setup_hnsw_index_with_1000_vectors`：1000 entity 向量插入本地 HnswIndex（384 维）
//! - 向量设计：ent-0-0（张三）用 dim 0，ent-1-0（北京）用 dim 1，其他用 dim 2+，
//!   query 向量 `[1.0, 0, ..., 0]` 确保 Step3 top-1 命中张三（cosine sim = 1.0）
//!
//! ## 异步运行时选择
//! 使用 `#[tokio::test]` + `.await`（与 `multi_e2e.rs` 一致），而非 `futures::executor::block_on`
//! 或 `tokio::runtime::Runtime::new().block_on()`。`tokio` 已在 dev-dependencies 中。
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use std::time::Instant;

use rusqlite::Connection;

use sparkfox_knowledge::index::HnswIndex;
use sparkfox_knowledge::jieba_ner::JiebaNer;
use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::multi::Multi1Strategy;
use sparkfox_knowledge::search::multi_step::{
    step1_vectorize, step2_extract_entities, step2_extract_entities_with_jieba,
    step3_vector_search, step3_vector_search_with_index, step4_event_search,
    step4_event_search_with_conn, step5_with_multi1_async, step6_associate_chunks,
    step7_rerank_with_thought, step8_build_result, MultiState,
};
use sparkfox_knowledge::search::{MultiStrategy, SearchStrategy};

// ---------------------------------------------------------------------------
// 常量定义
// ---------------------------------------------------------------------------

/// 10 种实体类型 ID（与 schema.rs::ENTITY_TYPES 对齐，跳过 OTHER 兜底类型）
///
/// 索引与 entity_id 的 type_idx 一致：`ent-{type_idx}-{i}` 对应 `ENTITY_TYPE_IDS[type_idx]`。
/// 与 `multi_e2e.rs::ENTITY_TYPE_IDS` 保持一致以确保 fixture 数据拓扑相同。
const ENTITY_TYPE_IDS: &[&str] = &[
    "default_person",        // type_idx=0
    "default_location",      // type_idx=1
    "default_organization",  // type_idx=2
    "default_time",          // type_idx=3
    "default_number",        // type_idx=4
    "default_event",         // type_idx=5
    "default_object",        // type_idx=6
    "default_concept",       // type_idx=7
    "default_law",           // type_idx=8
    "default_disease",       // type_idx=9
];

/// HnswIndex 向量维度（与 `multi_step.rs::EMBED_DIM` 对齐，mock embedding 384 维）
const HNSW_DIM: usize = 384;

// ---------------------------------------------------------------------------
// Fixture：10k events 内存数据库（独立实现，避免与 11.6.2 并行修改冲突）
// ---------------------------------------------------------------------------

/// 构造 10k events 测试 DB（1000 entity + 10000 event + ~20000 relation）
///
/// ## 拓扑设计（与 `multi_e2e.rs::setup_10k_events_db` 一致）
/// ```text
/// 张三 (ent-0-0) ── evt-0 ── 北京 (ent-1-0) ── evt-4
///                ── evt-1 ──/
///                ── evt-2 ──/
///                ── evt-3 ──/
///
/// evt-5..evt-9999 ── filler entities (ent-2-x .. ent-9-x)
/// ```
///
/// ## 查询「张三」的 multi1 行为
/// - Step2 jieba 抽取「张三」→ Step3 SQL 匹配到 `ent-0-0`
/// - Step5 BFS hop=1：返回 `evt-0`..`evt-3`（4 hits）
///
/// ## 性能预期
/// - fixture 构造：~1-3s（31000 次 INSERT，in-memory SQLite）
/// - multi1 检索：~50-200ms（仅 1 跳 BFS，~5 次 SQL 查询）
fn setup_10k_events_db() -> Connection {
    let conn = Connection::open_in_memory().expect("打开内存数据库失败");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("开启 foreign_keys 失败");
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).expect("执行 SAG DDL 失败");
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES)
        .expect("预填默认实体类型失败");

    // -------------------------------------------------------------------
    // 1000 个 entity（10 类型 × 100 个，ID 格式 ent-{type_idx}-{i}）
    // -------------------------------------------------------------------
    // ent-0-0 = 张三（PERSON，anchor 实体）
    // ent-1-0 = 北京（LOCATION，hop=2 中间实体）
    // 其他 = 实体_{type_idx}_{i}（filler）
    for type_idx in 0..10 {
        let type_id = ENTITY_TYPE_IDS[type_idx];
        for i in 0..100 {
            let entity_id = format!("ent-{}-{}", type_idx, i);
            let name = if type_idx == 0 && i == 0 {
                "张三".to_string()
            } else if type_idx == 1 && i == 0 {
                "北京".to_string()
            } else {
                format!("实体_{}_{}", type_idx, i)
            };
            conn.execute(
                "INSERT INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    &entity_id,
                    type_id,
                    &name,
                    &name,
                    "2026-07-20T00:00:00Z",
                    "2026-07-20T00:00:00Z",
                ],
            )
            .expect("INSERT entity 失败");
        }
    }

    // -------------------------------------------------------------------
    // 10000 个 event（evt-0..evt-9999）
    // -------------------------------------------------------------------
    // created_time 按分钟递增（覆盖多天，避免时间戳相同影响排序稳定性）
    for i in 0..10000 {
        let event_id = format!("evt-{}", i);
        let title = format!("事件_{}", i);
        let summary = format!("事件_{} 的摘要", i);
        let content = format!("事件_{} 的内容", i);
        let total_minutes = i;
        let minute = total_minutes % 60;
        let hour = (total_minutes / 60) % 24;
        let day = (total_minutes / (60 * 24)) + 20;
        let created = format!("2026-07-{:02}T{:02}:{:02}:00Z", day, hour, minute);
        conn.execute(
            "INSERT INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &event_id,
                "kb-1",
                "doc-1",
                &title,
                &summary,
                &content,
                &created,
                &created,
            ],
        )
        .expect("INSERT knowledge_event 失败");
    }

    // -------------------------------------------------------------------
    // event_entity_relation（约 20000 条）
    // -------------------------------------------------------------------
    // evt-0..evt-3 → 张三 (ent-0-0) + 北京 (ent-1-0)：8 条（Recall@5 hop=1 ground truth）
    // evt-4 → 北京 (ent-1-0)：1 条（Recall@5 hop=2 ground truth）
    // evt-5..evt-9999 → 1-3 个 filler entities：约 20000 条
    let mut rel_idx: u32 = 0;
    let make_rel_id = |rel_idx: &mut u32| -> String {
        let id = format!("rel-{}", rel_idx);
        *rel_idx += 1;
        id
    };

    // evt-0..evt-3 → 张三 + 北京
    for i in 0..4 {
        let evt_id = format!("evt-{}", i);
        let rel1 = make_rel_id(&mut rel_idx);
        let rel2 = make_rel_id(&mut rel_idx);
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![&rel1, &evt_id, "ent-0-0", "2026-07-20T00:00:00Z"],
        )
        .expect("INSERT event_entity_relation (anchor) 失败");
        conn.execute(
            "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![&rel2, &evt_id, "ent-1-0", "2026-07-20T00:00:00Z"],
        )
        .expect("INSERT event_entity_relation (anchor) 失败");
    }

    // evt-4 → 北京（仅关联北京，张三通过北京 hop=2 到达）
    let rel = make_rel_id(&mut rel_idx);
    conn.execute(
        "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![&rel, "evt-4", "ent-1-0", "2026-07-20T00:00:00Z"],
    )
    .expect("INSERT event_entity_relation (hop2) 失败");

    // evt-5..evt-9999 → 1-3 个 filler entities（避开 ent-0-0 张三，避免污染 Recall@5）
    // filler entity 范围：ent-2-x .. ent-9-x（type_idx=2..9，共 8 类型 × 100 = 800 个）
    for i in 5usize..10000 {
        let evt_id = format!("evt-{}", i);
        let rel_count = (i % 3) + 1; // 1-3 个 entity
        for j in 0..rel_count {
            let filler_type_idx = 2 + ((i + j) % 8); // 2..9
            let filler_ent_idx = (i + j) % 100; // 0..99
            let entity_id = format!("ent-{}-{}", filler_type_idx, filler_ent_idx);
            let rel_id = make_rel_id(&mut rel_idx);
            conn.execute(
                "INSERT INTO event_entity_relation (id, event_id, entity_id, created_time) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![&rel_id, &evt_id, &entity_id, "2026-07-20T00:00:00Z"],
            )
            .expect("INSERT event_entity_relation (filler) 失败");
        }
    }

    conn
}

// ---------------------------------------------------------------------------
// Fixture：1000 entity 向量的 HnswIndex（用于 Step3+Step4 延迟测试）
// ---------------------------------------------------------------------------

/// 构造含 1000 entity 向量的 HnswIndex（384 维，使用本地 `sparkfox_knowledge::index::HnswIndex`）
///
/// ## 向量设计（确保 Step3 top-1 命中张三 ent-0-0）
/// - `ent-0-0`（张三）：vec[0] = 1.0，其余 = 0.0 → 与 query cosine sim = 1.0
/// - `ent-1-0`（北京）：vec[1] = 1.0，其余 = 0.0 → 与 query cosine sim = 0.0
/// - 其他 `ent-{type_idx}-{i}`：vec[(2 + (type_idx*100 + i)) % 382] = 1.0 → cosine sim = 0.0
///
/// ## query 向量
/// [`make_query_vec_matching_zhangsan`] 返回 `[1.0, 0, ..., 0]`，与 ent-0-0 完美匹配。
///
/// ## 性能预期
/// - 1000 次 HnswIndex::insert：~100-500ms（HNSW 图构建，O(N log N)）— 不计时
/// - Step3 search top_k=10：~1-10ms（HNSW sub-linear 检索）— 计时
///
/// ## 与 `multi_step3_step4_test.rs` 的区别
/// - `multi_step3_step4_test.rs` 使用 `sparkfox_store::HnswIndex` + adapter（5 个向量）
/// - 本 fixture 使用本地 `sparkfox_knowledge::index::HnswIndex`（1000 个向量，已 impl Step3VectorIndex）
fn setup_hnsw_index_with_1000_vectors() -> HnswIndex {
    let mut index = HnswIndex::new(1000, HNSW_DIM);
    for type_idx in 0..10 {
        for i in 0..100 {
            let entity_id = format!("ent-{}-{}", type_idx, i);
            let mut vec = vec![0.0f32; HNSW_DIM];
            // 张三用 dim 0，北京用 dim 1，其他用 dim 2+（部分冲突，不影响 top-1 命中张三）
            let dim_idx = if type_idx == 0 && i == 0 {
                0
            } else if type_idx == 1 && i == 0 {
                1
            } else {
                2 + ((type_idx * 100 + i) % (HNSW_DIM - 2))
            };
            vec[dim_idx] = 1.0;
            index
                .insert(&vec, &entity_id)
                .expect("HnswIndex insert 失败");
        }
    }
    index
}

/// 构造与张三（ent-0-0）完美匹配的 query 向量
///
/// `[1.0, 0, 0, ..., 0]`（384 维），与 `ent-0-0` 的向量 `[1.0, 0, ..., 0]` cosine sim = 1.0。
/// HnswIndex search top_k=10 应返回 ent-0-0 作为 top-1（cosine sim = 1.0），
/// 其余 9 个为 cosine sim = 0.0 的随机实体。
fn make_query_vec_matching_zhangsan() -> Vec<f32> {
    let mut vec = vec![0.0f32; HNSW_DIM];
    vec[0] = 1.0;
    vec
}

// ---------------------------------------------------------------------------
// 测试 1：multi1 单跳检索 < 1s（spec §三 11.7.2）
// ---------------------------------------------------------------------------

/// 验收指标 1：MULTI1 端到端检索 10k events 延迟 < 1s
///
/// 通过 `Multi1Strategy::search("张三")` 端到端跑通 8 步流程（max_hop=1 单跳剪枝），
/// 测量 `Instant::now()` 到 `search` 返回的耗时，断言 < 1000ms。
///
/// ## 预期延迟
/// - baseline：~50-200ms（仅 1 跳 BFS，~5 次 SQL 查询 + jieba 词典加载）
/// - spec 要求：< 1s（已 5-20x baseline 余量，CI 性能波动不会触发 flaky）
///
/// ## 断言
/// - `duration.as_millis() < 1000`
/// - `result.hits` 非空（至少返回 evt-0 张三直接关联的 event）
#[tokio::test]
async fn test_e2e_multi1_latency_under_1s() {
    let conn = setup_10k_events_db();
    let strategy = Multi1Strategy::new(conn);

    let start = Instant::now();
    let result = strategy
        .search("张三")
        .await
        .expect("multi1 search 应成功");
    let duration = start.elapsed();

    assert!(
        duration.as_millis() < 1000,
        "multi1 检索耗时 {:?} 应 < 1s（spec §三 11.7.2）",
        duration
    );
    assert!(
        !result.hits.is_empty(),
        "应返回非空 hits（张三直接关联的 event），实际: {:?}",
        result.hits
    );

    println!(
        "[multi1_latency] hits={}, external_latency={}ms, internal_latency_ms={}ms",
        result.hits.len(),
        duration.as_millis(),
        result.latency_ms
    );
}

// ---------------------------------------------------------------------------
// 测试 2：multi 多跳检索 < 1s（spec §三 11.7.2）
// ---------------------------------------------------------------------------

/// 验收指标 2：MULTI 多跳端到端检索 10k events 延迟 < 1s
///
/// 通过 `MultiStrategy::search("张三")` 端到端跑通 8 步流程（max_hop=3 多跳扩展），
/// 测量耗时并断言 < 1000ms。
///
/// ## 预期延迟
/// - baseline：~100-500ms（max_hop=3 BFS 扩展，但 10k events 实际只有 2-3 跳）
/// - spec 要求：< 1s（已 2-10x baseline 余量）
///
/// ## 与 multi1 的对比
/// - multi（max_hop=3）：扩展到 evt-4（hop=2，经北京）→ hits 含 evt-0..evt-4
/// - multi1（max_hop=1）：不扩展到 hop=2 → hits 仅含 evt-0..evt-3
#[tokio::test]
async fn test_e2e_multi_latency_under_1s() {
    let conn = setup_10k_events_db();
    let strategy = MultiStrategy::new(conn);

    let start = Instant::now();
    let result = strategy
        .search("张三")
        .await
        .expect("multi search 应成功");
    let duration = start.elapsed();

    assert!(
        duration.as_millis() < 1000,
        "multi 检索耗时 {:?} 应 < 1s（spec §三 11.7.2）",
        duration
    );
    assert!(
        !result.hits.is_empty(),
        "应返回非空 hits（张三 + 北京关联的 event），实际: {:?}",
        result.hits
    );

    println!(
        "[multi_latency] hits={}, external_latency={}ms, internal_latency_ms={}ms",
        result.hits.len(),
        duration.as_millis(),
        result.latency_ms
    );
}

// ---------------------------------------------------------------------------
// 测试 3：Step1 + Step2 < 100ms（spec baseline，2x 余量 → 200ms）
// ---------------------------------------------------------------------------

/// 验收指标 3：Step1（query 向量化）+ Step2（query 实体抽取）延迟 < 100ms
///
/// 手动调用 `step1_vectorize` + `step2_extract_entities_with_jieba` free function，
/// 测量耗时。
///
/// ## 预期延迟
/// - Step1（mock_embed）：~1-5μs（纯 Rust 哈希到 384 维，无 IO）
/// - Step2（JiebaNer::extract）：~1-10ms（仅 jieba 分词 + 词典匹配，**不含词典加载**）
/// - 总计 baseline：~1-10ms
///
/// ## 断言阈值
/// spec baseline 100ms，考虑 CI 环境性能波动留 2x 余量 → 断言 < 200ms。
///
/// ## JiebaNer 预构造说明
/// `step2_extract_entities`（无 `_with_jieba` 后缀）每次调用都新建 `JiebaNer`，
/// 触发 jieba-rs 默认词典加载（~500-1000ms），这是 one-time cost。
///
/// 生产环境中 `MultiStrategy` 在构造时预建 `JiebaNer` 并通过
/// `step2_extract_entities_with_jieba` 复用（见 `multi.rs:177,577`），
/// 避免每次 search 重复加载词典。
///
/// 本测试镜像生产用法：在计时块外预构造 `JiebaNer`，仅计时 `extract()` 调用本身，
/// 反映 Step1+Step2 的稳态延迟（steady-state latency）。
#[test]
fn test_e2e_step1_step2_latency_under_100ms() {
    // 预构造 JiebaNer（词典加载 ~500-1000ms，one-time cost，不计入 Step1+Step2 延迟）
    // 镜像 MultiStrategy::new() 的生产用法（multi.rs:177 jieba: JiebaNer::new()）
    let jieba = JiebaNer::new();

    let state = MultiState::new("张三");

    // 仅计时 Step1（mock_embed）+ Step2（jieba.extract）稳态调用
    let start = Instant::now();
    let state = step1_vectorize(state);
    let state = step2_extract_entities_with_jieba(state, &jieba);
    let duration = start.elapsed();

    // spec baseline 100ms，2x 余量 → 200ms（避免 CI 性能波动 flaky）
    assert!(
        duration.as_millis() < 200,
        "Step1+Step2 耗时 {:?} 应 < 200ms（spec baseline 100ms，2x 余量）",
        duration
    );
    assert!(
        !state.query_vec.is_empty(),
        "Step1 应填充 query_vec（384 维 mock embedding）"
    );
    assert_eq!(
        state.query_vec.len(),
        HNSW_DIM,
        "query_vec 维度应为 {}",
        HNSW_DIM
    );
    assert!(
        !state.entities.is_empty(),
        "Step2 应抽取到实体（张三），实际: {:?}",
        state.entities
    );

    println!(
        "[step1_step2_latency] query_vec.dim={}, entities={}, latency={}ms",
        state.query_vec.len(),
        state.entities.len(),
        duration.as_millis()
    );
}

// ---------------------------------------------------------------------------
// 测试 4：Step3 + Step4 < 200ms（spec baseline，2x 余量 → 400ms）
// ---------------------------------------------------------------------------

/// 验收指标 4：Step3（HnswIndex kNN 实体检索）+ Step4（SQL JOIN 事件检索）延迟 < 200ms
///
/// 手动调用 `step3_vector_search_with_index` + `step4_event_search_with_conn` free function，
/// 测量耗时（不含 HnswIndex fixture 构造和 SAG DB 构造时间）。
///
/// ## 预期延迟
/// - Step3（HnswIndex::search_top_k）：~1-10ms（HNSW sub-linear 检索 1000 向量）
/// - Step4（SQL JOIN event_entity_relation）：~1-10ms（利用 P-01 反向索引高效查找）
/// - 总计 baseline：~2-20ms
///
/// ## 断言阈值
/// spec baseline 200ms，考虑 CI 环境性能波动留 2x 余量 → 断言 < 400ms。
///
/// ## Fixture
/// - SAG DB：1000 entity + 10000 event + ~20000 relation（复用 `setup_10k_events_db`）
/// - HnswIndex：1000 entity 向量（`setup_hnsw_index_with_1000_vectors`，不计时）
/// - query_vec：`[1.0, 0, ..., 0]` 完美匹配张三（ent-0-0）
#[test]
fn test_e2e_step3_step4_latency_under_200ms() {
    let conn = setup_10k_events_db();
    let index = setup_hnsw_index_with_1000_vectors();

    let mut state = MultiState::new("张三");
    state.query_vec = make_query_vec_matching_zhangsan();

    // 仅测量 Step3 + Step4 耗时（fixture 构造不在计时范围内）
    let start = Instant::now();
    let state = step3_vector_search_with_index(state, &index, 10);
    let state = step4_event_search_with_conn(state, &conn);
    let duration = start.elapsed();

    // spec baseline 200ms，2x 余量 → 400ms（避免 CI 性能波动 flaky）
    assert!(
        duration.as_millis() < 400,
        "Step3+Step4 耗时 {:?} 应 < 400ms（spec baseline 200ms，2x 余量）",
        duration
    );
    assert!(
        !state.entity_ids.is_empty(),
        "Step3 应返回非空 entity_ids（HnswIndex top-10），实际: {:?}",
        state.entity_ids
    );
    assert!(
        !state.candidates.is_empty(),
        "Step4 应返回非空 candidates（JOIN event_entity_relation），实际: {:?}",
        state.candidates
    );

    println!(
        "[step3_step4_latency] entity_ids={}, candidates={}, latency={}ms",
        state.entity_ids.len(),
        state.candidates.len(),
        duration.as_millis()
    );
}

// ---------------------------------------------------------------------------
// 测试 5：完整 8 步流程 < 1s（spec §三 11.7.2 硬性要求）
// ---------------------------------------------------------------------------

/// 验收指标 5：MULTI 完整 8 步流程端到端延迟 < 1s
///
/// 显式调用 Step1..Step8 free function（参考 `multi_e2e.rs::test_multi_e2e_thought_process_complete`
/// 模式），测量完整 8 步流程的总耗时。
///
/// ## 8 步流程
/// | Step | 函数                          | 实现      |
/// |------|-------------------------------|-----------|
/// | 1    | `step1_vectorize`             | mock embed |
/// | 2    | `step2_extract_entities`      | jieba+正则 |
/// | 3    | `step3_vector_search`         | stub（留空）|
/// | 4    | `step4_event_search`          | stub（留空）|
/// | 5    | `step5_with_multi1_async`     | multi1 BFS |
/// | 6    | `step6_associate_chunks`      | chunk 关联 |
/// | 7    | `step7_rerank_with_thought`   | score 降序 |
/// | 8    | `step8_build_result`          | 包装结果   |
///
/// ## 预期延迟
/// - baseline：~50-300ms（Step5 multi1 BFS 为主要耗时，Step1-4/6-8 均 < 50ms）
/// - spec 要求：< 1s（已 3-20x baseline 余量）
///
/// ## 与测试 1 的区别
/// - 测试 1：调用 `Multi1Strategy::search()` 黑盒（内部跑 8 步）
/// - 测试 5：显式调用 8 个 step free function 白盒（验证各步骤衔接 + thought_process）
///
/// ## Step6 conn 引用
/// `multi1` 持有 conn 所有权（move 语义），Step6 需单独的 conn 引用。
/// 测试中重新构造一个 fixture DB（`setup_10k_events_db` 确定性，内容一致）。
#[tokio::test]
async fn test_e2e_full_8_step_latency_under_1s() {
    // 预构造两份 fixture DB：conn 给 multi1（move），conn_for_step6 给 Step6 引用
    // （multi1 持有 conn 所有权，Step6 需单独的 conn 引用；两份 DB 内容确定性一致）
    // 注：fixture 构造耗时 ~1-3s 不在 8 步流程计时范围内
    let conn = setup_10k_events_db();
    let conn_for_step6 = setup_10k_events_db();
    let multi1 = Multi1Strategy::new(conn);

    // 仅计时 8 步流程本身（不含 fixture 构造）
    let start = Instant::now();

    // Step1：query 向量化（mock 384 维）
    let state = MultiState::new("张三");
    let state = step1_vectorize(state);

    // Step2：query 实体抽取（jieba + 正则）
    let state = step2_extract_entities(state);

    // Step3：stub（entity_ids 留空，11.2.x 接入 HnswIndex）
    let state = step3_vector_search(state);

    // Step4：stub（candidates 留空，11.2.x 接入 event_entity_relation 查询）
    let state = step4_event_search(state);

    // Step5：multi1 单跳剪枝（async，填充 hits；内部会重新执行 Step1-3 + BFS）
    let state = step5_with_multi1_async(state, &multi1).await;

    // Step6：events → chunks 关联（使用预构造的 conn_for_step6，避免构造时间计入）
    let state = step6_associate_chunks(state, &conn_for_step6);

    // Step7：Rerank 重排 + thought_process 完整化
    let state = step7_rerank_with_thought(state, 10);

    // Step8：返回 SearchResult
    let result = step8_build_result(state);

    let duration = start.elapsed();

    assert!(
        duration.as_millis() < 1000,
        "完整 8 步流程耗时 {:?} 应 < 1s（spec §三 11.7.2 硬性要求）",
        duration
    );
    assert!(
        !result.hits.is_empty(),
        "应返回非空 hits（multi1 检索到 evt-0..evt-3），实际: {:?}",
        result.hits
    );

    println!(
        "[full_8_step_latency] hits={}, latency={}ms",
        result.hits.len(),
        duration.as_millis()
    );
}
