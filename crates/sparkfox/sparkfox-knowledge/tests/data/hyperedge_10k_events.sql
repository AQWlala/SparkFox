-- Sub-Step 12.2.4 — 动态超边 E2E 测试 10k events fixture（spec §三 12.2.4）
--
-- 本文件是动态超边 E2E 测试的「anchor 种子」：仅含形成超边所需的 3 anchor events +
-- 3 anchor entities + 9 anchor relations（K_{3,3} 完全二分图），以及若干扩展 anchor
-- events 用于验证 Recall@5 不退化。测试代码（hyperedge_e2e.rs 中的
-- `setup_hyperedge_10k_events_db()`）在加载本文件后，用 Rust 循环扩展到 10k events +
-- ~20k relations，避免在 git 仓库中提交 5MB+ 的全量 SQL（参考 multi_es_10k_events.sql
-- 的「SQL 种子 + Rust 扩展」混合方案）。
--
-- ## fixture 拓扑（K_{3,3} 完全二分图 → 1 条超边）
-- ```text
--                ┌── evt-he-0 ──┐
--   张三 (ent-0-0) ── evt-he-1 ── 北京 (ent-1-0)
--                └── evt-he-2 ──┘ 腾讯 (ent-2-0)
--
-- 每个 evt-he-* 关联全部 3 个 anchor entity（K_{3,3}）→ 自动形成 1 条超边
--   hyperedge-1 = {evt-he-0, evt-he-1, evt-he-2} ↔ {ent-0-0, ent-1-0, ent-2-0}
--
-- evt-anchor-extra-{0..2} ── 仅关联 1 个 anchor entity（不形成超边，用于 hop=1 ground truth）
-- evt-filler-0..evt-filler-9990 ── 关联 filler entities（由 Rust 代码扩展，不形成超边）
-- ```
--
-- ## 超边场景设计
-- - **K_{3,3} 完全二分图**：3 events × 3 entities = 9 relations，自动形成 1 条超边
--   （满足 min_events=3 AND min_entities=3 的严格 >2 阈值）
-- - **anchor extra events**：3 个仅关联 1 个 anchor entity 的 events，不形成超边
--   （用于验证非超边 events 不被误激活）
-- - **filler entities / events**：Rust 扩展 1000 个 filler entities + 9991 filler events
--   （不与 anchor entities 形成超边，避免污染超边检测测试）
--
-- ## Recall@5 ground truth（query="张三"）
-- - evt-he-0..evt-he-2：通过超边激活返回（hyperedge activation）
-- - evt-anchor-extra-0..2：通过 hop=1 BFS 返回（直接关联 ent-0-0）
-- - 共 6 个 ground truth events
--
-- ## MULTI_ES ES-first 适配
-- anchor 实体「张三」是 jieba 默认词典可识别的中文实体名，
-- MULTI_ES 的 ES-first 直接用 query 作为 entity name LIKE 匹配（如 query="张三" →
-- name LIKE '%张三%' 命中 ent-0-0），跳过 jieba NER 抽取，加速检索。
-- 命中 ent-0-0 后，超边激活机制激活整条超边，返回全部 3 个 evt-he-*。
--
-- ## 使用方式
-- ```rust,ignore
-- conn.execute_batch(ALL_SAG_DDL).unwrap();
-- conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();
-- conn.execute_batch(include_str!("data/hyperedge_10k_events.sql")).unwrap();
-- // 然后 Rust 循环扩展 filler entities + events + relations
-- ```
--
-- ## License
-- AGPL-3.0-only

-- ============ anchor entities：3 行（张三 + 北京 + 腾讯，构成超边成员）============
-- 用 INSERT OR IGNORE 幂等，避免与测试代码扩展的 filler entities 冲突
INSERT OR IGNORE INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES ('ent-0-0', 'default_person', '张三', '张三', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES ('ent-1-0', 'default_location', '北京', '北京', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES ('ent-2-0', 'default_organization', '腾讯', '腾讯', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z');

-- ============ anchor hyperedge events：3 行（evt-he-0..evt-he-2，构成超边成员）============
-- 这 3 个 event 全部关联 3 个 anchor entity（K_{3,3}），自动形成 1 条超边
INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES ('evt-he-0', 'kb-1', 'doc-1', '超边事件_0', '张三在北京参加腾讯的超边事件_0 摘要', '张三在北京参加腾讯的超边事件_0 内容', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES ('evt-he-1', 'kb-1', 'doc-1', '超边事件_1', '张三在北京参加腾讯的超边事件_1 摘要', '张三在北京参加腾讯的超边事件_1 内容', '2026-07-20T00:01:00Z', '2026-07-20T00:01:00Z');
INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES ('evt-he-2', 'kb-1', 'doc-1', '超边事件_2', '张三在北京参加腾讯的超边事件_2 摘要', '张三在北京参加腾讯的超边事件_2 内容', '2026-07-20T00:02:00Z', '2026-07-20T00:02:00Z');

-- ============ anchor extra events：3 行（仅关联 1 个 anchor entity，不形成超边）============
-- 用于验证非超边 events 不被误激活（hop=1 BFS 直接关联 ent-0-0）
INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES ('evt-anchor-extra-0', 'kb-1', 'doc-1', '扩展事件_0', '张三的扩展事件_0 摘要', '张三的扩展事件_0 内容', '2026-07-20T00:03:00Z', '2026-07-20T00:03:00Z');
INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES ('evt-anchor-extra-1', 'kb-1', 'doc-1', '扩展事件_1', '张三的扩展事件_1 摘要', '张三的扩展事件_1 内容', '2026-07-20T00:04:00Z', '2026-07-20T00:04:00Z');
INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES ('evt-anchor-extra-2', 'kb-1', 'doc-1', '扩展事件_2', '张三的扩展事件_2 摘要', '张三的扩展事件_2 内容', '2026-07-20T00:05:00Z', '2026-07-20T00:05:00Z');

-- ============ anchor relations：12 条 ============
-- K_{3,3} 完全二分图：evt-he-0..evt-he-2 × ent-0-0/ent-1-0/ent-2-0 = 9 条（超边 ground truth）
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-he-0', 'evt-he-0', 'ent-0-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-he-1', 'evt-he-0', 'ent-1-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-he-2', 'evt-he-0', 'ent-2-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-he-3', 'evt-he-1', 'ent-0-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-he-4', 'evt-he-1', 'ent-1-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-he-5', 'evt-he-1', 'ent-2-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-he-6', 'evt-he-2', 'ent-0-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-he-7', 'evt-he-2', 'ent-1-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-he-8', 'evt-he-2', 'ent-2-0', '2026-07-20T00:00:00Z');

-- evt-anchor-extra-0..2 → ent-0-0：3 条（hop=1 ground truth，仅张三直接关联）
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-extra-0', 'evt-anchor-extra-0', 'ent-0-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-extra-1', 'evt-anchor-extra-1', 'ent-0-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-extra-2', 'evt-anchor-extra-2', 'ent-0-0', '2026-07-20T00:00:00Z');
