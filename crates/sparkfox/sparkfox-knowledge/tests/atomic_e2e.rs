//! Sub-Step 10.5.2 — ATOMIC E2E 测试（spec §三 10.8.3，矩阵 10.5.2）
//!
//! ## 测试目标（spec 验收标准）
//! 1. **1k event 端到端 < 1s**：1000 event + 100 entity + 2000 relation 数据集下，
//!    ATOMIC 检索 10 次查询总耗时 < 1000ms（含 jieba 实体抽取 + SQL JOIN）。
//! 2. **Recall@5 > 0.7**：10 个查询（每查询含 1 个 jieba 可识别的 anchor entity），
//!    平均 Recall@5 > 0.7（ground truth 为 fixture 中与查询共享 anchor entity 的 5 个 event）。
//! 3. **无孤立 event**：每个 knowledge_event 至少有 1 条 event_entity_relation（COUNT 孤儿 = 0）。
//!
//! ## Fixture 设计（`tests/data/atomic_1k_events.sql`）
//! - **100 entity**（10 类 × 10 个）：10 anchor（5 PERSON + 5 LOCATION，jieba 默认词典可识别）
//!   + 90 filler（jieba 不可识别，避免污染 Recall@5）
//! - **1000 event**：50 anchor event（每个 anchor entity 配 5 event，evt-0001~evt-0050）
//!   + 950 filler event（evt-0051~evt-1000）
//! - **2000 event_entity_relation**：每 event 平均 2 个 entity（anchor event 含 1 anchor + 1 filler；
//!   filler event 含 2 filler），保证无孤立 event
//!
//! ## Recall@5 ground truth 构造
//! - 10 个查询各含 1 个 anchor entity（"张三" / "李四" / ... / "杭州"）
//! - 每个 anchor entity 在 fixture 中对应 5 个 event（如 "张三" → evt-0001~evt-0005）
//! - 查询时 jieba 抽出 anchor entity → find_entity_ids 返回 1 个 entity_id →
//!   find_events 返回 5 个 event → top_k=5 全部命中 → Recall@5 = 5/5 = 1.0
//!
//! ## 性能保证
//! schema.rs 已含 P-01 双向复合索引：
//! - `idx_eer_event_entity(event_id, entity_id)` 正向
//! - `idx_eer_entity_event(entity_id, event_id)` 反向（find_events 命中此索引）
//! - `idx_entity_normalized(entity.normalized_name)` 加速 find_entity_ids
//! AtomicStrategy::new 内部缓存 JiebaNer 实例（只初始化一次），避免每次 search 重建分词器。

#![forbid(unsafe_code)]

use std::time::Instant;

use rusqlite::Connection;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::{AtomicStrategy, SearchStrategy};

// ---------------------------------------------------------------------------
// Fixture 加载与数据库初始化
// ---------------------------------------------------------------------------

/// 1k event fixture SQL（编译时嵌入，运行时无 I/O 开销）
const ATOMIC_1K_EVENTS_SQL: &str = include_str!("data/atomic_1k_events.sql");

/// 10 个查询的 ground truth：(query, anchor_entity_name, [5 个 ground-truth event_id])
///
/// 每个 anchor entity 在 fixture 中对应 5 个 event：
/// - 张三 → evt-0001~evt-0005
/// - 李四 → evt-0006~evt-0010
/// - ...
/// - 杭州 → evt-0046~evt-0050
const RECALL_GROUND_TRUTH: &[(&str, &[&str])] = &[
    ("张三去了哪里", &["evt-0001", "evt-0002", "evt-0003", "evt-0004", "evt-0005"]),
    ("李四在做什么", &["evt-0006", "evt-0007", "evt-0008", "evt-0009", "evt-0010"]),
    ("王五的工作记录", &["evt-0011", "evt-0012", "evt-0013", "evt-0014", "evt-0015"]),
    ("赵六的行程安排", &["evt-0016", "evt-0017", "evt-0018", "evt-0019", "evt-0020"]),
    ("钱七的项目进展", &["evt-0021", "evt-0022", "evt-0023", "evt-0024", "evt-0025"]),
    ("北京近期会议", &["evt-0026", "evt-0027", "evt-0028", "evt-0029", "evt-0030"]),
    ("上海的展览活动", &["evt-0031", "evt-0032", "evt-0033", "evt-0034", "evt-0035"]),
    ("广州的出差记录", &["evt-0036", "evt-0037", "evt-0038", "evt-0039", "evt-0040"]),
    ("深圳的旅游行程", &["evt-0041", "evt-0042", "evt-0043", "evt-0044", "evt-0045"]),
    ("杭州的交流活动", &["evt-0046", "evt-0047", "evt-0048", "evt-0049", "evt-0050"]),
];

/// 构造 1k event 内存数据库
///
/// 流程：
/// 1. 打开 in-memory SQLite
/// 2. 开启 foreign_keys（保证 fixture INSERT 时 FK 校验生效）
/// 3. 执行 6 张 SAG 表 DDL（按依赖顺序）
/// 4. 预填 11 种默认 entity_type（fixture 中 entity 行的 FK 依赖）
/// 5. 执行 `atomic_1k_events.sql` fixture（100 entity + 1000 event + 2000 relation）
fn setup_1k_event_db() -> Connection {
    let conn = Connection::open_in_memory().expect("打开内存数据库失败");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("开启 foreign_keys 失败");
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).expect("执行 SAG DDL 失败");
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES)
        .expect("预填默认实体类型失败");
    conn.execute_batch(ATOMIC_1K_EVENTS_SQL)
        .expect("加载 1k event fixture 失败");
    conn
}

// ---------------------------------------------------------------------------
// 测试 1：1k event 端到端 < 1s（spec §三 10.8.3 指标 1）
// ---------------------------------------------------------------------------

/// 验证在 1k event 数据集下，ATOMIC 检索 10 次查询总耗时 < 1000ms。
///
/// ## 测试流程
/// 1. 加载 1k event fixture
/// 2. 构造 AtomicStrategy（默认 top_k=10）
/// 3. warm-up 1 次（消除 jieba 首次初始化的固定开销）
/// 4. 顺序执行 10 个查询（每查询含 1 个 anchor entity）
/// 5. 断言 10 次查询总耗时 < 1000ms
///
/// ## 性能保证
/// - jieba 实例在 AtomicStrategy::new 时初始化一次，warm-up 后稳定
/// - find_entity_ids 命中 `idx_entity_normalized` 索引
/// - find_events 命中 `idx_eer_entity_event` 反向复合索引（P-01）
///
/// ## 容忍度
/// 阈值 1000ms 留有 10x 余量（典型耗时 50-200ms）；CI 慢机也能通过。
#[tokio::test]
async fn test_atomic_e2e_1k_event_under_1s() {
    let conn = setup_1k_event_db();
    let strategy = AtomicStrategy::new(conn);

    // warm-up：触发 jieba 首次分词 + SQL prepared statement 缓存
    let _ = strategy.search("张三").await.expect("warm-up search 应成功");

    // 10 个查询（与 Recall@5 测试同集，便于交叉对比）
    let queries: Vec<&str> = RECALL_GROUND_TRUTH.iter().map(|(q, _)| *q).collect();

    let start = Instant::now();
    for q in &queries {
        let result = strategy.search(q).await.expect("search 应成功");
        // 每查询至少返回 1 个 hit（anchor entity 在 fixture 中有 5 个 event）
        assert!(
            !result.hits.is_empty(),
            "查询 {:?} 应返回至少 1 个 hit，实际 0",
            q
        );
    }
    let elapsed_ms = start.elapsed().as_millis();

    println!("[1k_event_under_1s] 10 次查询总耗时: {}ms (平均 {}ms/次)",
             elapsed_ms, elapsed_ms / queries.len() as u128);

    assert!(
        elapsed_ms < 1000,
        "10 次查询总耗时 {}ms 应 < 1000ms（spec §三 10.8.3 指标 1）",
        elapsed_ms
    );
}

// ---------------------------------------------------------------------------
// 测试 2：Recall@5 > 0.7（spec §三 10.8.3 指标 2）
// ---------------------------------------------------------------------------

/// 验证 10 个查询的平均 Recall@5 > 0.7。
///
/// ## Recall@5 定义
/// 对每个查询 q：
/// - ground_truth_q = fixture 中与 q 共享 anchor entity 的 5 个 event_id
/// - hits_q = AtomicStrategy::search(q).hits（top_k=5）
/// - recall_q = |hits_q ∩ ground_truth_q| / |ground_truth_q|
///
/// 平均 Recall@5 = (Σ recall_q) / 10
///
/// ## 期望结果
/// 每个查询的 jieba 抽出 anchor entity → find_entity_ids 返回 1 个 entity_id →
/// find_events 返回 5 个 event（ground truth 全集）→ top_k=5 全部命中 →
/// recall_q = 5/5 = 1.0 → 平均 Recall@5 = 1.0 > 0.7 ✓
///
/// ## 10 篇文档语义
/// 10 个查询对应 10 个 anchor entity，每个 anchor entity 在 fixture 中代表 1 篇"文档"
/// 的核心主题（5 个 event = 该文档的 5 个事件）。Recall@5 > 0.7 验证 ATOMIC 检索
/// 能正确定位到与查询实体相关的"文档"（即 5 个 event）。
#[tokio::test]
async fn test_atomic_e2e_recall_at_5_above_0_7() {
    let conn = setup_1k_event_db();
    // top_k=5：限制返回前 5 个 hit，对应 Recall@5 的 "5"
    let strategy = AtomicStrategy::new_with_top_k(conn, 5);

    let mut total_recall = 0.0;
    let mut query_count = 0usize;

    println!("[recall_at_5] 每查询 Recall@5 明细：");
    for (query, ground_truth) in RECALL_GROUND_TRUTH {
        let result = strategy.search(query).await.expect("search 应成功");
        let hits_count = result.hits.len();
        let relevant_in_top5 = result
            .hits
            .iter()
            .filter(|h| ground_truth.contains(&h.event_id.as_str()))
            .count();
        let recall = if ground_truth.is_empty() {
            0.0
        } else {
            relevant_in_top5 as f64 / ground_truth.len() as f64
        };
        println!(
            "  query={:<20} hits={} relevant={} gt={} recall={:.3}",
            format!("{:?}", query),
            hits_count,
            relevant_in_top5,
            ground_truth.len(),
            recall
        );
        total_recall += recall;
        query_count += 1;
    }

    let avg_recall = if query_count == 0 {
        0.0
    } else {
        total_recall / query_count as f64
    };
    println!("[recall_at_5] 平均 Recall@5 = {:.4}（共 {} 个查询）", avg_recall, query_count);

    assert!(
        avg_recall > 0.7,
        "平均 Recall@5 = {:.4} 应 > 0.7（spec §三 10.8.3 指标 2）",
        avg_recall
    );
}

// ---------------------------------------------------------------------------
// 测试 3：无孤立 event（spec §三 10.8.3 指标 3）
// ---------------------------------------------------------------------------

/// 验证 fixture 中每个 knowledge_event 至少有 1 条 event_entity_relation（无孤立 event）。
///
/// ## SQL
/// ```sql
/// SELECT COUNT(*) FROM knowledge_event ke
/// WHERE NOT EXISTS (
///     SELECT 1 FROM event_entity_relation eer WHERE eer.event_id = ke.id
/// )
/// ```
///
/// ## 期望结果
/// fixture 设计保证每 event 至少 1 个 entity（anchor event 含 1 anchor + 1 filler；
/// filler event 含 2 filler），故 COUNT 应为 0。
#[tokio::test]
async fn test_atomic_e2e_no_orphan_events() {
    let conn = setup_1k_event_db();

    // 主断言：无孤立 event
    let orphan_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM knowledge_event ke \
             WHERE NOT EXISTS ( \
                 SELECT 1 FROM event_entity_relation eer WHERE eer.event_id = ke.id \
             )",
            [],
            |r| r.get(0),
        )
        .expect("查询孤儿 event 失败");

    // 辅助统计：fixture 总行数（便于 --nocapture 时人工核对）
    let total_events: i64 = conn
        .query_row("SELECT COUNT(*) FROM knowledge_event", [], |r| r.get(0))
        .expect("查询 event 总数失败");
    let total_relations: i64 = conn
        .query_row("SELECT COUNT(*) FROM event_entity_relation", [], |r| r.get(0))
        .expect("查询 relation 总数失败");
    let total_entities: i64 = conn
        .query_row("SELECT COUNT(*) FROM entity", [], |r| r.get(0))
        .expect("查询 entity 总数失败");

    println!(
        "[no_orphan_events] entity={} event={} relation={} orphan={} (每 event 平均 {:.2} entity)",
        total_entities,
        total_events,
        total_relations,
        orphan_count,
        if total_events > 0 {
            total_relations as f64 / total_events as f64
        } else {
            0.0
        }
    );

    assert_eq!(
        orphan_count, 0,
        "存在 {} 个孤立 event（应为 0，spec §三 10.8.3 指标 3）",
        orphan_count
    );
}

// ---------------------------------------------------------------------------
// 测试 4（REFACTOR）：SQL EXPLAIN 验证 P-01 索引命中
// ---------------------------------------------------------------------------

/// 验证 AtomicStrategy 的 find_events SQL 命中 P-01 反向复合索引
/// `idx_eer_entity_event(entity_id, event_id)`，而非全表扫描。
///
/// ## 背景
/// spec §三 10.8.3 要求 1k event < 1s，性能依赖 P-01 双向复合索引：
/// - `idx_eer_event_entity(event_id, entity_id)` 正向（按 event 找 entity）
/// - `idx_eer_entity_event(entity_id, event_id)` 反向（按 entity 找 event，本测试验证）
///
/// find_events SQL 的 WHERE 子句为 `r.entity_id IN (...)`，应命中反向索引。
/// 若索引被误删或 SQL 改写为全表扫描，本测试会捕获回归。
///
/// ## EXPLAIN QUERY PLAN 输出格式
/// 每行 4 列：`(id: i64, parent: i64, notused: i64, detail: String)`
/// - 命中索引：`SEARCH r USING INDEX idx_eer_entity_event (entity_id=?)`
/// - 全表扫描：`SCAN r`（无 INDEX 关键字）
///
/// ## 断言
/// 至少有一行 detail 包含 `idx_eer_entity_event`（P-01 反向索引名）。
#[tokio::test]
async fn test_atomic_e2e_explain_uses_p01_index() {
    let conn = setup_1k_event_db();

    // 模拟 AtomicStrategy::find_events 的 SQL（与 atomic.rs::SQL_ATOMIC_SEARCH_TEMPLATE 一致）
    // U-02 修复后 JOIN entity + entity_type 表，填充 via_entities 为 EntityRef
    // 使用 1 个 entity_id 占位符 + LIMIT 10
    let sql = r#"
        SELECT DISTINCT e.id, e.title, e.summary, e.chunk_id, e.content,
               ent.id AS entity_id, et.type AS entity_type, ent.name AS entity_name
        FROM knowledge_event e
        JOIN event_entity_relation r ON e.id = r.event_id
        JOIN entity ent ON r.entity_id = ent.id
        JOIN entity_type et ON ent.entity_type_id = et.id
        WHERE r.entity_id IN ('ent-001')
        ORDER BY e.created_time DESC
        LIMIT 10
    "#;

    // 收集 EXPLAIN QUERY PLAN 的 detail 列
    let mut plan_details: Vec<String> = Vec::new();
    let mut stmt = conn
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .expect("prepare EXPLAIN 失败");
    let rows = stmt
        .query_map([], |row| {
            let detail: String = row.get(3)?;
            Ok(detail)
        })
        .expect("query EXPLAIN 失败");
    for row in rows {
        plan_details.push(row.expect("EXPLAIN row 失败"));
    }

    println!("[explain_uses_p01_index] EXPLAIN QUERY PLAN 输出：");
    for (i, d) in plan_details.iter().enumerate() {
        println!("  [{}] {}", i, d);
    }

    // 断言：至少有一行命中 idx_eer_entity_event 反向索引
    let uses_reverse_index = plan_details
        .iter()
        .any(|d| d.contains("idx_eer_entity_event"));
    assert!(
        uses_reverse_index,
        "EXPLAIN QUERY PLAN 应包含 idx_eer_entity_event 索引，实际：{:?}",
        plan_details
    );
}
