//! 查询嵌入缓存 — 仅缓存查询嵌入（用户决策 B：文档嵌入每次重建）
//!
//! 缓存表（复用 sparkfox-store 的 SQLite）：
//! ```sql
//! CREATE TABLE IF NOT EXISTS query_embedding_cache (
//!     content_hash TEXT NOT NULL,     -- SHA256(text)
//!     model         TEXT NOT NULL,    -- 如 "BAAI/bge-small-zh-v1.5"
//!     embedding     BLOB NOT NULL,    -- f32 小端序拼接
//!     dim           INTEGER NOT NULL, -- 向量维度（便于反序列化校验）
//!     ts            INTEGER NOT NULL, -- 写入时间戳（Unix 秒）
//!     PRIMARY KEY (content_hash, model)
//! );
//! ```
//!
//! 【设计理由】用户决策 B：文档嵌入每次重建（不缓存），仅缓存查询嵌入。
//! 查询文本通常较短且重复率高（用户会反复问相似问题），缓存收益高；
//! 文档嵌入随文档变更而失效，缓存命中率低且维护成本高。
//!
//! 【复合主键】主键为 (content_hash, model)，相同文本在不同模型下独立缓存，
//! 避免模型切换后读到脏数据。

use sha2::{Digest, Sha256};

use sparkfox_core::Result;
use sparkfox_store::Store;

/// 查询嵌入缓存
pub struct QueryCache<'a> {
    store: &'a Store,
}

impl<'a> QueryCache<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// 初始化缓存表（幂等）
    pub fn init_schema(&self) -> Result<()> {
        self.store.conn().execute_batch(
            r#"CREATE TABLE IF NOT EXISTS query_embedding_cache (
                content_hash TEXT NOT NULL,
                model TEXT NOT NULL,
                embedding BLOB NOT NULL,
                dim INTEGER NOT NULL,
                ts INTEGER NOT NULL,
                PRIMARY KEY (content_hash, model)
            );"#,
        )?;
        Ok(())
    }

    /// 查询缓存
    ///
    /// 返回 `Ok(Some(vec))` 命中，`Ok(None)` 未命中。
    /// 维度不匹配视为未命中（避免模型切换后读到脏数据）。
    pub fn get(&self, text: &str, model: &str) -> Result<Option<Vec<f32>>> {
        let hash = hash_text(text);
        let mut stmt = self.store.conn().prepare(
            "SELECT embedding, dim FROM query_embedding_cache WHERE content_hash=? AND model=?",
        )?;
        let row: Option<(Vec<u8>, i64)> = stmt
            .query_row(rusqlite::params![hash, model], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, i64>(1)?))
            })
            .ok();
        match row {
            Some((blob, expected_dim)) => {
                let vec = bytes_to_vec(&blob);
                if vec.len() as i64 != expected_dim {
                    log::warn!(
                        "query_cache 维度不匹配（可能模型已切换）：实际 {}，缓存记录 {}",
                        vec.len(),
                        expected_dim
                    );
                    return Ok(None);
                }
                Ok(Some(vec))
            }
            None => Ok(None),
        }
    }

    /// 写入缓存（INSERT OR REPLACE）
    pub fn put(&self, text: &str, model: &str, embedding: &[f32]) -> Result<()> {
        let hash = hash_text(text);
        let blob = vec_to_bytes(embedding);
        self.store.conn().execute(
            "INSERT OR REPLACE INTO query_embedding_cache(content_hash, model, embedding, dim, ts) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![hash, model, blob, embedding.len() as i64, now_ts()],
        )?;
        Ok(())
    }

    /// 清空缓存（用于模型切换或测试）
    pub fn clear(&self) -> Result<()> {
        self.store
            .conn()
            .execute("DELETE FROM query_embedding_cache", [])?;
        Ok(())
    }

    /// 缓存条目数（用于测试/监控）
    pub fn count(&self) -> Result<i64> {
        let n: i64 = self
            .store
            .conn()
            .query_row("SELECT COUNT(*) FROM query_embedding_cache", [], |r| r.get(0))?;
        Ok(n)
    }
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn vec_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn bytes_to_vec(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_to_bytes_round_trip() {
        let v = vec![0.1f32, 0.2, 0.3, -1.5, 999.999];
        let bytes = vec_to_bytes(&v);
        let back = bytes_to_vec(&bytes);
        assert_eq!(back.len(), v.len());
        for (a, b) in v.iter().zip(back.iter()) {
            assert_eq!(a.to_bits(), b.to_bits(), "f32 bit 不一致");
        }
    }

    #[test]
    fn hash_text_stable_and_distinct() {
        let h1 = hash_text("你好");
        let h2 = hash_text("你好");
        let h3 = hash_text("你好 ");
        assert_eq!(h1, h2, "相同文本应产生相同 hash");
        assert_ne!(h1, h3, "不同文本应产生不同 hash");
        assert_eq!(h1.len(), 64, "SHA256 应为 64 字符十六进制");
    }

    #[test]
    fn bytes_to_vec_empty() {
        let v = bytes_to_vec(&[]);
        assert!(v.is_empty());
    }

    #[test]
    fn vec_to_bytes_length() {
        let v = vec![1.0f32; 512];
        let bytes = vec_to_bytes(&v);
        assert_eq!(bytes.len(), 512 * 4);
    }
}
