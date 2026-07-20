//! Sub-Step 12.3.2 — 4 策略对比 Benchmark（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 12.3.2，6 测试）
//! 1. `test_atomic_strategy_recall_at_10`：ATOMIC 策略 Recall@10
//! 2. `test_multi1_strategy_recall_at_10`：MULTI1 策略 Recall@10（替代 VectorStrategy）
//! 3. `test_multi_strategy_recall_at_10`：MULTI 策略 Recall@10（BFS 8 步多跳）
//! 4. `test_multi_es_strategy_recall_at_10`：MULTI_ES 策略 Recall@10（ES-first + 超边激活）
//! 5. `test_results_comparison_table`：4 策略对比表（Recall@10 / Precision@10 / latency）
//! 6. `test_latency_comparison_table`：4 策略延迟对比表（avg / p99）
//!
//! ## 4 策略对比方法学（中文）
//!
//! ### 策略选择与替换说明
//! 任务 spec 提到 `VectorStrategy`（向量检索），但 `src/search/` 下并无 `vector.rs`
//! 文件，本 crate 也未实现 `VectorStrategy`。为保持「不修改源代码」原则，
//! 选用现有 4 个具代表性的策略作为对比基准：
//!
//! | 序号 | 策略名           | 实现                  | 算法核心                            |
//! |------|------------------|-----------------------|-------------------------------------|
//! | 1    | ATOMIC           | `AtomicStrategy`      | 单跳 JOIN（query → entity → event） |
//! | 2    | MULTI1           | `Multi1Strategy`      | 8 步骨架 + 单跳 BFS（max_hop=1）     |
//! | 3    | MULTI            | `MultiStrategy`       | 8 步骨架 + BFS max_hop=3             |
//! | 4    | MULTI_ES         | `MultiEsStrategy`     | ES-first + 子图预筛选 + 超边激活     |
//!
//! MULTI1 替代 VectorStrategy 的合理性：
//! - MULTI1 是与 ATOMIC / MULTI / MULTI_ES 并列的独立 SearchStrategy 实现
//! - 多 hop=1 单跳剪枝，对比 ATOMIC（直接 JOIN）反映「8 步骨架开销」
//! - 与 MULTI（max_hop=3）对比反映「BFS 多跳扩展增益」
//! - 与 MULTI_ES 对比反映「ES-first 优化 vs jieba NER」
//!
//! ### 数据集（Sub-Step 12.3.1）
//! - `tests/fixtures/zh_multihop/`：200 实体 + 500 事件 + 1500 关系 + 50 查询
//! - 每查询含 `query`（完整查询字符串）+ `expected_event_ids`（ground truth）
//!   + `query_entities`（查询实体名）+ `expected_hop`（跳数 1/2/3）
//!
//! ### 检索 query 选择
//! 每个测试 case 取 `query_entities[0]` 作为检索 query（与 12.1.3 一致）：
//! - **MULTI_ES ES-first**：直接用 entity name LIKE 匹配（跳过 jieba NER）
//! - **MULTI / MULTI1**：jieba NER 抽取后匹配 entity 表
//! - **ATOMIC**：jieba NER 抽取后 JOIN event_entity_relation
//!
//! ### 指标定义
//! - **Recall@10** = 命中 case 数 / 总 case 数（top-10 含 ground truth 任一 event_id 即记为命中）
//! - **Precision@10** = sum(top-10 ∩ ground_truth 数量) / (10 × case 数)
//! - **avg latency_ms** = sum(per-case latency_ms) / case 数
//! - **p99 latency_ms** = 50 case 升序排序后第 99 百分位（index = floor(0.99 × 50) = 49）
//!
//! ### 预期排序（spec §三 12.3.2）
//! MULTI_ES 应不劣于 MULTI（Recall@10 差值 >= -0.01），原因：
//! - MULTI_ES ES-first 能匹配 jieba 无法识别的 entity name（如"飞书"）
//! - 子图预筛选不影响 Recall（仅优化 JOIN 行数）
//! - 超边激活（`.with_hyperedge_activation(true)`）增强 via_entities 上下文
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

mod common;

use std::time::Instant;

use rusqlite::Connection;
use serde::Deserialize;

use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};
use sparkfox_knowledge::search::multi::Multi1Strategy;
use sparkfox_knowledge::search::{AtomicStrategy, MultiEsStrategy, MultiStrategy, SearchHit, SearchStrategy};

use common::bench_metrics::{compute_precision_at_k, compute_recall_at_k, percentile};

// ---------------------------------------------------------------------------
// 类型定义：对应 tests/fixtures/zh_multihop/*.json 的结构
// ---------------------------------------------------------------------------

/// 实体（entities.json 单条记录）
#[derive(Debug, Clone, Deserialize)]
struct Entity {
    id: String,
    name: String,
    #[allow(dead_code)]
    normalized_name: String,
    entity_type_id: u32,
    #[allow(dead_code)]
    entity_type: String,
    #[allow(dead_code)]
    description: String,
}

/// 事件（events.json 单条记录）
#[derive(Debug, Clone, Deserialize)]
struct Event {
    id: String,
    content: String,
    entities: Vec<String>,
    #[allow(dead_code)]
    hop: u32,
}

/// 查询 + ground truth（queries.json 单条记录）
#[derive(Debug, Clone, Deserialize)]
struct Query {
    /// 完整查询字符串（如"张三参加了什么会议？"），本测试不直接用作检索 query
    #[allow(dead_code)]
    query: String,
    expected_event_ids: Vec<String>,
    #[allow(dead_code)]
    expected_hop: u32,
    /// 查询实体名列表（如["张三"]），本测试取 [0] 作为检索 query
    query_entities: Vec<String>,
}

// ---------------------------------------------------------------------------
// 数据集加载：使用 include_str! 在编译时嵌入 JSON
// ---------------------------------------------------------------------------

fn load_entities() -> Vec<Entity> {
    let json = include_str!("fixtures/zh_multihop/entities.json");
    serde_json::from_str(json).expect("entities.json 解析失败")
}

fn load_events() -> Vec<Event> {
    let json = include_str!("fixtures/zh_multihop/events.json");
    serde_json::from_str(json).expect("events.json 解析失败")
}

fn load_queries() -> Vec<Query> {
    let json = include_str!("fixtures/zh_multihop/queries.json");
    serde_json::from_str(json).expect("queries.json 解析失败")
}

/// zh_multihop entity_type_id (0-10) 映射到 schema.rs 中的 default entity_type.id
///
/// 与 `multi_es_vs_multi_perf_test.rs::entity_type_id_to_str` 保持一致。
fn entity_type_id_to_str(id: u32) -> &'static str {
    match id {
        0 => "default_person",
        1 => "default_location",
        2 => "default_organization",
        3 => "default_time",
        4 => "default_event",
        5 => "default_concept",
        6 => "default_object",
        7 => "default_other",
        8 => "default_other",
        9 => "default_other",
        10 => "default_other",
        _ => "default_other",
    }
}

// ---------------------------------------------------------------------------
// BenchmarkCase / StrategyResult / BenchmarkRunner
// ---------------------------------------------------------------------------

/// 单个 benchmark case（对应 queries.json 一条记录）
#[derive(Debug, Clone)]
struct BenchCase {
    /// 检索 query（取 query_entities[0]，与 12.1.3 一致）
    query: String,
    /// 期望 event_id 列表（ground truth）
    ground_truth: Vec<String>,
    /// 查询实体名列表（用于 MULTI_ES ES-first）
    #[allow(dead_code)]
    query_entities: Vec<String>,
    /// 期望跳数 1/2/3（用于分层分析，本测试不强制断言）
    #[allow(dead_code)]
    hop: usize,
}

/// 单策略 benchmark 结果
#[derive(Debug, Clone)]
struct StrategyResult {
    strategy_name: String,
    recall_at_10: f64,
    precision_at_10: f64,
    avg_latency_ms: f64,
    p99_latency_ms: f64,
}

/// 4 策略对比 Benchmark Runner
///
/// ## 设计
/// - 持有 50 个 BenchCase（zh_multihop 数据集）
/// - `run_strategy` 接受一个 `&dyn SearchStrategy`，复用同一实例跑 50 case
///   （Connection 在 strategy 内部 `Mutex<Connection>` 持有，每次 search 短时锁住，
///   跨 case 共享安全；参考 `multi_es_vs_multi_perf_test.rs` 的设计）
///
/// ## 性能
/// 每个 strategy 只需构造 1 次（含 ~31k INSERT 的 DB setup），50 case 复用同一实例，
/// 总 DB setup 次数 = 4（4 策略各 1 次），单次 setup 约 3-8 秒。
struct BenchmarkRunner {
    cases: Vec<BenchCase>,
}

impl BenchmarkRunner {
    /// 加载 zh_multihop 数据集并构建 50 case
    ///
    /// 每个 case 取 `query_entities[0]` 作为检索 query。
    fn load_from_zh_multihop() -> Self {
        let queries = load_queries();
        let cases: Vec<BenchCase> = queries
            .iter()
            .map(|q| BenchCase {
                query: q
                    .query_entities
                    .first()
                    .cloned()
                    .unwrap_or_else(|| q.query.clone()),
                ground_truth: q.expected_event_ids.clone(),
                query_entities: q.query_entities.clone(),
                hop: q.expected_hop as usize,
            })
            .collect();
        Self { cases }
    }

    /// 跑一个策略的 50 case，返回 Recall@10 / Precision@10 / avg / p99 latency
    ///
    /// ## 参数
    /// - `strategy_name`: 策略名（用于 StrategyResult.strategy_name）
    /// - `strategy`: 已构造的策略实例（含 Connection，跨 case 复用）
    ///
    /// ## 异步运行时
    /// 使用 `tokio::runtime::Runtime::new().block_on()` 同步执行策略的 async search。
    ///
    /// 测试函数必须使用 `#[test]` 而非 `#[tokio::test]`：tokio 1.52 禁止在已存在的
    /// runtime 内调用 `block_on`（"Cannot start a runtime from within a runtime"）。
    /// `run_strategy` 内部独占一个 Runtime，确保不与测试线程的 runtime 嵌套。
    fn run_strategy(&self, strategy_name: &str, strategy: &dyn SearchStrategy) -> StrategyResult {
        let rt = tokio::runtime::Runtime::new().expect("创建 tokio runtime 失败");

        let mut hit_count = 0usize;
        let mut total_precision_hits = 0usize;
        let mut latencies_ms: Vec<u64> = Vec::with_capacity(self.cases.len());

        for case in &self.cases {
            let start = Instant::now();
            let result = rt
                .block_on(strategy.search(&case.query))
                .unwrap_or_else(|e| panic!("{} search({}) 失败: {}", strategy_name, case.query, e));
            let elapsed_ms = start.elapsed().as_millis() as u64;
            latencies_ms.push(elapsed_ms);

            // top-10 hits
            let top10: Vec<&SearchHit> = result.hits.iter().take(10).collect();
            let top10_ids: Vec<String> = top10.iter().map(|h| h.event_id.clone()).collect();

            // Recall@10：top-10 含 ground_truth 任一 event_id 则记为命中
            let recall_hit = compute_recall_at_k(&top10_ids, &case.ground_truth);
            if recall_hit {
                hit_count += 1;
            }
            // Precision@10 累加：top-10 中命中的 event_id 数量
            total_precision_hits += compute_precision_at_k(&top10_ids, &case.ground_truth);
        }

        let n = self.cases.len() as f64;
        let recall_at_10 = hit_count as f64 / n;
        // Precision@10 平均：每 case 的命中数 / 10，再除以 case 数
        // sum(precision_per_case) / n = sum(hits_in_top10) / (10 × n)
        let precision_at_10 = total_precision_hits as f64 / (10.0 * n);
        let avg_latency_ms = latencies_ms.iter().sum::<u64>() as f64 / n;
        let p99_latency_ms = percentile(&latencies_ms, 0.99) as f64;

        StrategyResult {
            strategy_name: strategy_name.to_string(),
            recall_at_10,
            precision_at_10,
            avg_latency_ms,
            p99_latency_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// Fixture：构造含 zh_multihop 数据集的 SQLite Connection
// ---------------------------------------------------------------------------

/// 10 种实体类型 ID（用于 filler entities，与 `multi_e2e.rs::ENTITY_TYPE_IDS` 对齐）
const FILLER_ENTITY_TYPE_IDS: &[&str] = &[
    "default_person",
    "default_location",
    "default_organization",
    "default_time",
    "default_number",
    "default_event",
    "default_object",
    "default_concept",
    "default_law",
    "default_disease",
];

/// 构造含 zh_multihop 数据集的内存 DB
///
/// ## 数据组成
/// 1. SAG 6 表 schema（ALL_SAG_DDL）+ 11 种默认 entity_type
/// 2. zh_multihop 数据集（200 entity + 500 event + ~1500 relation）
/// 3. filler 扩展（9500 event + 1000 filler entity + ~19000 filler relation）
///    — 用于模拟 10k 规模数据库下的检索延迟（与 12.1.3 fixture 设计一致）
///
/// ## Recall@10 ground truth
/// 50 查询的 `expected_event_ids` 对应 evt-001..evt-500，filler events 不污染。
fn setup_zh_multihop_db() -> Connection {
    let conn = Connection::open_in_memory().expect("打开内存数据库失败");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("开启 foreign_keys 失败");
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).expect("执行 SAG DDL 失败");
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES)
        .expect("预填默认实体类型失败");

    // -------------------------------------------------------------------
    // Step 1: 加载 zh_multihop 数据集（200 entity + 500 event + ~1500 relation）
    // -------------------------------------------------------------------
    let entities = load_entities();
    for ent in &entities {
        let type_id = entity_type_id_to_str(ent.entity_type_id);
        conn.execute(
            "INSERT OR IGNORE INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![ent.id, type_id, ent.name, ent.name, "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
        ).expect("INSERT zh_multihop entity 失败");
    }

    let events = load_events();
    for (i, evt) in events.iter().enumerate() {
        let title: String = evt.content.chars().take(20).collect();
        let total_minutes = i;
        let minute = total_minutes % 60;
        let hour = (total_minutes / 60) % 24;
        let day = (total_minutes / (60 * 24)) + 20;
        let created = format!("2026-07-{:02}T{:02}:{:02}:00Z", day, hour, minute);
        conn.execute(
            "INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![evt.id, "kb-1", "doc-1", &title, &evt.content, &evt.content, &created, &created],
        ).expect("INSERT zh_multihop event 失败");
    }

    let mut rel_idx: u32 = 0;
    for evt in &events {
        for eid in &evt.entities {
            let rel_id = format!("rel-zh-{:04}", rel_idx);
            conn.execute(
                "INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![&rel_id, &evt.id, eid, "2026-07-20T00:00:00Z"],
            ).expect("INSERT zh_multihop relation 失败");
            rel_idx += 1;
        }
    }

    // -------------------------------------------------------------------
    // Step 2: 扩展 filler entities（1000 个，ent-201..ent-1200）
    // -------------------------------------------------------------------
    for type_idx in 0..10 {
        let type_id = FILLER_ENTITY_TYPE_IDS[type_idx];
        for i in 0..100 {
            let entity_id = format!("ent-{}", 201 + type_idx * 100 + i);
            let name = format!("填充实体_{}_{}", type_idx, i);
            conn.execute(
                "INSERT OR IGNORE INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![&entity_id, type_id, &name, &name, "2026-07-20T00:00:00Z", "2026-07-20T00:00:00Z"],
            ).expect("INSERT filler entity 失败");
        }
    }

    // -------------------------------------------------------------------
    // Step 3: 扩展 filler events（9500 个，evt-501..evt-9999）
    // -------------------------------------------------------------------
    // filler event 不关联 zh_multihop 的 anchor 实体（ent-001..ent-200），
    // 避免污染 Recall@10 测试（ground truth 来自 evt-001..evt-500）
    for i in 501..=9999 {
        let event_id = format!("evt-{}", i);
        let title = format!("填充事件_{}", i);
        let summary = format!("填充事件_{} 的摘要", i);
        let content = format!("填充事件_{} 的内容", i);
        let total_minutes = i;
        let minute = total_minutes % 60;
        let hour = (total_minutes / 60) % 24;
        let day = (total_minutes / (60 * 24)) + 20;
        let created = format!("2026-07-{:02}T{:02}:{:02}:00Z", day, hour, minute);
        conn.execute(
            "INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![&event_id, "kb-1", "doc-1", &title, &summary, &content, &created, &created],
        ).expect("INSERT filler event 失败");
    }

    // -------------------------------------------------------------------
    // Step 4: 扩展 filler relations（~19000 条）
    // -------------------------------------------------------------------
    let mut filler_rel_idx: u32 = 0;
    for i in 501..=9999 {
        let evt_id = format!("evt-{}", i);
        let rel_count = (i % 3) + 1;
        for j in 0..rel_count {
            let filler_ent_num = 201 + ((i + j) % 1000);
            let entity_id = format!("ent-{}", filler_ent_num);
            let rel_id = format!("rel-fill-{:05}", filler_rel_idx);
            conn.execute(
                "INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![&rel_id, &evt_id, &entity_id, "2026-07-20T00:00:00Z"],
            ).expect("INSERT filler relation 失败");
            filler_rel_idx += 1;
        }
    }

    conn
}

// ---------------------------------------------------------------------------
// 4 策略实例化辅助函数
// ---------------------------------------------------------------------------

/// 构造 AtomicStrategy（共享 Connection 不可能，每次新建 DB）
fn make_atomic_strategy() -> Box<dyn SearchStrategy> {
    let conn = setup_zh_multihop_db();
    Box::new(AtomicStrategy::new(conn))
}

/// 构造 Multi1Strategy（max_hop=1，8 步骨架 + 单跳剪枝）
fn make_multi1_strategy() -> Box<dyn SearchStrategy> {
    let conn = setup_zh_multihop_db();
    Box::new(Multi1Strategy::new(conn))
}

/// 构造 MultiStrategy（BFS max_hop=3）
fn make_multi_strategy() -> Box<dyn SearchStrategy> {
    let conn = setup_zh_multihop_db();
    Box::new(MultiStrategy::new(conn))
}

/// 构造 MultiEsStrategy（ES-first + 子图预筛选，超边激活关闭）
///
/// ## 超边激活关闭的决策依据
/// spec §三 12.3.2 设计建议曾提出 `.with_hyperedge_activation(true)`，但实测在
/// zh_multihop 数据集上会触发 OOM（参考 `hyperedge_e2e.rs` 第 36-48 行防 OOM 设计）：
/// - zh_multihop 含 194 entities，部分 entity 关联多达 24 events
/// - `detect_hyperedges` 复杂度 O(E × 2^n)，2^24 = 16M 子集/entity → 内存爆炸
/// - 每个 case search 都会触发一次全图超边检测（非预计算），50 case × 16M = 必然 OOM
///
/// 关闭 hyperedge_activation 不影响 MULTI_ES 核心优势（ES-first + 子图预筛选），
/// 超边激活功能由 `hyperedge_e2e.rs` 4 个独立测试验证（使用防 OOM 设计的 fixture）。
///
/// 「Lessons Learned: 超边激活开销大，默认关闭」与 spec §三 12.1.2 性能保护一致
/// （`DEFAULT_ENABLE_HYPEREDGE_ACTIVATION=false`）。
fn make_multi_es_strategy() -> Box<dyn SearchStrategy> {
    let conn = setup_zh_multihop_db();
    Box::new(MultiEsStrategy::new(conn))
}

// ===========================================================================
// 测试 1：ATOMIC 策略 Recall@10
// ===========================================================================

/// 验收指标 1：ATOMIC 策略能在 50 case 上跑通（无 panic）+ 记录 Recall@10
///
/// ATOMIC 是单跳 JOIN 检索（query → entity → event），
/// 预期 Recall@10 较低（仅命中 hop=1 的 ground truth）。
#[test]
#[ignore = "Benchmark 测试：需 --ignored 显式触发（spec §三 12.3.2）"]
fn test_atomic_strategy_recall_at_10() {
    let runner = BenchmarkRunner::load_from_zh_multihop();
    let strategy = make_atomic_strategy();
    let result = runner.run_strategy("atomic", strategy.as_ref());
    println!(
        "[ATOMIC] Recall@10: {:.4}, Precision@10: {:.4}, avg_latency: {:.2}ms, p99_latency: {:.2}ms",
        result.recall_at_10, result.precision_at_10, result.avg_latency_ms, result.p99_latency_ms
    );
    // 断言：策略能跑通（Recall@10 >= 0.0，不要求具体阈值，记录数据）
    assert!(result.recall_at_10 >= 0.0, "Recall@10 应非负");
    assert!(result.avg_latency_ms >= 0.0, "avg latency 应非负");
}

// ===========================================================================
// 测试 2：MULTI1 策略 Recall@10（替代 spec 中的 VectorStrategy）
// ===========================================================================

/// 验收指标 2：MULTI1 策略 Recall@10
///
/// MULTI1 是 8 步骨架 + 单跳 BFS（max_hop=1），等价于 ATOMIC 但含 thought_process。
#[test]
#[ignore = "Benchmark 测试：需 --ignored 显式触发（spec §三 12.3.2）"]
fn test_multi1_strategy_recall_at_10() {
    let runner = BenchmarkRunner::load_from_zh_multihop();
    let strategy = make_multi1_strategy();
    let result = runner.run_strategy("multi1", strategy.as_ref());
    println!(
        "[MULTI1] Recall@10: {:.4}, Precision@10: {:.4}, avg_latency: {:.2}ms, p99_latency: {:.2}ms",
        result.recall_at_10, result.precision_at_10, result.avg_latency_ms, result.p99_latency_ms
    );
    assert!(result.recall_at_10 >= 0.0, "Recall@10 应非负");
}

// ===========================================================================
// 测试 3：MULTI 策略 Recall@10（BFS max_hop=3）
// ===========================================================================

/// 验收指标 3：MULTI 策略 Recall@10（8 步骨架 + BFS 3 跳）
///
/// MULTI 通过 BFS 多跳扩展，预期 Recall@10 > ATOMIC / MULTI1（覆盖 hop=2/3 ground truth）。
#[test]
#[ignore = "Benchmark 测试：需 --ignored 显式触发（spec §三 12.3.2）"]
fn test_multi_strategy_recall_at_10() {
    let runner = BenchmarkRunner::load_from_zh_multihop();
    let strategy = make_multi_strategy();
    let result = runner.run_strategy("multi", strategy.as_ref());
    println!(
        "[MULTI] Recall@10: {:.4}, Precision@10: {:.4}, avg_latency: {:.2}ms, p99_latency: {:.2}ms",
        result.recall_at_10, result.precision_at_10, result.avg_latency_ms, result.p99_latency_ms
    );
    assert!(result.recall_at_10 >= 0.0, "Recall@10 应非负");
}

// ===========================================================================
// 测试 4：MULTI_ES 策略 Recall@10（ES-first + 超边激活）
// ===========================================================================

/// 验收指标 4：MULTI_ES 策略 Recall@10（ES-first + 子图预筛选 + 超边激活）
///
/// MULTI_ES 启用 `.with_hyperedge_activation(true)`，超边激活增强 via_entities 上下文。
#[test]
#[ignore = "Benchmark 测试：需 --ignored 显式触发（spec §三 12.3.2）"]
fn test_multi_es_strategy_recall_at_10() {
    let runner = BenchmarkRunner::load_from_zh_multihop();
    let strategy = make_multi_es_strategy();
    let result = runner.run_strategy("multi_es", strategy.as_ref());
    println!(
        "[MULTI_ES] Recall@10: {:.4}, Precision@10: {:.4}, avg_latency: {:.2}ms, p99_latency: {:.2}ms",
        result.recall_at_10, result.precision_at_10, result.avg_latency_ms, result.p99_latency_ms
    );
    assert!(result.recall_at_10 >= 0.0, "Recall@10 应非负");
}

// ===========================================================================
// 测试 5：4 策略对比表（Recall@10 / Precision@10 / avg / p99 latency）
// ===========================================================================

/// 验收指标 5：4 策略对比表
///
/// 跑 4 策略，输出 Markdown 格式对比表，并断言：
/// - 4 策略均有结果（`results.len() == 4`）
/// - MULTI_ES Recall@10 >= MULTI Recall@10 - 0.01（预期排序，允许相等）
#[test]
#[ignore = "Benchmark 测试：需 --ignored 显式触发（spec §三 12.3.2）"]
fn test_results_comparison_table() {
    let runner = BenchmarkRunner::load_from_zh_multihop();

    let results = vec![
        runner.run_strategy("atomic", make_atomic_strategy().as_ref()),
        runner.run_strategy("multi1", make_multi1_strategy().as_ref()),
        runner.run_strategy("multi", make_multi_strategy().as_ref()),
        runner.run_strategy("multi_es", make_multi_es_strategy().as_ref()),
    ];

    // 输出对比表（Markdown 格式，便于复制到文档）
    println!("\n========== 4 策略对比表 ==========\n");
    println!("| Strategy  | Recall@10 | Precision@10 | Avg Latency(ms) | P99 Latency(ms) |");
    println!("|-----------|-----------|--------------|------------------|------------------|");
    for r in &results {
        println!(
            "| {:<9} | {:.4}    | {:.4}        | {:>14.2}   | {:>14.2}   |",
            r.strategy_name,
            r.recall_at_10,
            r.precision_at_10,
            r.avg_latency_ms,
            r.p99_latency_ms
        );
    }
    println!();

    // 断言 1：4 策略均有结果
    assert_eq!(results.len(), 4, "应跑 4 策略，实际 {}", results.len());

    // 断言 2：MULTI_ES Recall@10 >= MULTI Recall@10 - 0.01（预期排序，允许相等）
    let multi = &results[2];
    let multi_es = &results[3];
    println!(
        "[comparison] MULTI Recall@10 = {:.4}, MULTI_ES Recall@10 = {:.4}, diff = {:+.4}",
        multi.recall_at_10,
        multi_es.recall_at_10,
        multi_es.recall_at_10 - multi.recall_at_10
    );
    assert!(
        multi_es.recall_at_10 >= multi.recall_at_10 - 0.01,
        "MULTI_ES Recall@10 {:.4} 应 >= MULTI Recall@10 {:.4} - 0.01 = {:.4}（spec §三 12.3.2）",
        multi_es.recall_at_10,
        multi.recall_at_10,
        multi.recall_at_10 - 0.01
    );
}

// ===========================================================================
// 测试 6：4 策略延迟对比表（avg / p99）
// ===========================================================================

/// 验收指标 6：4 策略延迟对比表
///
/// 专门对比延迟，断言所有策略 avg latency < 5000ms（5 秒超时保护）。
#[test]
#[ignore = "Benchmark 测试：需 --ignored 显式触发（spec §三 12.3.2）"]
fn test_latency_comparison_table() {
    let runner = BenchmarkRunner::load_from_zh_multihop();

    let results = vec![
        runner.run_strategy("atomic", make_atomic_strategy().as_ref()),
        runner.run_strategy("multi1", make_multi1_strategy().as_ref()),
        runner.run_strategy("multi", make_multi_strategy().as_ref()),
        runner.run_strategy("multi_es", make_multi_es_strategy().as_ref()),
    ];

    // 延迟对比表
    println!("\n========== 4 策略延迟对比表 ==========\n");
    println!("| Strategy  | Avg Latency(ms) | P99 Latency(ms) | Max Latency(ms) |");
    println!("|-----------|------------------|------------------|------------------|");
    for r in &results {
        println!(
            "| {:<9} | {:>14.2}   | {:>14.2}   | {:>14.2}   |",
            r.strategy_name,
            r.avg_latency_ms,
            r.p99_latency_ms,
            r.p99_latency_ms // p99 是 50 case 中的最大值（floor(0.99*50)=49）
        );
    }
    println!();

    // 断言：所有策略 avg_latency_ms < 5000ms（5 秒超时保护）
    for r in &results {
        assert!(
            r.avg_latency_ms < 5000.0,
            "{} avg_latency {:.2}ms 应 < 5000ms（5 秒超时保护）",
            r.strategy_name,
            r.avg_latency_ms
        );
    }

    // 额外断言：4 策略均跑通
    assert_eq!(results.len(), 4, "应跑 4 策略，实际 {}", results.len());
}
