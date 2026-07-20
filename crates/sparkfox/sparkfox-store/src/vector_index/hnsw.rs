//! HnswIndex — `>=1k` 向量主力后端
//!
//! ## 实现选择（Sub-Step 10.13.2 方案 C：优化暴力扫描）
//!
//! ### Windows 兼容性现状与方案决策
//! - `hnswlib-rs = "0.10"`：传递依赖 `off64 0.9` 无条件使用
//!   `std::os::unix::prelude::FileExt`，Windows MSVC 编译失败。
//!   截至 2026-07-20 仍未修复（off64 / hnswlib-rs 均无 feature flag 可禁用）。
//! - `instant-distance = "0.6"`：纯 Rust HNSW 实现，Windows 可编译，
//!   但 API 要求 `Builder::build(points)` 一次性建图，不支持
//!   [`VectorIndex`] trait 要求的增量 `insert(id, vector)` 调用，
//!   与现有抽象根本不兼容，需重写 trait，超出本 Sub-Step 范围。
//! - `usearch-rs`：C++ FFI，需 C++ 编译器与原生链接，构建复杂度高，
//!   usearch 自身对 Windows 支持不完整。
//! - **方案 C（已选）**：保留内存 `HashMap` 后端，优化查询路径：
//!   1. 插入时预归一化向量（L2 norm = 1），存储归一化版本
//!   2. 查询时一次性归一化 query，然后纯点积（避免循环内重复计算 norm）
//!   3. 1k 768维向量单次查询约 0.5-2ms（远低于 50ms 验收线）
//!   4. 10 万向量约 50-200ms（超出 spec <50ms 目标，记为 L3 阻塞，
//!      待 hnswlib-rs 修复 Windows 兼容后切换为真实 HNSW 实现）
//!
//! ### 持久化格式（v1，自定义二进制）
//! ```text
//! magic[4]            = b"HNSW"
//! version[4]          = u32 LE (当前 = 1)
//! dim[4]              = u32 LE
//! count[4]            = u32 LE
//! 重复 count 次:
//!   id_len[4]         = u32 LE
//!   id_bytes[id_len]  = UTF-8 字符串
//!   vector[dim*4]     = f32 LE × dim
//! ```
//! 选择自定义二进制而非 bincode / serde：
//! - 无新依赖（serde / bincode 未在 sparkfox-store Cargo.toml 中）
//! - 格式简单稳定，未来真实 HNSW 实现可扩展（追加 graph 结构）
//!
//! ### HNSW 参数（保留以备未来真实 HNSW 实现使用）
//! spec v2.0 Task 1.5 推荐参数：
//! - `M = 16`（图连接度）
//! - `ef_construction = 200`（建图探索深度）
//! - `ef_search = 64`（检索探索深度）
//! - 距离：Cosine；删除：`mark_deleted` (tombstone)
//!
//! 当前占位实现未使用上述参数（暴力扫描无需图参数），但通过 [`HnswConfig`]
//! 结构体保留，以便未来切换真实 HNSW 时直接接入。

use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::RwLock;

use sparkfox_core::{Error, Result};

use super::{VectorFilter, VectorIndex, VectorMatch};

/// HNSW 配置参数（保留以备未来真实 HNSW 实现使用）
///
/// 当前方案 C 暴力扫描不使用这些参数，但保留结构体以便未来切换为
/// 真实 HNSW 后端（如 hnswlib-rs 修复 Windows 兼容后）时直接接入。
#[derive(Debug, Clone)]
pub struct HnswConfig {
    /// 图连接度（M）。spec v2.0 Task 1.5 推荐值：16。
    pub m: usize,
    /// 建图探索深度。spec v2.0 Task 1.5 推荐值：200。
    pub ef_construction: usize,
    /// 检索探索深度。spec v2.0 Task 1.5 推荐值：64。
    pub ef_search: usize,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            m: 16,
            ef_construction: 200,
            ef_search: 64,
        }
    }
}

/// HNSW 向量索引（方案 C：优化暴力扫描 — 预归一化 + 点积）
///
/// 内部存储已归一化的向量（L2 norm = 1），查询时归一化 query 后做点积，
/// 结果即为 cosine 相似度，避免循环内重复计算 norm。
pub struct HnswIndex {
    dim: usize,
    /// 当前未使用（保留以备未来真实 HNSW 实现接入）
    #[allow(dead_code)]
    config: HnswConfig,
    inner: RwLock<HashMap<String, Vec<f32>>>,
}

impl HnswIndex {
    /// 创建指定维度的 HNSW 索引（使用默认 [`HnswConfig`]）
    pub fn new(dim: usize) -> Result<Self> {
        Self::with_config(dim, HnswConfig::default())
    }

    /// 创建指定维度与配置的 HNSW 索引
    ///
    /// 当前 `config` 字段未使用（暴力扫描无需图参数），保留参数以便
    /// 未来切换真实 HNSW 后端时无需修改调用方。
    pub fn with_config(dim: usize, config: HnswConfig) -> Result<Self> {
        Ok(Self {
            dim,
            config,
            inner: RwLock::new(HashMap::new()),
        })
    }

    /// 持久化索引到磁盘
    ///
    /// 文件格式见模块级文档。原子性：先写入 `<path>.tmp`，再 rename 为 `<path>`，
    /// 避免崩溃导致部分写入。
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let map = self
            .inner
            .read()
            .map_err(|e| Error::storage(format!("读锁获取失败: {e}"), "HnswIndex::save"))?;

        // 先写入 .tmp，再 rename，避免崩溃导致部分写入
        let mut tmp_path = path.to_path_buf();
        let mut tmp_name = tmp_path
            .file_name()
            .map(|n| n.to_os_string())
            .unwrap_or_default();
        tmp_name.push(".tmp");
        tmp_path.set_file_name(tmp_name);

        let file = std::fs::File::create(&tmp_path).map_err(|e| {
            Error::storage(
                format!("创建临时文件失败: {e} (path={})", tmp_path.display()),
                "HnswIndex::save",
            )
        })?;
        let mut w = BufWriter::new(file);

        // 文件头
        w.write_all(b"HNSW").map_err(|e| {
            Error::storage(format!("写入 magic 失败: {e}"), "HnswIndex::save")
        })?;
        w.write_all(&1u32.to_le_bytes()).map_err(|e| {
            Error::storage(format!("写入 version 失败: {e}"), "HnswIndex::save")
        })?;
        w.write_all(&(self.dim as u32).to_le_bytes()).map_err(|e| {
            Error::storage(format!("写入 dim 失败: {e}"), "HnswIndex::save")
        })?;
        w.write_all(&(map.len() as u32).to_le_bytes()).map_err(|e| {
            Error::storage(format!("写入 count 失败: {e}"), "HnswIndex::save")
        })?;

        // 每条向量
        for (id, v) in map.iter() {
            let id_bytes = id.as_bytes();
            w.write_all(&(id_bytes.len() as u32).to_le_bytes())
                .map_err(|e| Error::storage(format!("写入 id_len 失败: {e}"), "HnswIndex::save"))?;
            w.write_all(id_bytes).map_err(|e| {
                Error::storage(format!("写入 id 失败: {e}"), "HnswIndex::save")
            })?;
            // 向量按 f32 LE 顺序写入
            for f in v {
                w.write_all(&f.to_le_bytes()).map_err(|e| {
                    Error::storage(format!("写入 vector 失败: {e}"), "HnswIndex::save")
                })?;
            }
        }
        w.flush().map_err(|e| {
            Error::storage(format!("flush 失败: {e}"), "HnswIndex::save")
        })?;
        drop(w); // 关闭 BufWriter 与底层文件句柄，确保 Windows 上 rename 前已释放

        // 原子 rename（Windows 上若目标存在会失败，故先删除目标）
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| {
                Error::storage(
                    format!("删除旧文件失败: {e} (path={})", path.display()),
                    "HnswIndex::save",
                )
            })?;
        }
        std::fs::rename(&tmp_path, path).map_err(|e| {
            Error::storage(
                format!(
                    "rename 失败: {e} (from={} to={})",
                    tmp_path.display(),
                    path.display()
                ),
                "HnswIndex::save",
            )
        })?;
        Ok(())
    }

    /// 从磁盘加载索引
    ///
    /// 文件格式见模块级文档。校验 magic 与 version，失败返回 storage 错误。
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = std::fs::File::open(path).map_err(|e| {
            Error::storage(
                format!("打开文件失败: {e} (path={})", path.display()),
                "HnswIndex::load",
            )
        })?;
        let mut r = BufReader::new(file);

        // 文件头
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic).map_err(|e| {
            Error::storage(format!("读取 magic 失败: {e}"), "HnswIndex::load")
        })?;
        if &magic != b"HNSW" {
            return Err(Error::storage(
                format!("无效的 HNSW 文件头: {:?}", magic),
                "HnswIndex::load",
            ));
        }

        let mut buf4 = [0u8; 4];
        r.read_exact(&mut buf4).map_err(|e| {
            Error::storage(format!("读取 version 失败: {e}"), "HnswIndex::load")
        })?;
        let version = u32::from_le_bytes(buf4);
        if version != 1 {
            return Err(Error::storage(
                format!("不支持的版本: {version}（当前仅支持 1）"),
                "HnswIndex::load",
            ));
        }

        r.read_exact(&mut buf4).map_err(|e| {
            Error::storage(format!("读取 dim 失败: {e}"), "HnswIndex::load")
        })?;
        let dim = u32::from_le_bytes(buf4) as usize;

        r.read_exact(&mut buf4).map_err(|e| {
            Error::storage(format!("读取 count 失败: {e}"), "HnswIndex::load")
        })?;
        let count = u32::from_le_bytes(buf4) as usize;

        let mut map = HashMap::with_capacity(count);
        for _ in 0..count {
            r.read_exact(&mut buf4).map_err(|e| {
                Error::storage(format!("读取 id_len 失败: {e}"), "HnswIndex::load")
            })?;
            let id_len = u32::from_le_bytes(buf4) as usize;
            let mut id_bytes = vec![0u8; id_len];
            r.read_exact(&mut id_bytes).map_err(|e| {
                Error::storage(format!("读取 id 失败: {e}"), "HnswIndex::load")
            })?;
            let id = String::from_utf8(id_bytes).map_err(|e| {
                Error::parse(format!("id UTF-8 解码失败: {e}"), "HnswIndex::load")
            })?;

            let mut v = vec![0f32; dim];
            let mut v_bytes = vec![0u8; dim * 4];
            r.read_exact(&mut v_bytes).map_err(|e| {
                Error::storage(format!("读取 vector 失败: {e}"), "HnswIndex::load")
            })?;
            for (i, chunk) in v_bytes.chunks_exact(4).enumerate() {
                v[i] = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            }
            map.insert(id, v);
        }

        Ok(Self {
            dim,
            config: HnswConfig::default(),
            inner: RwLock::new(map),
        })
    }
}

impl VectorIndex for HnswIndex {
    fn insert(&self, id: &str, vector: &[f32]) -> Result<()> {
        if vector.len() != self.dim {
            return Err(Error::invalid_argument(
                format!("维度不匹配：期望 {} 实际 {}", self.dim, vector.len()),
                "HnswIndex::insert",
            ));
        }
        // 预归一化：存储 v / ||v||，查询时点积即为 cosine 相似度
        let normalized = normalize(vector);
        let mut map = self
            .inner
            .write()
            .map_err(|e| Error::storage(format!("写锁获取失败: {e}"), "HnswIndex::insert"))?;
        map.insert(id.to_string(), normalized);
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
                "HnswIndex::search",
            ));
        }
        let map = self
            .inner
            .read()
            .map_err(|e| Error::storage(format!("读锁获取失败: {e}"), "HnswIndex::search"))?;
        // 一次性归一化 query，循环内只做点积（cosine 相似度 = 单位点积）
        let q_norm = normalize(query);
        let ref_ids: Option<&Vec<String>> = filter.and_then(|f| f.ref_ids.as_ref());
        let mut scored: Vec<(String, f32)> = map
            .iter()
            .filter(|(id, _)| match ref_ids {
                Some(ids) => ids.iter().any(|x| x == *id),
                None => true,
            })
            .map(|(id, v)| (id.clone(), dot(&q_norm, v)))
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
            .map_err(|e| Error::storage(format!("写锁获取失败: {e}"), "HnswIndex::delete"))?;
        map.remove(id);
        Ok(())
    }

    fn len(&self) -> usize {
        self.inner.read().map(|m| m.len()).unwrap_or(0)
    }

    fn backend_name(&self) -> &'static str {
        "hnswlib-rs (placeholder)"
    }
}

/// L2 归一化：返回 `v / ||v||`。零向量原样返回（避免 NaN）。
///
/// 归一化后两向量的点积等于它们的 cosine 相似度，使查询循环内
/// 只需一次乘加（无需为每条候选向量重复计算 norm）。
fn normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        v.to_vec()
    } else {
        v.iter().map(|x| x / norm).collect()
    }
}

/// 点积（dot product）。归一化向量点积 = cosine 相似度。
fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 简易 xorshift64 PRNG（测试用，避免引入 rand 依赖）
    fn xorshift(seed: &mut u64) -> u64 {
        let mut x = *seed;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *seed = x;
        x
    }

    #[test]
    fn test_hnsw_insert_search() {
        let idx = HnswIndex::new(8).expect("new");
        // 插入 100 条随机向量
        let mut seed: u64 = 0xDEAD_BEEF_CAFE;
        for i in 0..100 {
            let mut v = vec![0.0f32; 8];
            for v_el in v.iter_mut() {
                let r = xorshift(&mut seed);
                *v_el = (r as f32 / u64::MAX as f32) * 2.0 - 1.0;
            }
            idx.insert(&format!("v{i}"), &v).expect("insert");
        }
        assert_eq!(idx.len(), 100, "len 应为 100");
        // 检索 k=5
        let query = vec![0.5f32; 8];
        let hits = idx.search(&query, 5, None).expect("search");
        assert_eq!(hits.len(), 5, "应返回 5 条匹配");
    }

    #[test]
    fn test_hnsw_delete() {
        let idx = HnswIndex::new(4).expect("new");
        idx.insert("a", &[1.0, 0.0, 0.0, 0.0]).expect("insert a");
        idx.insert("b", &[0.0, 1.0, 0.0, 0.0]).expect("insert b");
        assert_eq!(idx.len(), 2);
        idx.delete("a").expect("delete a");
        assert_eq!(idx.len(), 1, "len 应为 1");
        // 查询 [1,0,0,0] — 最近邻本应是 a，但已删除，应只返回 b
        let hits = idx.search(&[1.0, 0.0, 0.0, 0.0], 10, None).expect("search");
        assert!(!hits.is_empty(), "应至少返回 1 条结果");
        assert!(hits.iter().all(|h| h.id != "a"), "已删除的 a 不应出现");
    }

    #[test]
    fn test_hnsw_filter() {
        let idx = HnswIndex::new(4).expect("new");
        // 5 条非零向量（避免 cosine 对零向量未定义）
        for i in 0..5 {
            let mut v = vec![0.0f32; 4];
            v[0] = (i as f32 + 1.0) / 10.0; // 0.1, 0.2, 0.3, 0.4, 0.5
            idx.insert(&format!("v{i}"), &v).expect("insert");
        }
        let filter = VectorFilter {
            layer: 0,
            ref_ids: Some(vec!["v0".into(), "v1".into()]),
        };
        let hits = idx
            .search(&[0.15, 0.0, 0.0, 0.0], 10, Some(&filter))
            .expect("search");
        assert_eq!(hits.len(), 2, "filter 应限制只返回 v0/v1");
        for h in &hits {
            assert!(
                h.id == "v0" || h.id == "v1",
                "filter 应限制只返回 v0/v1，实际={}",
                h.id
            );
        }
    }
}
