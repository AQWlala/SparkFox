//! SAG schema DDL — 6 张表定义（P-03/P-04 P0 修复）
//!
//! 字段名与字段类型严格对齐 spec 2.0 §六 6.4（SAG 重构方案-七专家评审-1.0.md）。
//! 表结构来源：SAG 论文 + SAG-Benchmark (MIT License, verified 2026-07-19)。
//! 字段重命名基于 MIT 许可的 schema 借鉴（非清洁室重写，见 NOTICE）。
//!
//! 与 spec §6.4 的有意偏差（避免引用尚不存在的表，FK 校验在 foreign_keys=ON 下会拒绝写入）：
//! - `knowledge_event` 移除对 `knowledge_base` / `kb_document` / `knowledge_chunk` 的 FK 约束
//!   （这些表尚未在 sparkfox-store 中创建；列保留以便未来迁移补 FK）
//! - `knowledge_event.chunk_id` 由 NOT NULL 改为可空（无对应表时允许 NULL）
//! - `entity_type` 移除对 `source_config` / `article` 的 FK 约束
//! - `entity` 移除对 `source_config` 的 FK 约束
//! 表间 FK（knowledge_event 自引用 / entity→entity_type / event_entity_relation→
//! knowledge_event+entity / event_entity_embedding→knowledge_event+entity）保留不变。

/// 1. knowledge_event 表 — SAG 事件表（L3 Episodic）
///
/// spec §6.4 DDL 原样字段；FK 到 knowledge_base/kb_document/knowledge_chunk 移除
/// （目标表尚未存在，foreign_keys=ON 下会拒绝 INSERT）。
pub const DDL_KNOWLEDGE_EVENT: &str = r#"
CREATE TABLE IF NOT EXISTS knowledge_event (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    doc_id TEXT NOT NULL,
    chunk_id TEXT,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    content TEXT NOT NULL,
    category TEXT,
    keywords TEXT,
    rank INTEGER NOT NULL DEFAULT 0,
    level INTEGER NOT NULL DEFAULT 0,
    parent_id TEXT,
    start_time TEXT,
    end_time TEXT,
    status TEXT NOT NULL DEFAULT 'COMPLETED',
    sync_date TEXT,
    extra_data TEXT,
    created_time TEXT NOT NULL,
    updated_time TEXT NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES knowledge_event(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_event_kb_doc ON knowledge_event(kb_id, doc_id);
CREATE INDEX IF NOT EXISTS idx_event_chunk ON knowledge_event(chunk_id);
CREATE INDEX IF NOT EXISTS idx_event_rank ON knowledge_event(kb_id, rank);
CREATE INDEX IF NOT EXISTS idx_event_category ON knowledge_event(kb_id, category);
CREATE INDEX IF NOT EXISTS idx_event_time ON knowledge_event(start_time, end_time);
"#;

/// 2. entity_type 表 — 实体类型字典
///
/// spec §6.4 DDL 原样字段；FK 到 source_config/article 移除（目标表尚未存在）。
pub const DDL_ENTITY_TYPE: &str = r#"
CREATE TABLE IF NOT EXISTS entity_type (
    id TEXT PRIMARY KEY,
    scope TEXT NOT NULL DEFAULT 'global',
    source_config_id TEXT,
    article_id TEXT,
    type TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    weight REAL NOT NULL DEFAULT 1.0,
    similarity_threshold REAL NOT NULL DEFAULT 0.8,
    is_active INTEGER NOT NULL DEFAULT 1,
    is_default INTEGER NOT NULL DEFAULT 0,
    value_format TEXT,
    value_constraints TEXT,
    extra_data TEXT,
    created_time TEXT NOT NULL,
    updated_time TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_entity_type_scope ON entity_type(scope, source_config_id, article_id, type);
CREATE INDEX IF NOT EXISTS idx_entity_type_default ON entity_type(is_default, is_active);
"#;

/// 3. entity 表 — SAG 实体表（L3 Semantic + L3 GraphNode）
///
/// spec §6.4 DDL 原样字段；FK 到 source_config 移除（目标表尚未存在）。
pub const DDL_ENTITY: &str = r#"
CREATE TABLE IF NOT EXISTS entity (
    id TEXT PRIMARY KEY,
    source_config_id TEXT,
    entity_type_id TEXT NOT NULL,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    int_value INTEGER,
    float_value REAL,
    datetime_value TEXT,
    bool_value INTEGER,
    enum_value TEXT,
    value_unit TEXT,
    description TEXT,
    extra_data TEXT,
    created_time TEXT NOT NULL,
    updated_time TEXT NOT NULL,
    FOREIGN KEY (entity_type_id) REFERENCES entity_type(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_entity_type ON entity(entity_type_id);
CREATE INDEX IF NOT EXISTS idx_entity_normalized ON entity(normalized_name);
"#;

/// 4. event_entity_relation 表 — SAG 关联表（L3 GraphEdge，含双向索引 P-01）
///
/// spec §6.4 DDL 原样字段；P-01 双向复合索引在表创建后追加。
pub const DDL_EVENT_ENTITY_RELATION: &str = r#"
CREATE TABLE IF NOT EXISTS event_entity_relation (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    relation_type TEXT,
    confidence REAL NOT NULL DEFAULT 1.0,
    extra_data TEXT,
    created_time TEXT NOT NULL,
    FOREIGN KEY (event_id) REFERENCES knowledge_event(id) ON DELETE CASCADE,
    FOREIGN KEY (entity_id) REFERENCES entity(id) ON DELETE CASCADE
);
-- 【P-01 P0 修复】双向复合索引（SAG 原版仅有 entity_id→event_id 单向索引）
CREATE INDEX IF NOT EXISTS idx_eer_event_entity ON event_entity_relation(event_id, entity_id);  -- 正向
CREATE INDEX IF NOT EXISTS idx_eer_entity_event ON event_entity_relation(entity_id, event_id);  -- 反向
"#;

/// 5. event_entity_embedding 表 — SAG 关联级嵌入（A-04/P-04 P0 修复，L3 EventEntity）
///
/// spec §6.4 DDL 原样字段；SAG 核心创新 — 每个 (event, entity) 对的嵌入向量。
pub const DDL_EVENT_ENTITY_EMBEDDING: &str = r#"
CREATE TABLE IF NOT EXISTS event_entity_embedding (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    model TEXT NOT NULL,
    embedding BLOB NOT NULL,
    created_time TEXT NOT NULL,
    FOREIGN KEY (event_id) REFERENCES knowledge_event(id) ON DELETE CASCADE,
    FOREIGN KEY (entity_id) REFERENCES entity(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_eee_event_entity ON event_entity_embedding(event_id, entity_id);
CREATE INDEX IF NOT EXISTS idx_eee_model ON event_entity_embedding(model);
"#;

/// 6. llm_audit_log 表 — LLM 调用审计日志（S-01 P0 修复）
///
/// spec §6.4 DDL 原样字段；不存原文，仅存 doc_hash。
pub const DDL_LLM_AUDIT_LOG: &str = r#"
CREATE TABLE IF NOT EXISTS llm_audit_log (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    doc_hash TEXT,
    llm_provider TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    status TEXT NOT NULL,
    error_msg TEXT,
    extra_data TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON llm_audit_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_doc ON llm_audit_log(doc_hash);
"#;

/// 所有 SAG DDL 列表（按依赖顺序：knowledge_event → entity_type → entity →
/// event_entity_relation → event_entity_embedding → llm_audit_log）
pub const ALL_SAG_DDL: &[&str] = &[
    DDL_KNOWLEDGE_EVENT,
    DDL_ENTITY_TYPE,
    DDL_ENTITY,
    DDL_EVENT_ENTITY_RELATION,
    DDL_EVENT_ENTITY_EMBEDDING,
    DDL_LLM_AUDIT_LOG,
];

// ---------------------------------------------------------------------------
// Sub-Step 10.7.1：11 种默认实体类型（GREEN 阶段实现）
// ---------------------------------------------------------------------------

/// 11 种默认实体类型常量数组：(id, type, name)
///
/// 与 `INSERT_DEFAULT_ENTITY_TYPES` SQL 和 `config/extract.yaml` 保持三方一致。
/// 顺序与 `docs/SparkFox-v1.1.0-规划.md` Sub-Step 10.7.1 一致：
/// 人名 / 地名 / 机构 / 时间 / 数字 / 事件 / 物品 / 概念 / 法律 / 疾病 / 其他（兜底）。
pub const ENTITY_TYPES: &[(&str, &str, &str)] = &[
    ("default_person", "PERSON", "人名"),
    ("default_location", "LOCATION", "地名"),
    ("default_organization", "ORGANIZATION", "机构"),
    ("default_time", "TIME", "时间"),
    ("default_number", "NUMBER", "数字"),
    ("default_event", "EVENT", "事件"),
    ("default_object", "OBJECT", "物品"),
    ("default_concept", "CONCEPT", "概念"),
    ("default_law", "LAW", "法律"),
    ("default_disease", "DISEASE", "疾病"),
    ("default_other", "OTHER", "其他"),
];

/// 默认实体类型 INSERT 语句（INSERT OR IGNORE，幂等）
///
/// 在 `DDL_ENTITY_TYPE` 建表后执行；预填 11 行 `is_default=1` 的全局实体类型字典。
/// `created_time` / `updated_time` 使用固定时间戳 `2026-07-20T00:00:00Z`（Sub-Step 10.7.1 完成日），
/// 便于测试断言与多设备 CRDT 合并时稳定。
///
/// 字段映射：
/// - `id` / `scope='global'` / `type` / `name`：来自 [`ENTITY_TYPES`]
/// - `source_config_id` / `article_id`：NULL（全局默认，不绑定具体来源）
/// - `description`：简要说明识别范围
/// - `weight=1.0` / `similarity_threshold=0.8`：默认抽取权重与归一化阈值
/// - `is_active=1` / `is_default=1`：默认启用且标记为内置
/// - `value_format` / `value_constraints` / `extra_data`：NULL（v1.1.0 暂未使用）
pub const INSERT_DEFAULT_ENTITY_TYPES: &str = r#"
INSERT OR IGNORE INTO entity_type (
    id, scope, type, name, description,
    weight, similarity_threshold, is_active, is_default,
    value_format, value_constraints, extra_data,
    created_time, updated_time
) VALUES
    ('default_person',       'global', 'PERSON',       '人名', '人物名称，包括真实人物、虚构角色、历史人物等', 1.0, 0.8, 1, 1, NULL, NULL, NULL, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'),
    ('default_location',     'global', 'LOCATION',     '地名', '地理位置，包括国家、城市、区域、地标等',       1.0, 0.8, 1, 1, NULL, NULL, NULL, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'),
    ('default_organization', 'global', 'ORGANIZATION', '机构', '组织机构，包括公司、政府、学校、团队等',       1.0, 0.8, 1, 1, NULL, NULL, NULL, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'),
    ('default_time',         'global', 'TIME',         '时间', '时间表达，包括日期、时刻、时间段、节日等',     1.0, 0.8, 1, 1, NULL, NULL, NULL, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'),
    ('default_number',       'global', 'NUMBER',       '数字', '数值量词，包括数量、金额、百分比、计量等',     1.0, 0.8, 1, 1, NULL, NULL, NULL, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'),
    ('default_event',        'global', 'EVENT',        '事件', '事件活动，包括历史事件、会议、事故、庆典等',   1.0, 0.8, 1, 1, NULL, NULL, NULL, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'),
    ('default_object',       'global', 'OBJECT',       '物品', '物品实物，包括产品、设备、工具、文物等',       1.0, 0.8, 1, 1, NULL, NULL, NULL, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'),
    ('default_concept',      'global', 'CONCEPT',      '概念', '抽象概念，包括理论、思想、术语、技术等',       1.0, 0.8, 1, 1, NULL, NULL, NULL, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'),
    ('default_law',          'global', 'LAW',          '法律', '法律法规，包括法律条文、法规、条约、判例等',   1.0, 0.8, 1, 1, NULL, NULL, NULL, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'),
    ('default_disease',      'global', 'DISEASE',      '疾病', '疾病症状，包括疾病名、症状、综合征等',         1.0, 0.8, 1, 1, NULL, NULL, NULL, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z'),
    ('default_other',        'global', 'OTHER',        '其他', '兜底类型，无法归入上述 10 类的实体',           1.0, 0.8, 1, 1, NULL, NULL, NULL, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z');
"#;
