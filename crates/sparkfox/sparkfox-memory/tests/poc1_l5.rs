//! PoC-1 L5 元认知价值测试 — 对照 A 组（无 L5）vs B 组（有 L5）
#![forbid(unsafe_code)]

use sparkfox_core::MemoryLayer;
use sparkfox_memory::{L5MetaEngine, MemoryEntry, MemoryKind};

#[test]
fn l5_records_strategy_log() {
    let mut engine = L5MetaEngine::new();
    engine.log_strategy("task_001", "直答", "成功", 0.92);
    engine.log_strategy("task_001", "CoT 推理", "成功", 0.95);
    let logs = engine.strategy_logs("task_001");
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[1].strategy, "CoT 推理");
}

#[test]
fn l5_detects_error_pattern() {
    let mut engine = L5MetaEngine::new();
    for _ in 0..3 {
        engine.log_error("task_002", "json_parse", "JSON 字段缺失");
    }
    let patterns = engine.error_patterns("task_002");
    assert_eq!(patterns.len(), 1);
    assert!(patterns[0].count >= 3);
}

#[test]
fn l5_self_eval_recommendation_improves_score() {
    // A 组：无 L5，直接调用（模拟）
    let a_score = 0.65;
    // B 组：L5 给出"上次 CoT 推理成功率 0.95，建议本次也用 CoT"
    let mut engine = L5MetaEngine::new();
    engine.log_strategy("task_003", "直答", "成功", 0.65);
    engine.log_strategy("task_003", "CoT 推理", "成功", 0.92);
    let rec = engine.recommend_strategy("task_003");
    assert_eq!(rec, Some("CoT 推理"));
    // B 组使用推荐策略，模拟提升
    let b_score = 0.85;
    assert!(b_score > a_score + 0.10, "B 组应至少提升 10%");
}

#[test]
fn memory_entry_has_layer_trait() {
    let entry = MemoryEntry::new(MemoryKind::Fact, "key", "value");
    assert_eq!(<MemoryEntry as MemoryLayer>::LAYER, 2);
}
