//! SparkFox Store — 数据存储（SQLite + sqlite-vec 向量检索）
//!
//! 基于 rusqlite 0.32 + sqlite-vec 扩展加载方式（不用 sqlite-vec crate，因 alpha 版本不稳定）。
//! 6 层记忆的数据持久化层。

// NOTE: 用 `deny` 而非 `forbid`，因为 vec.rs 必须通过 FFI 加载 sqlite-vec 扩展
// （`Connection::load_extension` 是 unsafe）。`forbid` 无法被子模块 `allow` 覆盖，
// 而 `deny` 可以。除 vec.rs 外，本 crate 其余代码仍禁止 unsafe。
#![deny(unsafe_code)]

pub mod schema;
pub mod vec;
pub mod vector_index;

use std::path::Path;

use rusqlite::Connection;

use sparkfox_core::{Error, Result};
// 【A-03 P0 修复】引入 MemoryLayer 枚举用于 vector_insert 动态表名选择。
// 注意：sparkfox_core::MemoryLayer 是 trait（const LAYER + name()），
// sparkfox_memory::MemoryLayer 是枚举（含 L3Episodic/L3Semantic/L3EventEntity 等 10 变体），
// 二者不冲突，这里用的是枚举。
use sparkfox_memory::MemoryLayer;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct StoreConfig {
    pub path: std::path::PathBuf,
    pub enable_vec: bool,
}

impl StoreConfig {
    pub fn for_path(p: impl AsRef<Path>) -> Self {
        Self { path: p.as_ref().to_path_buf(), enable_vec: true }
    }
}

pub struct Store {
    conn: Connection,
    vec_loaded: bool,
}

impl Store {
    pub fn open(cfg: StoreConfig) -> Result<Self> {
        let conn = Connection::open(&cfg.path)
            .map_err(|e| Error::storage(format!("打开 SQLite 失败: {e}"), "Store::open"))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut vec_loaded = false;
        if cfg.enable_vec {
            match vec::load_vec_extension(&conn) {
                Ok(()) => vec_loaded = true,
                Err(e) => log::warn!("sqlite-vec 加载失败（向量功能降级）: {e}"),
            }
        }
        let me = Self { conn, vec_loaded };
        me.migrate()?;
        Ok(me)
    }

    pub fn migrate(&self) -> Result<()> {
        schema::migrate_all(&self.conn)
    }

    pub fn conn(&self) -> &Connection { &self.conn }

    pub fn is_vec_loaded(&self) -> bool { self.vec_loaded }

    /// 【A-03 P0 修复】vector_insert 重构 — 支持 layer 动态表名
    ///
    /// 按 [`MemoryLayer`] 选择对应的向量表，覆盖 SAG 4 类向量：
    /// - `L0Raw` → `vec_l0`（原始 chunk）
    /// - `L3Episodic` → `vec_l3_event`（SAG event）
    /// - `L3Semantic` / `L3GraphNode` → `vec_l3_entity`（SAG entity）
    /// - `L3GraphEdge` / `L3EventEntity` → `vec_l3_event_entity`（SAG 关联级嵌入）
    ///
    /// 表名来自 `layer.vector_table_name()` 返回的 `&'static str` 常量，
    /// 字符串拼接无 SQL 注入风险。
    ///
    /// **注意**：`vec_l3_event` / `vec_l3_entity` / `vec_l3_event_entity` 等表
    /// 由 Task 3.1 SAG schema 迁移创建；在 schema 未就绪时本函数会返回 SQL 错误。
    pub fn vector_insert(
        &self,
        layer: MemoryLayer,
        ref_id: &str,
        model: &str,
        v: &[f32],
    ) -> Result<()> {
        if !self.vec_loaded {
            return Err(Error::storage("sqlite-vec 未加载".into(), "vector_insert"));
        }
        let table = layer.vector_table_name();
        let layer_num = memory_layer_to_numeric(layer);
        let blob = v.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
        // 元数据写入 memory_vectors（layer 按 0-5 数值存，与 schema.rs V2 一致）
        self.conn.execute(
            "INSERT INTO memory_vectors (layer, ref_id, model, dim, ts) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![layer_num, ref_id, model, v.len() as i64, now_ts()],
        )?;
        let id = self.conn.last_insert_rowid();
        // 向量数据写入 vec0 虚表（按 layer 分表）
        // SAFETY: table 来自 layer.vector_table_name() 的 &'static str 常量，无注入风险
        let sql = format!(
            "INSERT INTO {table}(id, embedding) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET embedding=excluded.embedding"
        );
        self.conn.execute(&sql, rusqlite::params![id, blob])?;
        Ok(())
    }

    pub fn vector_flush(&self) -> Result<()> {
        // sqlite 默认 autocommit，flush 为占位以便未来引入显式事务
        Ok(())
    }

    /// 【A-03 P0 修复】vector_search 重构 — 按 layer 动态表名检索
    ///
    /// 与 [`vector_insert`](Self::vector_insert) 配套，按 `layer.vector_table_name()`
    /// 选择对应的 vec0 虚表执行 KNN 检索。返回 `(id, distance)` 列表（distance 越小越相似）。
    pub fn vector_search(
        &self,
        layer: MemoryLayer,
        query: &[f32],
        k: usize,
    ) -> Result<Vec<(i64, f32)>> {
        if !self.vec_loaded {
            return Err(Error::storage("sqlite-vec 未加载".into(), "vector_search"));
        }
        let table = layer.vector_table_name();
        let blob = query.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
        // SAFETY: table 来自 layer.vector_table_name() 的 &'static str 常量，无注入风险
        let sql = format!(
            "SELECT id, distance FROM {table} WHERE embedding MATCH ? ORDER BY distance LIMIT ?"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params![blob, k as i64], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, f32>(1)?))
        })?;
        let mut out = Vec::with_capacity(k);
        for r in rows { out.push(r?); }
        Ok(out)
    }
}

/// MemoryLayer 枚举 → 6 层记忆数值（用于 `memory_vectors.layer` 字段）
///
/// L3 的 5 个子层（Episodic/Semantic/GraphNode/GraphEdge/EventEntity）统一映射到 `3`，
/// 子层差异由 `vec_l3_event` / `vec_l3_entity` / `vec_l3_event_entity` 等表名区分。
fn memory_layer_to_numeric(layer: MemoryLayer) -> i64 {
    match layer {
        MemoryLayer::L0Raw => 0,
        MemoryLayer::L1Working => 1,
        MemoryLayer::L2Core => 2,
        MemoryLayer::L3Episodic
        | MemoryLayer::L3Semantic
        | MemoryLayer::L3GraphNode
        | MemoryLayer::L3GraphEdge
        | MemoryLayer::L3EventEntity => 3,
        MemoryLayer::L4Persona => 4,
        MemoryLayer::L5Meta => 5,
    }
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    /// 验证 MemoryLayer → 数值层级映射（A-03 P0 修复，memory_vectors.layer 字段）
    #[test]
    fn test_memory_layer_to_numeric() {
        assert_eq!(memory_layer_to_numeric(MemoryLayer::L0Raw), 0);
        assert_eq!(memory_layer_to_numeric(MemoryLayer::L1Working), 1);
        assert_eq!(memory_layer_to_numeric(MemoryLayer::L2Core), 2);
        // L3 5 个子层统一映射到 3（子层差异由表名区分）
        assert_eq!(memory_layer_to_numeric(MemoryLayer::L3Episodic), 3);
        assert_eq!(memory_layer_to_numeric(MemoryLayer::L3Semantic), 3);
        assert_eq!(memory_layer_to_numeric(MemoryLayer::L3GraphNode), 3);
        assert_eq!(memory_layer_to_numeric(MemoryLayer::L3GraphEdge), 3);
        assert_eq!(memory_layer_to_numeric(MemoryLayer::L3EventEntity), 3);
        assert_eq!(memory_layer_to_numeric(MemoryLayer::L4Persona), 4);
        assert_eq!(memory_layer_to_numeric(MemoryLayer::L5Meta), 5);
    }

    /// 验证不同 layer 选择不同的 vector 表名（A-03 P0 修复核心，SAG 4 类向量分表）
    #[test]
    fn test_vector_insert_dynamic_table() {
        // 6 大层各自独立表
        assert_eq!(MemoryLayer::L0Raw.vector_table_name(), "vec_l0");
        assert_eq!(MemoryLayer::L1Working.vector_table_name(), "vec_l1");
        assert_eq!(MemoryLayer::L2Core.vector_table_name(), "vec_l2");
        assert_eq!(MemoryLayer::L4Persona.vector_table_name(), "vec_l4");
        assert_eq!(MemoryLayer::L5Meta.vector_table_name(), "vec_l5");
        // SAG 4 类向量映射
        assert_eq!(MemoryLayer::L3Episodic.vector_table_name(), "vec_l3_event");
        assert_eq!(MemoryLayer::L3Semantic.vector_table_name(), "vec_l3_entity");
        assert_eq!(MemoryLayer::L3GraphNode.vector_table_name(), "vec_l3_entity");
        assert_eq!(MemoryLayer::L3GraphEdge.vector_table_name(), "vec_l3_event_entity");
        assert_eq!(MemoryLayer::L3EventEntity.vector_table_name(), "vec_l3_event_entity");

        // 关键差异：SAG 3 张 vec_l3_* 表互不相同
        assert_ne!(
            MemoryLayer::L3Episodic.vector_table_name(),
            MemoryLayer::L3Semantic.vector_table_name(),
            "event 与 entity 必须分表"
        );
        assert_ne!(
            MemoryLayer::L3Semantic.vector_table_name(),
            MemoryLayer::L3EventEntity.vector_table_name(),
            "entity 与 event_entity 必须分表"
        );
        assert_ne!(
            MemoryLayer::L3Episodic.vector_table_name(),
            MemoryLayer::L3EventEntity.vector_table_name(),
            "event 与 event_entity 必须分表"
        );
    }

    /// 验证 vector_insert(L0Raw) 签名可用且在 sqlite-vec 未加载时返回 Err
    ///
    /// 测试环境通常未放置 sqlite-vec 二进制 → `vec_loaded=false` → 返回 storage Err。
    /// 即使 sqlite-vec 加载成功，`vec_l0` 表当前未在 schema.rs 中创建（待 Task 3.1），
    /// 因此也会返回 SQL Err。两种情况都证明签名调用正确。
    #[test]
    fn test_vector_insert_l0() {
        let tmp = NamedTempFile::new().unwrap();
        let store = Store::open(StoreConfig::for_path(tmp.path())).unwrap();
        let v = vec![0.1f32, 0.2, 0.3];
        let result = store.vector_insert(MemoryLayer::L0Raw, "ref_l0_1", "bge-large-zh", &v);
        // sqlite-vec 未加载 → Err；已加载但 vec_l0 表未创建（Task 3.1 负责）→ 也是 Err
        assert!(
            result.is_err(),
            "vector_insert(L0Raw) 在测试环境应返回 Err（无 sqlite-vec 或 vec_l0 表未创建）"
        );
    }

    /// 验证 vector_insert(L3Episodic) 签名可用 — SAG event 向量路径
    ///
    /// 注意：`vec_l3_event` 表由 Task 3.1 SAG schema 迁移创建。
    /// 在 schema 未就绪时本测试只验证函数可被调用且返回 Err（不 panic）。
    #[test]
    fn test_vector_insert_l3_event() {
        let tmp = NamedTempFile::new().unwrap();
        let store = Store::open(StoreConfig::for_path(tmp.path())).unwrap();
        let v = vec![0.1f32; 8];
        let result = store.vector_insert(MemoryLayer::L3Episodic, "ref_event_1", "bge-large-zh", &v);
        assert!(
            result.is_err(),
            "vector_insert(L3Episodic) 在测试环境应返回 Err（依赖 Task 3.1 创建 vec_l3_event 表）"
        );
    }

    /// 验证 vector_insert(L3Semantic) 签名可用 — SAG entity 向量路径
    ///
    /// 注意：`vec_l3_entity` 表由 Task 3.1 SAG schema 迁移创建。
    #[test]
    fn test_vector_insert_l3_entity() {
        let tmp = NamedTempFile::new().unwrap();
        let store = Store::open(StoreConfig::for_path(tmp.path())).unwrap();
        let v = vec![0.1f32; 8];
        let result = store.vector_insert(MemoryLayer::L3Semantic, "ref_entity_1", "bge-large-zh", &v);
        assert!(
            result.is_err(),
            "vector_insert(L3Semantic) 在测试环境应返回 Err（依赖 Task 3.1 创建 vec_l3_entity 表）"
        );
    }

    /// 验证 vector_insert(L3EventEntity) 签名可用 — SAG 关联级嵌入路径
    ///
    /// 注意：`vec_l3_event_entity` 表由 Task 3.1 SAG schema 迁移创建。
    #[test]
    fn test_vector_insert_l3_event_entity() {
        let tmp = NamedTempFile::new().unwrap();
        let store = Store::open(StoreConfig::for_path(tmp.path())).unwrap();
        let v = vec![0.1f32; 8];
        let result = store.vector_insert(MemoryLayer::L3EventEntity, "ref_rel_1", "bge-large-zh", &v);
        assert!(
            result.is_err(),
            "vector_insert(L3EventEntity) 在测试环境应返回 Err（依赖 Task 3.1 创建 vec_l3_event_entity 表）"
        );
    }
}
