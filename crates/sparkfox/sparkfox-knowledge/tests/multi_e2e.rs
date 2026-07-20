//! Sub-Step 11.1.4 — MULTI 8 步流程 E2E 集成（10k events fixture，spec §三 11.1.4）
//!
//! ## 测试目标（spec §三 11.1.4，4 测试）
//! 1. `test_multi_e2e_returns_search_result`：E2E 返回 SearchResult（含 hits / latency_ms / strategy_name）
//! 2. `test_multi_e2e_thought_process_complete`：thought_process 含 Step1..Step7（7 条记录）
//! 3. `test_multi_e2e_search_hits_have_hop_via_entities`：SearchHit 携带 hop / via_entities
//! 4. `test_multi_e2e_recall_at_5_above_0_6`：Recall@5 > 0.6（multi1 占位，预期低于完整 multi）
//!
//! ## 10k events fixture 设计
//! - **1000 entity**（10 类型 × 100 个，ID 格式 `ent-{type_idx}-{i}`）
//!   - `ent-0-0` = 张三（PERSON，Recall@5 anchor 实体，jieba 默认词典可识别）
//!   - `ent-1-0` = 北京（LOCATION，hop=2 中间实体）
//!   - 其他 `ent-{type_idx}-{i}` = `实体_{type_idx}_{i}`（filler，jieba 不可识别）
//! - **10000 event**（`evt-0`..`evt-9999`）
//!   - `evt-0`..`evt-3`：张三 + 北京（4 个，Recall@5 ground truth 的 hop=1 部分）
//!   - `evt-4`：仅北京（1 个，Recall@5 ground truth 的 hop=2 部分，multi1 不返回）
//!   - `evt-5`..`evt-9999`：filler events（关联 filler entities，避免污染 Recall@5）
//! - **~20000 event_entity_relation**：
//!   - `evt-0`..`evt-3` → 张三 + 北京（8 条）
//!   - `evt-4` → 北京（1 条）
//!   - `evt-5`..`evt-9999` → 1-3 个 filler entities（约 20000 条）
//!
//! ## Recall@5 ground truth
//! 查询「张三」期望返回的 5 个相关 event：
//! - `evt-0`..`evt-3`：张三直接关联（hop=1，multi1 应返回）
//! - `evt-4`：通过北京间接关联（hop=2，multi1 不返回，multi 会返回）
//!
//! multi1（max_hop=1）预期返回 4 个 hit（evt-0..evt-3），Recall@5 = 4/5 = 0.8 > 0.6 ✓
//!
//! ## 8 步流程
//! 通过 `Multi1Strategy`（max_hop=1）端到端验证 8 步流程跑通：
//! Step1（向量化）→ Step2（实体抽取）→ Step3（实体检索）→ Step4（事件检索 stub）
//! → Step5（multi1 单跳剪枝）→ Step6（chunk 关联）→ Step7（rerank）→ Step8（返回结果）
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::multi::Multi1Strategy;
use sparkfox_knowledge::search::multi_step::{
    step1_vectorize, step2_extract_entities, step3_vector_search, step4_event_search,
    step5_with_multi1_async, step6_associate_chunks, step7_rerank_with_thought, MultiState,
};
use sparkfox_knowledge::search::SearchStrategy;

// ---------------------------------------------------------------------------
// 常量定义
// ---------------------------------------------------------------------------

/// 10 种实体类型 ID（与 schema.rs::ENTITY_TYPES 对齐，跳过 OTHER 兜底类型）
///
/// 索引与 entity_id 的 type_idx 一致：`ent-{type_idx}-{i}` 对应 `ENTITY_TYPE_IDS[type_idx]`。
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

/// Recall@5 ground truth：查询「张三」对应的 5 个相关 event_ids
///
/// - `evt-0`..`evt-3`：张三直接关联（hop=1，multi1 应返回）
/// - `evt-4`：通过北京间接关联（hop=2，multi1 不返回，multi 会返回）
const RECALL_GROUND_TRUTH: &[&str] = &["evt-0", "evt-1", "evt-2", "evt-3", "evt-4"];

// ---------------------------------------------------------------------------
// Fixture：10k events 内存数据库
// ---------------------------------------------------------------------------

/// 构造 10k events 测试 DB（1000 entity + 10000 event + ~20000 relation）
///
/// ## 拓扑设计
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
/// - 不扩展到 `evt-4`（hop=2，超出 max_hop=1）
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
            // 特殊命名：张三 / 北京（jieba 可识别）
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
        // 时间戳：从 2026-07-20 开始，每分钟一个 event（覆盖多天）
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
            // 选择 filler entity：type_idx=2..9，避开 ent-0-* 和 ent-1-*（避免污染 Recall@5）
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
// 测试 1：E2E 返回 SearchResult（含 hits / latency_ms / strategy_name）
// ---------------------------------------------------------------------------

/// 验收指标 1：MULTI 8 步流程 E2E 返回结构完整的 SearchResult
///
/// 通过 `Multi1Strategy::search("张三")` 端到端跑通 8 步流程，验证：
/// - `result.strategy_name == "multi1"`（multi1 占位策略名）
/// - `result.hits` 非空（至少返回 evt-0 张三直接关联的 event）
/// - `result.latency_ms` 合理（10k event fixture 下应 < 5000ms）
#[tokio::test]
async fn test_multi_e2e_returns_search_result() {
    let conn = setup_10k_events_db();
    let strategy = Multi1Strategy::new(conn);

    let result = strategy
        .search("张三")
        .await
        .expect("multi1 search 应成功");

    // 验证 strategy_name
    assert_eq!(
        result.strategy_name, "multi1",
        "strategy_name 应为 multi1"
    );

    // 验证 hits 非空（至少返回 evt-0..evt-3 中的一个）
    assert!(
        !result.hits.is_empty(),
        "应返回至少 1 个 hit（张三直接关联的 event），实际: {:?}",
        result.hits
    );

    // 验证 latency_ms 合理（10k fixture 下 multi1 应 < 5000ms，留 10x 余量）
    assert!(
        result.latency_ms < 5000,
        "10k event 检索耗时 {}ms 应 < 5000ms",
        result.latency_ms
    );

    println!(
        "[multi_e2e_returns] hits={}, latency_ms={}ms, strategy={}",
        result.hits.len(),
        result.latency_ms,
        result.strategy_name
    );
}

// ---------------------------------------------------------------------------
// 测试 2：thought_process 含 Step1..Step7（7 条记录）
// ---------------------------------------------------------------------------

/// 验收指标 2：完整执行 Step1..Step7 后 thought_process 含 7 条记录
///
/// 手动按顺序调用 8 步流程的 free function（Step1-7），断言 thought_process
/// 含 Step1 / Step2 / Step3 / Step4 / Step5 / Step6 / Step7 各至少一条。
///
/// ## 注：Step6 需要 conn 引用
/// `multi1` 持有 conn 所有权（move 语义），Step6 需要单独的 conn 引用。
/// 测试中重新构造一个 fixture DB（`setup_10k_events_db` 确定性，内容一致）。
#[tokio::test]
async fn test_multi_e2e_thought_process_complete() {
    let conn = setup_10k_events_db();
    let multi1 = Multi1Strategy::new(conn);

    // Step1：query 向量化（mock 384 维）
    let state = MultiState::new("张三");
    let state = step1_vectorize(state);

    // Step2：query 实体抽取（jieba + 正则）
    let state = step2_extract_entities(state);

    // Step3：stub（entity_ids 留空，11.2.x 接入 HnswIndex）
    let state = step3_vector_search(state);

    // Step4：stub（candidates 留空，11.2.x 接入 event_entity_relation 查询）
    let state = step4_event_search(state);

    // Step5：multi1 单跳剪枝（async，填充 hits）
    let state = step5_with_multi1_async(state, &multi1).await;

    // Step6：events → chunks 关联（需要 conn 引用，重新构造 fixture DB）
    let conn_for_step6 = setup_10k_events_db();
    let state = step6_associate_chunks(state, &conn_for_step6);

    // Step7：Rerank 重排 + thought_process 完整化
    let state = step7_rerank_with_thought(state, 10);

    // 验证 thought_process 含 Step1..Step7（7 条记录）
    for step_num in 1..=7 {
        let step_label = format!("Step{}", step_num);
        assert!(
            state.thought_process.iter().any(|s| s.contains(&step_label)),
            "thought_process 应含 {}，实际: {:?}",
            step_label,
            state.thought_process
        );
    }

    // 计数：thought_process 至少 7 条 Step 记录
    let step_count = state
        .thought_process
        .iter()
        .filter(|s| {
            s.contains("Step1")
                || s.contains("Step2")
                || s.contains("Step3")
                || s.contains("Step4")
                || s.contains("Step5")
                || s.contains("Step6")
                || s.contains("Step7")
        })
        .count();
    assert!(
        step_count >= 7,
        "thought_process 应至少 7 条 Step 记录，实际 {} 条 ({:?})",
        step_count,
        state.thought_process
    );

    println!(
        "[thought_process_complete] 共 {} 条记录",
        state.thought_process.len()
    );
    for (i, entry) in state.thought_process.iter().enumerate() {
        println!("  [{}] {}", i, entry);
    }
}

// ---------------------------------------------------------------------------
// 测试 3：SearchHit 携带 hop / via_entities
// ---------------------------------------------------------------------------

/// 验收指标 3：MULTI E2E 检索的 SearchHit 携带 hop / via_entities 字段
///
/// 通过 `Multi1Strategy::search("张三")` 端到端跑通 8 步流程，验证每个 hit：
/// - `hop == Some(1)`（multi1 max_hop=1，所有 hit 均为单跳）
/// - `via_entities` 非空（BFS 路径上的 EntityRef 列表）
/// - `EntityRef.entity_id` / `EntityRef.name` 字段已填充
#[tokio::test]
async fn test_multi_e2e_search_hits_have_hop_via_entities() {
    let conn = setup_10k_events_db();
    let strategy = Multi1Strategy::new(conn);

    let result = strategy
        .search("张三")
        .await
        .expect("multi1 search 应成功");

    assert!(
        !result.hits.is_empty(),
        "应返回至少 1 个 hit（张三直接关联的 event），实际: {:?}",
        result.hits
    );

    // 验证每个 hit 的 hop / via_entities 字段
    for hit in &result.hits {
        // multi1 max_hop=1，所有 hit.hop 应为 Some(1)
        assert_eq!(
            hit.hop,
            Some(1),
            "multi1 返回的 hit.hop 应为 Some(1)，实际 evt-{} hop={:?}",
            hit.event_id,
            hit.hop
        );

        // via_entities 应非空（BFS 路径上的 EntityRef 列表）
        assert!(
            !hit.via_entities.is_empty(),
            "hit.via_entities 应非空，实际 evt-{} via_entities={:?}",
            hit.event_id,
            hit.via_entities
        );

        // EntityRef 字段应已填充（entity_id / name 非空）
        for ent_ref in &hit.via_entities {
            assert!(
                !ent_ref.entity_id.is_empty(),
                "EntityRef.entity_id 应非空，实际: {:?}",
                ent_ref
            );
            assert!(
                !ent_ref.name.is_empty(),
                "EntityRef.name 应非空，实际: {:?}",
                ent_ref
            );
        }
    }

    println!("[hits_have_hop_via] hits={}", result.hits.len());
    for hit in result.hits.iter().take(3) {
        println!(
            "  evt-{} hop={:?} score={:.3} via_entities={:?}",
            hit.event_id, hit.hop, hit.score, hit.via_entities
        );
    }
}

// ---------------------------------------------------------------------------
// 测试 4：Recall@5 > 0.6（multi1 占位，预期低于完整 multi）
// ---------------------------------------------------------------------------

/// 验收指标 4：MULTI E2E 检索 Recall@5 > 0.6
///
/// ## Recall@5 定义
/// `Recall@5 = |top5_hits ∩ ground_truth| / |ground_truth|`
///
/// ## ground truth
/// 查询「张三」对应的 5 个相关 event（[`RECALL_GROUND_TRUTH`]）：
/// - `evt-0`..`evt-3`：张三直接关联（hop=1，multi1 应返回）
/// - `evt-4`：通过北京间接关联（hop=2，multi1 不返回）
///
/// ## multi1 预期
/// - max_hop=1 → BFS 仅扩展 1 跳 → 返回 evt-0..evt-3（4 hits）
/// - top5 = 4 hits（不足 5 个，取全部）
/// - 命中 ground_truth = 4 个（evt-0..evt-3）
/// - Recall@5 = 4/5 = 0.8 > 0.6 ✓
///
/// ## 与完整 multi 的对比
/// - multi（max_hop=3）：会扩展到 evt-4（hop=2，经北京）→ Recall@5 = 5/5 = 1.0
/// - multi1（max_hop=1）：不扩展到 hop=2 → Recall@5 = 0.8（低于完整 multi）
#[tokio::test]
async fn test_multi_e2e_recall_at_5_above_0_6() {
    let conn = setup_10k_events_db();
    let strategy = Multi1Strategy::new(conn);

    let result = strategy
        .search("张三")
        .await
        .expect("multi1 search 应成功");

    // 取 top 5 hits（multi1 可能返回不足 5 个，取全部）
    let top5: Vec<&str> = result
        .hits
        .iter()
        .take(5)
        .map(|h| h.event_id.as_str())
        .collect();

    // 计算 Recall@5：top5 中命中 ground_truth 的数量 / ground_truth 总数
    let relevant_in_top5 = top5
        .iter()
        .filter(|id| RECALL_GROUND_TRUTH.contains(id))
        .count();
    let recall = relevant_in_top5 as f32 / RECALL_GROUND_TRUTH.len() as f32;

    println!(
        "[recall_at_5] top5={:?}, ground_truth={:?}, relevant={}, recall={:.2}",
        top5, RECALL_GROUND_TRUTH, relevant_in_top5, recall
    );

    assert!(
        recall > 0.6,
        "Recall@5 = {:.2} 应 > 0.6（multi1 占位，预期低于完整 multi）",
        recall
    );
}
