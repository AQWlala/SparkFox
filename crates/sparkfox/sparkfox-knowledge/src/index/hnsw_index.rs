//! Sub-Step 11.6.1 — HnswIndex 实现（spec §三 11.6.1）
//!
//! 基于 `hnsw_rs`（纯 Rust HNSW 算法）的高维向量近似最近邻索引，作为 `sqlite-vec`
//! 的补充向量检索引擎。
//!
//! ## 适用场景
//! - 高维向量（>= 256 维）的 kNN 检索
//! - 大规模向量库（> 10k 向量）的快速检索
//! - 内存充裕的场景（HNSW 索引驻留内存）
//!
//! ## 持久化策略
//! 采用"向量缓存 + 重建图"策略：
//! - `save()`：将 `dim` / `count` / `id_map` / `vectors` 序列化到单个二进制文件
//! - `load()`：读取文件，新建 HnswIndex 并重新 insert 所有向量（重建 HNSW 图）
//!
//! ### 为何不使用 `hnsw_rs::HnswIo::load_hnsw`
//! `HnswIo::load_hnsw` 返回的 `Hnsw<'b, T, D>` 含生命周期参数 `'b`，受
//! `HnswIo` reloader 生命周期约束（`'a: 'b`）。由于本项目 `forbid(unsafe_code)`，
//! 无法用 `unsafe` 将 `'b` 提升为 `'static`。因此采用"重建图"策略：
//! - 优势：单文件持久化，无生命周期复杂性，load 后 HnswIndex 完全 owned
//! - 代价：load 时间复杂度 O(N log N)（重建 HNSW 图），对 100k 向量约 1-3s
//!
//! ## 持久化文件格式（v1，自定义二进制，与 sparkfox-store/hnsw.rs 一致）
//! ```text
//! magic[4]            = b"SFHW"   (SparkFox Hnsw Wrapper)
//! version[4]          = u32 LE (当前 = 1)
//! dim[4]              = u32 LE
//! count[4]            = u32 LE
//! 重复 count 次:
//!   id_len[4]         = u32 LE
//!   id_bytes[id_len]  = UTF-8 字符串（entity_id）
//!   vector[dim*4]     = f32 LE × dim
//! ```
//!
//! ## 与 `Step3VectorIndex` trait 的集成（11.1.2 定义）
//! `Step3VectorIndex::search_top_k` 返回 `Vec<(id, score)>`，`score` 语义为
//! cosine 相似度（越大越相似）。本 impl 将 HnswIndex 的 `distance`（越小越相似）
//! 转换为 `similarity = 1.0 - distance`。
//!
//! ## HNSW 参数（spec v2.0 Task 1.5 推荐值）
//! - `M = 16`（max_nb_connection，图连接度）
//! - `ef_construction = 200`（建图探索深度）
//! - `ef_search = max(k * 2, 100)`（检索探索深度，动态调整保证小数据集准确性）
//! - 距离：DistCosine
//!
//! ## License
//! AGPL-3.0-only

use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

// hnsw_rs 重导出 anndists crate（见 hnsw_rs/src/lib.rs: pub use anndists;），
// DistCosine 实际定义在 anndists::dist::distances 模块，由 anndists::dist 通过
// `pub use distances::*` 再次导出。此处通过 hnsw_rs::anndists::dist::DistCosine 访问。
use hnsw_rs::anndists::dist::DistCosine;
// Hnsw / Neighbour 均定义在 hnsw_rs::hnsw 模块；AnnT 在 hnsw_rs::api 模块（本文件未使用）
use hnsw_rs::hnsw::{Hnsw, Neighbour};

use sparkfox_core::{Error, Result};

use crate::search::multi_step::Step3VectorIndex;

/// HNSW 向量索引（spec §三 11.6.1）
///
/// 基于 `hnsw_rs` 的高维向量近似最近邻索引，替代/补充 sqlite-vec。
///
/// ## 内部结构
/// - `hnsw`: `hnsw_rs` 的 HNSW 算法实现（pure Rust，Windows 兼容）
/// - `id_map`: HNSW 内部 id（usize）→ 业务 entity_id（String）映射
/// - `vectors`: 原始向量缓存，用于 save/load 持久化（重建图策略）
/// - `dim`: 向量维度
/// - `persist_path`: 上次 save/load 的路径（用于将来增量 save）
///
/// ## 线程安全
/// `Hnsw<T, D>` 内部使用 `parking_lot::RwLock`，故 `HnswIndex` 是 `Send + Sync`。
/// `insert` / `search` 均接收 `&self`（hnsw_rs API），无需外部同步。
///
/// ## 示例
/// ```ignore
/// use sparkfox_knowledge::index::HnswIndex;
///
/// let mut index = HnswIndex::new(1000, 384);
/// index.insert(&vec![0.1; 384], "ent-1")?;
/// index.insert(&vec![0.2; 384], "ent-2")?;
///
/// let results = index.search(&vec![0.15; 384], 5)?;
/// for (entity_id, distance) in results {
///     println!("{}: distance={}", entity_id, distance);
/// }
/// ```
pub struct HnswIndex {
    /// HNSW 内部索引（pure Rust 实现，Windows 兼容）
    hnsw: Hnsw<'static, f32, DistCosine>,
    /// id → entity_id 映射（HNSW 内部 id 到业务 entity_id）
    id_map: Vec<String>,
    /// 原始向量缓存（用于 save/load 持久化，重建图策略）
    vectors: Vec<Vec<f32>>,
    /// 维度
    dim: usize,
    /// 持久化路径（上次 save/load 的路径，用于将来增量 save）
    persist_path: Option<PathBuf>,
}

/// HNSW 参数（spec v2.0 Task 1.5 推荐值）
///
/// 当前为常量配置，未来可暴露为 `HnswIndex::with_config` 构造器。
const MAX_NB_CONNECTION: usize = 16; // M
const MAX_LAYER: usize = 16;
const EF_CONSTRUCTION: usize = 200;

/// 持久化文件 magic（SparkFox Hnsw Wrapper）
const MAGIC: &[u8; 4] = b"SFHW";
/// 持久化文件版本
const VERSION: u32 = 1;

impl HnswIndex {
    /// 新建 HnswIndex
    ///
    /// ## 参数
    /// - `max_elements`: 预期最大向量数（用于 HNSW 内部表预分配，非硬上限）
    /// - `dim`: 向量维度（如 384 / 768 / 1024）
    ///
    /// ## 返回
    /// 空的 HnswIndex（`len()=0`，`is_empty()=true`）
    pub fn new(max_elements: usize, dim: usize) -> Self {
        let hnsw = Hnsw::<f32, DistCosine>::new(
            MAX_NB_CONNECTION,
            max_elements,
            MAX_LAYER,
            EF_CONSTRUCTION,
            DistCosine,
        );
        Self {
            hnsw,
            id_map: Vec::new(),
            vectors: Vec::new(),
            dim,
            persist_path: None,
        }
    }

    /// 插入向量
    ///
    /// ## 参数
    /// - `vector`: 向量数据（长度必须等于 `dim`）
    /// - `entity_id`: 业务实体 ID（如 `"ent-1"`）
    ///
    /// ## 错误
    /// - 维度不匹配时返回 `InvalidArgument` 错误
    pub fn insert(&mut self, vector: &[f32], entity_id: &str) -> Result<()> {
        if vector.len() != self.dim {
            return Err(Error::invalid_argument(
                format!(
                    "维度不匹配：期望 {} 实际 {}",
                    self.dim,
                    vector.len()
                ),
                "HnswIndex::insert",
            ));
        }
        let id = self.id_map.len();
        // hnsw_rs::Hnsw::insert_slice 接收 &self（内部用 RwLock 保证线程安全）
        self.hnsw.insert_slice((vector, id));
        self.id_map.push(entity_id.to_string());
        // 缓存原始向量，用于 save/load 持久化（重建图策略）
        self.vectors.push(vector.to_vec());
        Ok(())
    }

    /// 批量插入向量
    ///
    /// ## 参数
    /// - `vectors`: `&[(向量切片, entity_id)]` 列表
    ///
    /// ## 错误
    /// - 任一向量维度不匹配时返回 `InvalidArgument` 错误（已插入的向量不回滚）
    pub fn insert_batch(&mut self, vectors: &[(&[f32], &str)]) -> Result<()> {
        for (vector, entity_id) in vectors {
            self.insert(vector, entity_id)?;
        }
        Ok(())
    }

    /// kNN 检索
    ///
    /// ## 参数
    /// - `query`: 查询向量（长度必须等于 `dim`）
    /// - `k`: 返回的最近邻数量
    ///
    /// ## 返回
    /// `Vec<(entity_id, distance)>`，按距离升序（最近在前）。
    /// `distance` 语义：cosine 距离，范围 `[0, 2]`，越小越相似（自身距离=0）。
    ///
    /// ## 错误
    /// - 维度不匹配时返回 `InvalidArgument` 错误
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        if query.len() != self.dim {
            return Err(Error::invalid_argument(
                format!(
                    "维度不匹配：期望 {} 实际 {}",
                    self.dim,
                    query.len()
                ),
                "HnswIndex::search",
            ));
        }
        // ef_arg 控制搜索宽度，必须 >= k。
        // 为小数据集准确性，使用 ef = max(k * 2, 100)（介于 k 与 ef_construction 之间）
        let ef = std::cmp::max(k * 2, 100);
        let neighbours: Vec<Neighbour> = self.hnsw.search(query, k, ef);
        let mut results: Vec<(String, f32)> = neighbours
            .into_iter()
            .map(|n| {
                let entity_id = self
                    .id_map
                    .get(n.d_id)
                    .cloned()
                    .unwrap_or_else(|| format!("unknown-{}", n.d_id));
                (entity_id, n.distance)
            })
            .collect();
        // 按距离升序（最近在前）— hnsw_rs 已排序，但显式排序保证语义
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    }

    /// 保存到磁盘
    ///
    /// 将 `dim` / `count` / `id_map` / `vectors` 序列化到单个二进制文件。
    /// 文件格式见模块级文档。
    ///
    /// ## 参数
    /// - `path`: 目标文件路径（如 `/tmp/hnsw_index`）
    ///
    /// ## 原子性
    /// 先写入 `<path>.tmp`，再 rename 为 `<path>`，避免崩溃导致部分写入。
    ///
    /// ## 错误
    /// - 文件创建 / 写入失败：返回 `Storage` 错误
    pub fn save(&self, path: &Path) -> Result<()> {
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
        w.write_all(MAGIC)
            .map_err(|e| Error::storage(format!("写入 magic 失败: {e}"), "HnswIndex::save"))?;
        w.write_all(&VERSION.to_le_bytes()).map_err(|e| {
            Error::storage(format!("写入 version 失败: {e}"), "HnswIndex::save")
        })?;
        w.write_all(&(self.dim as u32).to_le_bytes()).map_err(|e| {
            Error::storage(format!("写入 dim 失败: {e}"), "HnswIndex::save")
        })?;
        w.write_all(&(self.vectors.len() as u32).to_le_bytes()).map_err(|e| {
            Error::storage(format!("写入 count 失败: {e}"), "HnswIndex::save")
        })?;

        // 每条向量
        for (i, vector) in self.vectors.iter().enumerate() {
            let entity_id = &self.id_map[i];
            let id_bytes = entity_id.as_bytes();
            w.write_all(&(id_bytes.len() as u32).to_le_bytes())
                .map_err(|e| Error::storage(format!("写入 id_len 失败: {e}"), "HnswIndex::save"))?;
            w.write_all(id_bytes)
                .map_err(|e| Error::storage(format!("写入 id 失败: {e}"), "HnswIndex::save"))?;
            // 向量按 f32 LE 顺序写入
            for f in vector {
                w.write_all(&f.to_le_bytes()).map_err(|e| {
                    Error::storage(format!("写入 vector 失败: {e}"), "HnswIndex::save")
                })?;
            }
        }
        w.flush()
            .map_err(|e| Error::storage(format!("flush 失败: {e}"), "HnswIndex::save"))?;
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

    /// 从磁盘加载
    ///
    /// 读取 `save()` 写入的二进制文件，新建 HnswIndex 并重新 insert 所有向量
    /// （重建 HNSW 图）。文件格式见模块级文档。
    ///
    /// ## 参数
    /// - `path`: 源文件路径（如 `/tmp/hnsw_index`）
    ///
    /// ## 返回
    /// 加载完毕的 HnswIndex（`len()` 与 save 时一致）
    ///
    /// ## 错误
    /// - 文件读取失败 / magic 不匹配 / version 不支持：返回 `Storage` 错误
    /// - id UTF-8 解码失败：返回 `Parse` 错误
    /// - 重建图时维度不匹配：返回 `InvalidArgument` 错误（理论不应发生）
    pub fn load(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(|e| {
            Error::storage(
                format!("打开文件失败: {e} (path={})", path.display()),
                "HnswIndex::load",
            )
        })?;
        let mut r = BufReader::new(file);

        // 文件头
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)
            .map_err(|e| Error::storage(format!("读取 magic 失败: {e}"), "HnswIndex::load"))?;
        if &magic != MAGIC {
            return Err(Error::storage(
                format!("无效的文件头: {:?}（期望 {:?}）", magic, MAGIC),
                "HnswIndex::load",
            ));
        }

        let mut buf4 = [0u8; 4];
        r.read_exact(&mut buf4)
            .map_err(|e| Error::storage(format!("读取 version 失败: {e}"), "HnswIndex::load"))?;
        let version = u32::from_le_bytes(buf4);
        if version != VERSION {
            return Err(Error::storage(
                format!("不支持的版本: {version}（当前仅支持 {VERSION}）"),
                "HnswIndex::load",
            ));
        }

        r.read_exact(&mut buf4)
            .map_err(|e| Error::storage(format!("读取 dim 失败: {e}"), "HnswIndex::load"))?;
        let dim = u32::from_le_bytes(buf4) as usize;

        r.read_exact(&mut buf4)
            .map_err(|e| Error::storage(format!("读取 count 失败: {e}"), "HnswIndex::load"))?;
        let count = u32::from_le_bytes(buf4) as usize;

        // 新建 HnswIndex（max_elements = count，刚好容纳所有向量）
        let mut index = Self::new(count.max(1), dim);

        // 读取每条向量并 insert（重建 HNSW 图）
        for _ in 0..count {
            r.read_exact(&mut buf4).map_err(|e| {
                Error::storage(format!("读取 id_len 失败: {e}"), "HnswIndex::load")
            })?;
            let id_len = u32::from_le_bytes(buf4) as usize;
            let mut id_bytes = vec![0u8; id_len];
            r.read_exact(&mut id_bytes).map_err(|e| {
                Error::storage(format!("读取 id 失败: {e}"), "HnswIndex::load")
            })?;
            let entity_id = String::from_utf8(id_bytes).map_err(|e| {
                Error::parse(format!("id UTF-8 解码失败: {e}"), "HnswIndex::load")
            })?;

            let mut vector = vec![0f32; dim];
            let mut v_bytes = vec![0u8; dim * 4];
            r.read_exact(&mut v_bytes).map_err(|e| {
                Error::storage(format!("读取 vector 失败: {e}"), "HnswIndex::load")
            })?;
            for (i, chunk) in v_bytes.chunks_exact(4).enumerate() {
                vector[i] = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            }
            index.insert(&vector, &entity_id)?;
        }

        index.persist_path = Some(path.to_path_buf());
        Ok(index)
    }

    /// 元素数量
    ///
    /// 返回当前索引中的向量数（与 `id_map.len()` / `vectors.len()` 一致）。
    pub fn len(&self) -> usize {
        // hnsw_rs::Hnsw::get_nb_point 返回已插入的点数
        self.hnsw.get_nb_point()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ---------------------------------------------------------------------------
// Step3VectorIndex trait 实现（11.1.2 定义，11.6.1 集成）
// ---------------------------------------------------------------------------

/// `Step3VectorIndex` trait 实现（spec §三 11.1.2 + 11.6.1）
///
/// ## 语义转换
/// - `HnswIndex::search` 返回 `(entity_id, distance)`，distance 越小越相似
/// - `Step3VectorIndex::search_top_k` 期望 `(id, score)`，score 越大越相似
/// - 转换公式：`score = 1.0 - distance`（cosine 相似度 = 1 - cosine 距离）
///
/// ## 错误处理
/// `HnswIndex::search` 返回 `Result`，但 `Step3VectorIndex::search_top_k` 不返回
/// `Result`。错误时（如维度不匹配）返回空 Vec，并记录 `log::warn!`。
impl Step3VectorIndex for HnswIndex {
    fn search_top_k(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        match self.search(query, k) {
            Ok(results) => results
                .into_iter()
                .map(|(id, distance)| {
                    // distance → similarity（cosine 相似度 = 1 - cosine 距离）
                    let similarity = 1.0 - distance;
                    (id, similarity)
                })
                .collect(),
            Err(e) => {
                log::warn!("HnswIndex::search_top_k 失败: {}", e);
                Vec::new()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_index() {
        let index = HnswIndex::new(100, 64);
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_insert_increases_len() {
        let mut index = HnswIndex::new(100, 64);
        index
            .insert(&vec![1.0; 64], "ent-1")
            .expect("insert 失败");
        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());
    }

    #[test]
    fn test_insert_dimension_mismatch_returns_error() {
        let mut index = HnswIndex::new(100, 64);
        let result = index.insert(&vec![1.0; 32], "ent-1");
        assert!(result.is_err(), "维度不匹配应返回错误");
    }

    #[test]
    fn test_search_dimension_mismatch_returns_error() {
        let index = HnswIndex::new(100, 64);
        let result = index.search(&vec![1.0; 32], 5);
        assert!(result.is_err(), "维度不匹配应返回错误");
    }

    #[test]
    fn test_step3_vector_index_impl_converts_distance_to_similarity() {
        let mut index = HnswIndex::new(100, 8);
        // 插入 1 条向量
        let mut v = vec![0.0f32; 8];
        v[0] = 1.0;
        index.insert(&v, "ent-1").expect("insert 失败");

        // 查询同向量：distance 应 ≈ 0，similarity 应 ≈ 1
        let results = Step3VectorIndex::search_top_k(&index, &v, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "ent-1");
        // similarity = 1 - distance ≈ 1 - 0 = 1
        assert!(
            (results[0].1 - 1.0).abs() < 1e-5,
            "similarity 应 ≈ 1.0，实际: {}",
            results[0].1
        );
    }
}
