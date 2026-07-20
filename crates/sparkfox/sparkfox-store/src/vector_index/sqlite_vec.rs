//! SqliteVecIndex — `<1k` 向量轻量场景（内存 HashMap + 暴力 cosine）
//!
//! 选择理由：sqlite-vec 虚表已存在于 [`Store`](crate::Store)（`vec0`），
//! 但 `VectorIndex` 抽象需要与 `rusqlite::Connection` 生命周期解耦，故本实现
//! 采用纯内存 `HashMap` + 暴力 cosine 扫描。
//! `<1k` 规模下暴力检索 `<1ms`，无需 HNSW 索引开销。
//!
//! [`Store`]: crate::Store

use std::collections::HashMap;
use std::sync::RwLock;

use sparkfox_core::{Error, Result};

use super::{VectorFilter, VectorIndex, VectorMatch};

/// 内存版 SqliteVecIndex（暴力 cosine 扫描）
pub struct SqliteVecIndex {
    dim: usize,
    inner: RwLock<HashMap<String, Vec<f32>>>,
}

impl SqliteVecIndex {
    pub fn new(dim: usize) -> Result<Self> {
        Ok(Self {
            dim,
            inner: RwLock::new(HashMap::new()),
        })
    }
}

impl VectorIndex for SqliteVecIndex {
    fn insert(&self, id: &str, vector: &[f32]) -> Result<()> {
        if vector.len() != self.dim {
            return Err(Error::invalid_argument(
                format!("维度不匹配：期望 {} 实际 {}", self.dim, vector.len()),
                "SqliteVecIndex::insert",
            ));
        }
        let mut map = self
            .inner
            .write()
            .map_err(|e| Error::storage(format!("写锁获取失败: {e}"), "SqliteVecIndex::insert"))?;
        map.insert(id.to_string(), vector.to_vec());
        Ok(())
    }

    fn search(
        &self,
        query: &[f32],
        k: usize,
        filter: Option<&VectorFilter>,
    ) -> Result<Vec<VectorMatch>> {
        if query.len() != self.dim {
            return Err(Error::invalid_argument(
                format!("维度不匹配：期望 {} 实际 {}", self.dim, query.len()),
                "SqliteVecIndex::search",
            ));
        }
        let map = self
            .inner
            .read()
            .map_err(|e| Error::storage(format!("读锁获取失败: {e}"), "SqliteVecIndex::search"))?;
        let ref_ids: Option<&Vec<String>> = filter.and_then(|f| f.ref_ids.as_ref());
        let mut scored: Vec<(String, f32)> = map
            .iter()
            .filter(|(id, _)| match ref_ids {
                Some(ids) => ids.iter().any(|x| x == *id),
                None => true,
            })
            .map(|(id, v)| (id.clone(), cosine_sim(query, v)))
            .collect();
        // 按相似度降序
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        Ok(scored
            .into_iter()
            .map(|(id, score)| VectorMatch { id, score })
            .collect())
    }

    fn delete(&self, id: &str) -> Result<()> {
        let mut map = self
            .inner
            .write()
            .map_err(|e| Error::storage(format!("写锁获取失败: {e}"), "SqliteVecIndex::delete"))?;
        map.remove(id);
        Ok(())
    }

    fn len(&self) -> usize {
        self.inner.read().map(|m| m.len()).unwrap_or(0)
    }

    fn backend_name(&self) -> &'static str {
        "sqlite-vec"
    }
}

/// 余弦相似度（cosine similarity）。零向量返回 0 避免 NaN。
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_vec_insert_search() {
        let idx = SqliteVecIndex::new(3).expect("new");
        idx.insert("a", &[1.0, 0.0, 0.0]).expect("insert a");
        idx.insert("b", &[0.0, 1.0, 0.0]).expect("insert b");
        idx.insert("c", &[1.0, 1.0, 0.0]).expect("insert c");
        // 查询 [1,0,0]，最近邻应为 a（cos=1.0），次近邻为 c（cos≈0.707）
        let hits = idx.search(&[1.0, 0.0, 0.0], 2, None).expect("search");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "a");
        assert!(hits[0].score > 0.99, "score={} 应接近 1.0", hits[0].score);
        assert_eq!(hits[1].id, "c");
    }

    #[test]
    fn test_sqlite_vec_filter() {
        let idx = SqliteVecIndex::new(3).expect("new");
        idx.insert("a", &[1.0, 0.0, 0.0]).expect("insert a");
        idx.insert("b", &[1.0, 0.0, 0.0]).expect("insert b");
        idx.insert("c", &[1.0, 0.0, 0.0]).expect("insert c");
        idx.insert("d", &[1.0, 0.0, 0.0]).expect("insert d");
        idx.insert("e", &[1.0, 0.0, 0.0]).expect("insert e");
        let filter = VectorFilter {
            layer: 0,
            ref_ids: Some(vec!["a".into(), "b".into()]),
        };
        let hits = idx
            .search(&[1.0, 0.0, 0.0], 10, Some(&filter))
            .expect("search");
        assert_eq!(hits.len(), 2, "filter 应限制只返回 a/b");
        for h in &hits {
            assert!(h.id == "a" || h.id == "b", "意外 id={}", h.id);
        }
    }

    #[test]
    fn test_sqlite_vec_delete() {
        let idx = SqliteVecIndex::new(3).expect("new");
        idx.insert("a", &[1.0, 0.0, 0.0]).expect("insert a");
        idx.insert("b", &[0.0, 1.0, 0.0]).expect("insert b");
        assert_eq!(idx.len(), 2);
        idx.delete("a").expect("delete a");
        assert_eq!(idx.len(), 1);
        let hits = idx.search(&[1.0, 0.0, 0.0], 10, None).expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "b");
    }
}
