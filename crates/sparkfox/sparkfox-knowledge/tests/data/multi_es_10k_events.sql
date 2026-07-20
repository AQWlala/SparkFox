-- Sub-Step 12.1.3 — MULTI_ES vs MULTI 性能对比 10k events fixture（spec §三 12.1.3）
--
-- 本文件是 10k events fixture 的「anchor 种子」：仅含 5 个 ground truth events +
-- 2 个 anchor entities + 9 条 anchor relations。测试代码（multi_es_vs_multi_perf_test.rs
-- 中的 `setup_10k_events_db()`）在加载本文件后，用 Rust 循环扩展到 10k events +
-- ~20k relations，避免在 git 仓库中提交 5MB+ 的全量 SQL（参考 multi_10k_events.sql 的
-- 5.6MB 镜像方案，本处采用「SQL 种子 + Rust 扩展」混合方案）。
--
-- ## fixture 拓扑（与 multi_e2e.rs::setup_10k_events_db 一致）
-- ```text
-- 张三 (ent-0-0) ── evt-0 ── 北京 (ent-1-0) ── evt-4
--                ── evt-1 ──/
--                ── evt-2 ──/
--                ── evt-3 ──/
--
-- evt-5..evt-9999 ── filler entities (ent-2-x .. ent-9-x，由 Rust 代码扩展)
-- ```
--
-- ## Recall@5 ground truth（query="张三"）
-- - evt-0..evt-3：张三直接关联（hop=1）
-- - evt-4：通过北京间接关联（hop=2，MULTI max_hop=3 会返回，MULTI1 max_hop=1 不返回）
--
-- ## MULTI_ES ES-first 适配
-- anchor 实体「张三」/「北京」是 jieba 默认词典可识别的中文实体名，
-- MULTI_ES 的 ES-first 直接用 query 作为 entity name LIKE 匹配（如 query="张三" →
-- name LIKE '%张三%' 命中 ent-0-0），跳过 jieba NER 抽取，加速检索。
--
-- ## 使用方式
-- ```rust,ignore
-- conn.execute_batch(ALL_SAG_DDL).unwrap();
-- conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES).unwrap();
-- conn.execute_batch(include_str!("data/multi_es_10k_events.sql")).unwrap();
-- // 然后 Rust 循环扩展 filler entities + events + relations
-- ```
--
-- ## License
-- AGPL-3.0-only

-- ============ anchor entities：2 行（张三 + 北京）============
-- 用 INSERT OR IGNORE 幂等，避免与测试代码扩展的 filler entities 冲突
INSERT OR IGNORE INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES ('ent-0-0', 'default_person', '张三', '张三', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO entity (id, entity_type_id, name, normalized_name, created_time, updated_time) VALUES ('ent-1-0', 'default_location', '北京', '北京', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z');

-- ============ anchor events：5 行（evt-0..evt-4，Recall@5 ground truth）============
INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES ('evt-0', 'kb-1', 'doc-1', '事件_0', '张三在北京的事件_0 摘要', '张三在北京的事件_0 内容', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES ('evt-1', 'kb-1', 'doc-1', '事件_1', '张三在北京的事件_1 摘要', '张三在北京的事件_1 内容', '2026-07-20T00:01:00Z', '2026-07-20T00:01:00Z');
INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES ('evt-2', 'kb-1', 'doc-1', '事件_2', '张三在北京的事件_2 摘要', '张三在北京的事件_2 内容', '2026-07-20T00:02:00Z', '2026-07-20T00:02:00Z');
INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES ('evt-3', 'kb-1', 'doc-1', '事件_3', '张三在北京的事件_3 摘要', '张三在北京的事件_3 内容', '2026-07-20T00:03:00Z', '2026-07-20T00:03:00Z');
INSERT OR IGNORE INTO knowledge_event (id, kb_id, doc_id, title, summary, content, created_time, updated_time) VALUES ('evt-4', 'kb-1', 'doc-1', '事件_4', '北京的事件_4 摘要', '北京的事件_4 内容', '2026-07-20T00:04:00Z', '2026-07-20T00:04:00Z');

-- ============ anchor relations：9 条 ============
-- evt-0..evt-3 → 张三 (ent-0-0) + 北京 (ent-1-0)：8 条（hop=1 ground truth）
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-seed-0', 'evt-0', 'ent-0-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-seed-1', 'evt-0', 'ent-1-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-seed-2', 'evt-1', 'ent-0-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-seed-3', 'evt-1', 'ent-1-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-seed-4', 'evt-2', 'ent-0-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-seed-5', 'evt-2', 'ent-1-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-seed-6', 'evt-3', 'ent-0-0', '2026-07-20T00:00:00Z');
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-seed-7', 'evt-3', 'ent-1-0', '2026-07-20T00:00:00Z');
-- evt-4 → 北京 (ent-1-0)：1 条（hop=2 ground truth，张三通过北京到达）
INSERT OR IGNORE INTO event_entity_relation (id, event_id, entity_id, created_time) VALUES ('rel-seed-8', 'evt-4', 'ent-1-0', '2026-07-20T00:00:00Z');
