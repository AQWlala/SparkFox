//! Sub-Step 10.2.4 — EventSaver 测试（TDD-RED → GREEN → REFACTOR）
//!
//! ## 测试目标
//! 1. `save()` 写入 `knowledge_event` 表（title/summary/content/category/keywords 字段正确）
//! 2. `save()` 写入 `entity` 表（name/normalized_name/entity_type_id 正确）
//! 3. `save()` 写入 `event_entity_relation` 表（每对 event-entity 1 行）
//! 4. `EntityNormalizer` trait 可注入：自定义 MockNormalizer 影响 normalized_name
//! 5. 事务原子性：部分失败时 ROLLBACK，`knowledge_event` 表无残留写入
//! 6. entity 归一化去重：相同 (entity_type_id, normalized_name) 复用 entity_id
//!
//! ## TDD-RED 说明
//! 本测试在 GREEN 实现前应全部失败（saver 模块尚未创建，编译错误）。
//!
//! ## 设计参考
//! - `docs/SparkFox-v1.1.0-规划.md` Sub-Step 10.2.4
//! - SAG 论文 Chunk → Event → Entity 持久化流程

#![forbid(unsafe_code)]

use std::sync::Arc;

use rusqlite::{Connection, params};

use sparkfox_knowledge::extractor::{EntityMention, EventCandidate};
use sparkfox_knowledge::saver::{
    DefaultEntityNormalizer, EntityNormalizer, EventSaver,
};
use sparkfox_knowledge::schema::{ALL_SAG_DDL, INSERT_DEFAULT_ENTITY_TYPES};

// ---------------------------------------------------------------------------
// 测试辅助：构造内存数据库 + EventCandidate / EntityMention
// ---------------------------------------------------------------------------

/// 构造测试用内存数据库（开启 FK + 预填 11 种默认实体类型）
fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("打开内存数据库失败");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("开启 foreign_keys 失败");
    for ddl in ALL_SAG_DDL {
        conn.execute_batch(ddl).expect("执行 SAG DDL 失败");
    }
    conn.execute_batch(INSERT_DEFAULT_ENTITY_TYPES)
        .expect("预填默认实体类型失败");
    conn
}

/// 构造测试用 EventCandidate（含给定 title 与 entities 列表）
fn make_candidate(title: &str, entities: Vec<EntityMention>) -> EventCandidate {
    EventCandidate {
        title: title.to_string(),
        summary: format!("{title}摘要"),
        content: format!("{title}完整内容"),
        category: Some("测试".to_string()),
        keywords: vec!["测试".to_string()],
        entities,
    }
}

/// 构造测试用 EntityMention（start=0, end=text 长度）
fn make_entity(entity_type: &str, text: &str) -> EntityMention {
    EntityMention {
        entity_type: entity_type.to_string(),
        text: text.to_string(),
        start: 0,
        end: text.len(),
    }
}

// ---------------------------------------------------------------------------
// 测试 1：写入 knowledge_event 表（字段映射正确）
// ---------------------------------------------------------------------------

/// 验证 `EventSaver::save()` 将单个 EventCandidate 写入 `knowledge_event` 表，
/// 字段映射：title / summary / content / category / keywords（JSON 数组字符串）正确。
#[test]
fn test_saver_writes_event_to_knowledge_event_table() {
    let conn = setup_db();
    let saver = EventSaver::new(
        conn,
        "kb-test-1".to_string(),
        "doc-test-1".to_string(),
    );
    let candidate = make_candidate(
        "张三出差",
        vec![make_entity("PERSON", "张三")],
    );

    let stats = saver
        .save(vec![candidate])
        .expect("单个 EventCandidate 应保存成功");

    assert_eq!(stats.events_written, 1, "events_written 应为 1");
    assert_eq!(stats.entities_written, 1, "entities_written 应为 1");
    assert_eq!(stats.relations_written, 1, "relations_written 应为 1");

    // 直接查询 knowledge_event 表验证字段
    let row: (String, String, String, Option<String>, String, String, String) = saver
        .conn()
        .query_row(
            "SELECT title, summary, content, category, keywords, kb_id, doc_id \
             FROM knowledge_event WHERE title = ?",
            params!["张三出差"],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .expect("查询 knowledge_event 失败");

    assert_eq!(row.0, "张三出差", "title 字段错误");
    assert_eq!(row.1, "张三出差摘要", "summary 字段错误");
    assert_eq!(row.2, "张三出差完整内容", "content 字段错误");
    assert_eq!(row.3.as_deref(), Some("测试"), "category 字段错误");
    // keywords 应为 JSON 数组字符串：["测试"]
    assert_eq!(
        row.4, "[\"测试\"]",
        "keywords 字段应为 JSON 数组字符串，实际: {}",
        row.4
    );
    assert_eq!(row.5, "kb-test-1", "kb_id 字段错误");
    assert_eq!(row.6, "doc-test-1", "doc_id 字段错误");
}

// ---------------------------------------------------------------------------
// 测试 2：写入 entity 表（name / normalized_name / entity_type_id）
// ---------------------------------------------------------------------------

/// 验证 `EventSaver::save()` 将 EventCandidate 中的 2 个 EntityMention
/// （PERSON "张三" / LOCATION "北京"）写入 `entity` 表，
/// 字段映射：name / normalized_name（经 DefaultEntityNormalizer 处理为 "张三" / "北京"）
/// / entity_type_id（"default_person" / "default_location"）正确。
#[test]
fn test_saver_writes_entity_to_entity_table() {
    let conn = setup_db();
    let saver = EventSaver::new(
        conn,
        "kb-test-2".to_string(),
        "doc-test-2".to_string(),
    );
    let candidate = make_candidate(
        "出差事件",
        vec![
            make_entity("PERSON", "张三"),
            make_entity("LOCATION", "北京"),
        ],
    );

    let stats = saver.save(vec![candidate]).expect("保存应成功");

    assert_eq!(stats.events_written, 1, "events_written 应为 1");
    assert_eq!(stats.entities_written, 2, "entities_written 应为 2");
    assert_eq!(stats.relations_written, 2, "relations_written 应为 2");

    // 查询 entity 表中 name='张三' 的行
    let person_row: (String, String, String) = saver
        .conn()
        .query_row(
            "SELECT name, normalized_name, entity_type_id FROM entity WHERE name = ?",
            params!["张三"],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("查询 entity (张三) 失败");

    assert_eq!(person_row.0, "张三", "name 字段错误");
    assert_eq!(person_row.1, "张三", "normalized_name 字段错误（应为 '张三'）");
    assert_eq!(
        person_row.2, "default_person",
        "entity_type_id 应为 'default_person'，实际: {}",
        person_row.2
    );

    // 查询 entity 表中 name='北京' 的行
    let location_row: (String, String, String) = saver
        .conn()
        .query_row(
            "SELECT name, normalized_name, entity_type_id FROM entity WHERE name = ?",
            params!["北京"],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("查询 entity (北京) 失败");

    assert_eq!(location_row.0, "北京", "name 字段错误");
    assert_eq!(location_row.1, "北京", "normalized_name 字段错误（应为 '北京'）");
    assert_eq!(
        location_row.2, "default_location",
        "entity_type_id 应为 'default_location'，实际: {}",
        location_row.2
    );
}

// ---------------------------------------------------------------------------
// 测试 3：写入 event_entity_relation 表（每对 event-entity 1 行）
// ---------------------------------------------------------------------------

/// 验证 `EventSaver::save()` 为每对 (event, entity) 写入 1 行到
/// `event_entity_relation` 表，且 event_id / entity_id 关联正确。
#[test]
fn test_saver_writes_relation_to_event_entity_relation_table() {
    let conn = setup_db();
    let saver = EventSaver::new(
        conn,
        "kb-test-3".to_string(),
        "doc-test-3".to_string(),
    );
    let candidate = make_candidate(
        "事件 R",
        vec![
            make_entity("PERSON", "张三"),
            make_entity("LOCATION", "北京"),
        ],
    );

    let stats = saver.save(vec![candidate]).expect("保存应成功");

    assert_eq!(stats.relations_written, 2, "应有 2 行 relation 记录");

    // 查询 relation 表行数
    let relation_count: i64 = saver
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation",
            params![],
            |r| r.get(0),
        )
        .expect("查询 relation 行数失败");

    assert_eq!(
        relation_count, 2,
        "relation 表应有 2 行，实际 {}",
        relation_count
    );

    // 验证每行 relation 的 event_id 在 knowledge_event 表中存在（无孤儿）
    let orphan_event_count: i64 = saver
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation r \
             LEFT JOIN knowledge_event e ON r.event_id = e.id \
             WHERE e.id IS NULL",
            params![],
            |r| r.get(0),
        )
        .expect("查询孤儿 event_id 失败");

    assert_eq!(
        orphan_event_count, 0,
        "存在 {} 个孤儿 event_id（应为 0）",
        orphan_event_count
    );

    // 验证每行 relation 的 entity_id 在 entity 表中存在（无孤儿）
    let orphan_entity_count: i64 = saver
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation r \
             LEFT JOIN entity en ON r.entity_id = en.id \
             WHERE en.id IS NULL",
            params![],
            |r| r.get(0),
        )
        .expect("查询孤儿 entity_id 失败");

    assert_eq!(
        orphan_entity_count, 0,
        "存在 {} 个孤儿 entity_id（应为 0）",
        orphan_entity_count
    );
}

// ---------------------------------------------------------------------------
// 测试 4：注入自定义 EntityNormalizer（验证 normalize 被调用）
// ---------------------------------------------------------------------------

/// 测试用 Mock EntityNormalizer — 固定返回 "MOCK_NORMALIZED"
struct MockNormalizer;

impl EntityNormalizer for MockNormalizer {
    fn normalize(&self, _entity_type: &str, _text: &str) -> String {
        "MOCK_NORMALIZED".to_string()
    }
}

/// 验证注入自定义 `EntityNormalizer` 后，`save()` 调用 `normalizer.normalize()`
/// 处理 entity 文本，写入 `entity` 表的 `normalized_name` 列全部为 Mock 返回值。
#[test]
fn test_saver_calls_entity_normalizer_before_save() {
    let conn = setup_db();
    let saver = EventSaver::with_normalizer(
        conn,
        "kb-test-4".to_string(),
        "doc-test-4".to_string(),
        Arc::new(MockNormalizer),
    );
    let candidate = make_candidate(
        "事件 N",
        vec![
            make_entity("PERSON", "张三"),
            make_entity("LOCATION", "北京"),
        ],
    );

    let stats = saver.save(vec![candidate]).expect("保存应成功");

    assert_eq!(stats.entities_written, 2, "应有 2 个 entity 写入");

    // 查询 entity 表中 normalized_name='MOCK_NORMALIZED' 的行数
    let mock_count: i64 = saver
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM entity WHERE normalized_name = ?",
            params!["MOCK_NORMALIZED"],
            |r| r.get(0),
        )
        .expect("查询 MOCK_NORMALIZED 行数失败");

    assert_eq!(
        mock_count, 2,
        "应全部 2 行的 normalized_name 为 'MOCK_NORMALIZED'，实际 {} 行",
        mock_count
    );
}

// ---------------------------------------------------------------------------
// 测试 5：事务原子性 — 部分失败时 ROLLBACK，knowledge_event 表无残留写入
// ---------------------------------------------------------------------------

/// 验证 `save()` 在事务中执行，部分 INSERT 失败时 ROLLBACK，
/// `knowledge_event` 表不应有任何残留写入。
///
/// 触发失败方式：构造 entity_type="UNKNOWN_TYPE" 的 EntityMention，
/// 该类型不在 `entity_type` 表中（11 种默认类型之外），
/// `INSERT INTO entity` 时 FK 约束失败 → 触发 ROLLBACK。
#[test]
fn test_saver_transaction_rollback_on_partial_failure() {
    let conn = setup_db();
    let saver = EventSaver::new(
        conn,
        "kb-test-5".to_string(),
        "doc-test-5".to_string(),
    );
    // candidate 含未知 entity_type="UNKNOWN_TYPE"（不在 entity_type 表中）
    let candidate = make_candidate(
        "会回滚的事件",
        vec![
            make_entity("PERSON", "张三"),
            make_entity("UNKNOWN_TYPE", "未知实体"),
        ],
    );

    let save_result = saver.save(vec![candidate]);

    assert!(
        save_result.is_err(),
        "save() 应返回 Err（FK 约束失败），实际 Ok"
    );

    // 验证 knowledge_event 表无任何写入（事务已回滚）
    let event_count: i64 = saver
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM knowledge_event",
            params![],
            |r| r.get(0),
        )
        .expect("查询 knowledge_event 行数失败");

    assert_eq!(
        event_count, 0,
        "事务回滚后 knowledge_event 表应为空，实际 {} 行",
        event_count
    );

    // 同时验证 entity 表也无写入（事务回滚）
    let entity_count: i64 = saver
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM entity",
            params![],
            |r| r.get(0),
        )
        .expect("查询 entity 行数失败");

    assert_eq!(
        entity_count, 0,
        "事务回滚后 entity 表应为空，实际 {} 行",
        entity_count
    );
}

// ---------------------------------------------------------------------------
// 测试 6：entity 归一化去重（相同 normalized_name 复用 entity_id）
// ---------------------------------------------------------------------------

/// 验证 2 个 EventCandidate 共享相同 normalized_name 的 entity（如都含 "张三"）时，
/// `save()` 仅在 `entity` 表写入 1 行 "张三"，`event_entity_relation` 表写入 2 行
/// （分别关联 2 个 event 到同一个 entity_id）。
#[test]
fn test_saver_deduplicates_entities_by_normalized_id() {
    let conn = setup_db();
    let saver = EventSaver::new(
        conn,
        "kb-test-6".to_string(),
        "doc-test-6".to_string(),
    );
    // 2 个 EventCandidate，都含 entity_type="PERSON" text="张三"
    let candidates = vec![
        make_candidate("事件 A", vec![make_entity("PERSON", "张三")]),
        make_candidate("事件 B", vec![make_entity("PERSON", "张三")]),
    ];

    let stats = saver.save(candidates).expect("保存应成功");

    assert_eq!(stats.events_written, 2, "events_written 应为 2");
    assert_eq!(
        stats.entities_written, 1,
        "entities_written 应为 1（去重），实际 {}",
        stats.entities_written
    );
    assert_eq!(
        stats.entities_deduplicated, 1,
        "entities_deduplicated 应为 1（第 2 个 '张三' 去重），实际 {}",
        stats.entities_deduplicated
    );
    assert_eq!(stats.relations_written, 2, "relations_written 应为 2");

    // 验证 entity 表中 name='张三' 的行数 = 1
    let person_count: i64 = saver
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM entity WHERE name = ?",
            params!["张三"],
            |r| r.get(0),
        )
        .expect("查询 entity (张三) 行数失败");

    assert_eq!(
        person_count, 1,
        "entity 表应仅有 1 行 '张三'（去重），实际 {} 行",
        person_count
    );

    // 验证 event_entity_relation 表中关联到 '张三' 的 relation 行数 = 2
    let relation_count: i64 = saver
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM event_entity_relation r \
             JOIN entity e ON r.entity_id = e.id \
             WHERE e.name = ?",
            params!["张三"],
            |r| r.get(0),
        )
        .expect("查询关联 '张三' 的 relation 行数失败");

    assert_eq!(
        relation_count, 2,
        "应 2 行 relation 关联到同一个 '张三'，实际 {} 行",
        relation_count
    );

    // 验证 2 行 relation 关联到不同的 event_id
    let distinct_event_count: i64 = saver
        .conn()
        .query_row(
            "SELECT COUNT(DISTINCT r.event_id) FROM event_entity_relation r \
             JOIN entity e ON r.entity_id = e.id \
             WHERE e.name = ?",
            params!["张三"],
            |r| r.get(0),
        )
        .expect("查询 distinct event_id 失败");

    assert_eq!(
        distinct_event_count, 2,
        "2 行 relation 应关联到 2 个不同的 event_id，实际 {} 个",
        distinct_event_count
    );
}

// ---------------------------------------------------------------------------
// 辅助：测试 DefaultEntityNormalizer 的基本行为（trim + lowercase）
// ---------------------------------------------------------------------------

/// 验证 `DefaultEntityNormalizer` 对中文/英文/混合文本执行 trim + lowercase。
#[test]
fn test_default_entity_normalizer_trims_and_lowercases() {
    let normalizer = DefaultEntityNormalizer;

    // 中文文本不受 lowercase 影响
    assert_eq!(normalizer.normalize("PERSON", "张三"), "张三");
    // 前后空白被 trim
    assert_eq!(normalizer.normalize("PERSON", "  张三  "), "张三");
    // 英文大写转小写
    assert_eq!(normalizer.normalize("PERSON", "  John DOE  "), "john doe");
    // 混合：英文转小写 + 中文保留 + trim
    assert_eq!(
        normalizer.normalize("PERSON", "  Alice 与 张三  "),
        "alice 与 张三"
    );
}
