//! Sub-Step 12.2.1 — 超边检测算法集成测试（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标（spec §三 12.2.1，5 测试）
//! 验证 [`HyperedgeDetector`](sparkfox_knowledge::hyperedge::HyperedgeDetector)
//! 在 >2 个 event 共享 >2 个 entity 时自动形成超边（Hyperedge）。
//!
//! 超边是 SAG（Semantic Agentic Graph）的核心创新：超越传统二元关系
//! （一条边只连接 2 个节点），允许 >2 个 event 通过共享 >2 个 entity
//! 形成一条多元超边，表达「多事件-多实体共现」的语义聚合。
//!
//! 5 个测试用例：
//! 1. `test_hyperedge_formed_when_3_events_share_3_entities`：
//!    3 event 共享 3 entity → 形成 1 条超边（基础情形）
//! 2. `test_no_hyperedge_when_only_2_events`：
//!    仅 2 event 共享 3 entity → 不形成超边（边界严格 >2）
//! 3. `test_no_hyperedge_when_only_2_entities`：
//!    3 event 共享仅 2 entity → 不形成超边（边界严格 >2）
//! 4. `test_hyperedge_contains_all_member_events`：
//!    超边含所有成员 events（3 event 全部包含）
//! 5. `test_hyperedge_contains_all_member_entities`：
//!    超边含所有成员 entities（3 entity 全部包含）
//!
//! ## 边界设计理由（>2 而非 >=2）
//! - **传统二元边**：连接 2 个节点（如 entity↔event），表达一对一关系
//! - **SAG 超边**：>2 个 event 共享 >2 个 entity，表达多对多共现
//! - 若阈值放宽到 >=2，则退化为普通二元边，丧失 SAG 创新意义
//! - 故严格 >2（即 ≥3）保证超边的「多元」语义
//!
//! ## Fixture 设计
//! - 内存版（`detect_from_relations`）：直接构造 `(event_id, entity_id)` 关系列表
//! - 测试 1/4/5：3 event × 3 entity = 9 关系（完全二分图 K_{3,3}）
//! - 测试 2：2 event × 3 entity = 6 关系（K_{2,3}）
//! - 测试 3：3 event × 2 entity = 6 关系（K_{3,2}）
//!
//! ## License
//! AGPL-3.0-only

#![forbid(unsafe_code)]

use sparkfox_knowledge::hyperedge::{Hyperedge, HyperedgeDetector};

// ---------------------------------------------------------------------------
// Fixture 辅助函数：构造完全二分图 K_{n_events, n_entities} 的关系列表
// ---------------------------------------------------------------------------

/// 构造 `n_events` × `n_entities` 完全二分图关系列表
///
/// - event_id 格式：`evt-0`, `evt-1`, ..., `evt-{n_events-1}`
/// - entity_id 格式：`ent-0`, `ent-1`, ..., `ent-{n_entities-1}`
/// - 每对 (event, entity) 都生成一条关系，共 n_events × n_entities 条
///
/// ## 示例
/// ```ignore
/// let relations = make_complete_bipartite(3, 3);
/// // 返回 9 条关系：("evt-0","ent-0"), ("evt-0","ent-1"), ..., ("evt-2","ent-2")
/// ```
fn make_complete_bipartite(n_events: usize, n_entities: usize) -> Vec<(String, String)> {
    let mut relations = Vec::with_capacity(n_events * n_entities);
    for evt_idx in 0..n_events {
        for ent_idx in 0..n_entities {
            relations.push((
                format!("evt-{}", evt_idx),
                format!("ent-{}", ent_idx),
            ));
        }
    }
    relations
}

// ---------------------------------------------------------------------------
// 5 个测试（spec §三 12.2.1 验收指标）
// ---------------------------------------------------------------------------

/// 验收指标 1：3 event 共享 3 entity → 自动形成 1 条超边（基础情形）
///
/// - Fixture：K_{3,3} 完全二分图（9 条关系）
/// - 期望：检测器返回 1 条超边
/// - 超边语义：3 个 event（evt-0/evt-1/evt-2）共同关联 3 个 entity（ent-0/ent-1/ent-2）
///
/// ## SAG 核心创新体现
/// 传统二元图：3 event × 3 entity = 9 条二元边（信息冗余）
/// SAG 超边：1 条超边聚合 9 条二元关系，表达「3 事件共现 3 实体」的多元语义
#[test]
fn test_hyperedge_formed_when_3_events_share_3_entities() {
    let detector = HyperedgeDetector::new();
    let relations = make_complete_bipartite(3, 3);

    let hyperedges = detector.detect_from_relations(&relations);

    assert_eq!(
        hyperedges.len(),
        1,
        "3 event 共享 3 entity 应形成 1 条超边，实际: {:?}",
        hyperedges
    );
}

/// 验收指标 2：仅 2 event 共享 3 entity → 不形成超边（边界严格 >2）
///
/// - Fixture：K_{2,3} 完全二分图（6 条关系）
/// - 期望：检测器返回 0 条超边
/// - 设计理由：超边阈值严格 >2（即 ≥3），2 event 退化为普通二元关系
///
/// ## 边界检查
/// 此测试确保 min_events 阈值正确生效：2 < 3（min_events），不形成超边。
#[test]
fn test_no_hyperedge_when_only_2_events() {
    let detector = HyperedgeDetector::new();
    let relations = make_complete_bipartite(2, 3);

    let hyperedges = detector.detect_from_relations(&relations);

    assert_eq!(
        hyperedges.len(),
        0,
        "仅 2 event 不应形成超边（边界 >2），实际: {:?}",
        hyperedges
    );
}

/// 验收指标 3：3 event 共享仅 2 entity → 不形成超边（边界严格 >2）
///
/// - Fixture：K_{3,2} 完全二分图（6 条关系）
/// - 期望：检测器返回 0 条超边
/// - 设计理由：超边阈值严格 >2（即 ≥3），2 entity 退化为普通二元关系
///
/// ## 边界检查
/// 此测试确保 min_entities 阈值正确生效：2 < 3（min_entities），不形成超边。
#[test]
fn test_no_hyperedge_when_only_2_entities() {
    let detector = HyperedgeDetector::new();
    let relations = make_complete_bipartite(3, 2);

    let hyperedges = detector.detect_from_relations(&relations);

    assert_eq!(
        hyperedges.len(),
        0,
        "仅 2 entity 不应形成超边（边界 >2），实际: {:?}",
        hyperedges
    );
}

/// 验收指标 4：超边含所有成员 events（3 event 全部包含）
///
/// - Fixture：K_{3,3} 完全二分图（9 条关系）
/// - 期望：1 条超边，其 `member_events` 包含 evt-0 / evt-1 / evt-2 全部 3 个 event
///
/// ## 超边完整性
/// 超边的 `member_events` 必须列出所有参与超边的 event（无遗漏）。
/// 这是 SAG 超边的核心契约：超边 = (events 集合, entities 集合) 的多元关系。
#[test]
fn test_hyperedge_contains_all_member_events() {
    let detector = HyperedgeDetector::new();
    let relations = make_complete_bipartite(3, 3);

    let hyperedges = detector.detect_from_relations(&relations);

    assert_eq!(hyperedges.len(), 1, "应形成 1 条超边");
    let he: &Hyperedge = &hyperedges[0];

    assert_eq!(
        he.member_events.len(),
        3,
        "超边应含 3 个 member_events，实际: {:?}",
        he.member_events
    );

    // 验证 3 个 event 全部包含
    for k in 0..3 {
        let expected_evt = format!("evt-{}", k);
        assert!(
            he.member_events.contains(&expected_evt),
            "超边 member_events 应包含 {}，实际: {:?}",
            expected_evt,
            he.member_events
        );
    }
}

/// 验收指标 5：超边含所有成员 entities（3 entity 全部包含）
///
/// - Fixture：K_{3,3} 完全二分图（9 条关系）
/// - 期望：1 条超边，其 `member_entities` 包含 ent-0 / ent-1 / ent-2 全部 3 个 entity
///
/// ## 超边完整性
/// 超边的 `member_entities` 必须列出所有参与超边的 entity（无遗漏）。
/// 与 `member_events` 共同构成超边的多元关系语义。
#[test]
fn test_hyperedge_contains_all_member_entities() {
    let detector = HyperedgeDetector::new();
    let relations = make_complete_bipartite(3, 3);

    let hyperedges = detector.detect_from_relations(&relations);

    assert_eq!(hyperedges.len(), 1, "应形成 1 条超边");
    let he: &Hyperedge = &hyperedges[0];

    assert_eq!(
        he.member_entities.len(),
        3,
        "超边应含 3 个 member_entities，实际: {:?}",
        he.member_entities
    );

    // 验证 3 个 entity 全部包含
    for k in 0..3 {
        let expected_ent = format!("ent-{}", k);
        assert!(
            he.member_entities.contains(&expected_ent),
            "超边 member_entities 应包含 {}，实际: {:?}",
            expected_ent,
            he.member_entities
        );
    }
}
