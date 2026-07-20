//! RAG 引擎（spec 1.0 Task 3.3-3.5）
//!
//! ## 架构
//! [`RagEngine`] 编排三层检索：
//! 1. **向量召回**（[`Embedder::embed`] + [`VectorStore::search`]）— 语义相似
//! 2. **关键词召回**（FTS5 全文检索）— 词面匹配
//! 3. **RRF 融合**（Reciprocal Rank Fusion, Cormack 2009）— 上述两路结果按排名融合
//!
//! ## 循环依赖规避
//! `sparkfox-store` 已依赖本 crate（用于 SAG DDL），本 crate **不**反向依赖 `sparkfox-store`。
//! 因此 [`Embedder`] / [`VectorStore`] trait 在本 crate 本地定义，
//! 与 `sparkfox_embedding::Embedder` / `sparkfox_store::Store` 同构。
//! 集成测试（`tests/rag_e2e.rs`）提供适配器桥接具体实现。
//!
//! ## 存储
//! - `sf_chunks` 表（普通 SQLite 表）：存储分块文本 + 偏移 + 元数据
//! - `sf_chunks_fts` 虚拟表（FTS5）：全文索引，tokenize=`unicode61`（支持中文）
//! - 向量存储：通过 [`VectorStore`] trait 抽象，默认 [`InMemoryVectorStore`]（暴力 cosine）
//!
//! ## RRF 融合公式（Task 3.5）
//! ```text
//! score_rrf(d) = Σ 1 / (k + rank_i(d))
//! ```
//! 其中 `k = 60`（标准值），`rank_i(d)` 是文档 `d` 在第 `i` 个召回通道中的排名（从 1 开始）。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use rusqlite::Connection;

use sparkfox_core::{Error, Result};

use crate::chunk::{Chunk, Chunker};

// ---------------------------------------------------------------------------
// Trait 抽象（解耦 sparkfox-store / sparkfox-embedding，规避循环依赖）
// ---------------------------------------------------------------------------

/// 嵌入器 trait — 与 `sparkfox_embedding::Embedder` 同构
///
/// 实现方需保证线程安全（`Send + Sync`）。
/// 集成测试中通过 `EmbedderAdapter` 桥接 `sparkfox_embedding::BgeEmbedder`。
pub trait Embedder: Send + Sync {
    /// 单条文本嵌入（返回 L2 归一化后的向量）
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    /// 嵌入维度
    fn dim(&self) -> usize;
    /// 模型名（如 "BAAI/bge-small-zh-v1.5"）
    fn model_name(&self) -> &str;
}

/// 向量存储 trait — 与 `sparkfox_store::Store::vector_insert/search` 同构
///
/// `search` 返回 `(id, score)` 列表，`score` 语义：**相似度**（越大越相似，范围 `[-1, 1]`）。
/// 若底层返回距离（如 sqlite-vec 的 `distance`），实现方需转换为相似度 `score = 1.0 - distance`。
pub trait VectorStore: Send + Sync {
    /// 插入 / 更新向量（upsert 语义）
    fn upsert(&self, id: &str, vector: &[f32]) -> Result<()>;
    /// 检索 top-k 最近邻（按相似度降序）
    fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>>;
    /// 当前向量数
    fn len(&self) -> usize;
    /// 是否为空
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 关键词存储 trait — FTS5 全文检索抽象
///
/// v1.0.0 由 [`RagEngine`] 内部直接管理 FTS5 表，此 trait 预留给 v1.1.0+ 替换实现
/// （如外部 Elasticsearch / Meilisearch）。
pub trait KeywordStore: Send + Sync {
    /// 插入 / 更新文本（upsert 语义）
    fn upsert_text(&self, id: &str, text: &str) -> Result<()>;
    /// 关键词检索 top-k（按 BM25 rank 升序，即相关度降序）
    fn search(&self, query: &str, k: usize) -> Result<Vec<(String, f32)>>;
}

// ---------------------------------------------------------------------------
// 内置实现：InMemoryVectorStore（暴力 cosine，用于测试 / 小规模场景）
// ---------------------------------------------------------------------------

/// 内存向量存储 — 暴力 cosine 扫描
///
/// 适用场景：
/// - 测试（无需 sqlite-vec 扩展）
/// - 小规模知识库（< 1000 向量，延迟 < 1ms）
/// - sqlite-vec 加载失败时的降级方案
#[derive(Debug, Default)]
pub struct InMemoryVectorStore {
    inner: RwLock<HashMap<String, Vec<f32>>>,
}

impl InMemoryVectorStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl VectorStore for InMemoryVectorStore {
    fn upsert(&self, id: &str, vector: &[f32]) -> Result<()> {
        let mut map = self
            .inner
            .write()
            .map_err(|e| Error::storage(format!("写锁获取失败: {e}"), "InMemoryVectorStore::upsert"))?;
        map.insert(id.to_string(), vector.to_vec());
        Ok(())
    }

    fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        let map = self
            .inner
            .read()
            .map_err(|e| Error::storage(format!("读锁获取失败: {e}"), "InMemoryVectorStore::search"))?;
        let mut scored: Vec<(String, f32)> = map
            .iter()
            .map(|(id, v)| (id.clone(), cosine_sim(query, v)))
            .collect();
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(k);
        Ok(scored)
    }

    fn len(&self) -> usize {
        self.inner.read().map(|m| m.len()).unwrap_or(0)
    }
}

/// 余弦相似度。零向量返回 0 避免 NaN。
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

// ---------------------------------------------------------------------------
// 内置实现：MockEmbedder（确定性嵌入，用于单元测试 / E2E 测试无模型时）
// ---------------------------------------------------------------------------

/// Mock 嵌入器 — 基于字符频率的确定性嵌入
///
/// 用于测试场景：无需下载 bge 模型文件即可走完整 RAG 流程。
/// 嵌入维度固定 64，每个维度对应该字符码 `c % 64` 的频率（归一化后）。
/// 相同文本产生相同向量，相似文本（字符重叠多）产生相似向量。
#[derive(Debug, Default)]
pub struct MockEmbedder {
    dim: usize,
}

impl MockEmbedder {
    pub fn new() -> Self {
        Self { dim: 64 }
    }

    pub fn with_dim(dim: usize) -> Self {
        assert!(dim > 0, "dim 必须 > 0");
        Self { dim }
    }
}

impl Embedder for MockEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut vec = vec![0.0f32; self.dim];
        for c in text.chars() {
            let idx = (c as usize) % self.dim;
            vec[idx] += 1.0;
        }
        // L2 归一化（与 BgeEmbedder 保持一致）
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut vec {
                *x /= norm;
            }
        }
        Ok(vec)
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        "mock-embedder-v1"
    }
}

// ---------------------------------------------------------------------------
// SearchResult / SearchSource
// ---------------------------------------------------------------------------

/// 检索结果来源（标记命中通道）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchSource {
    /// 向量召回（附原始相似度分数）
    Vector(f32),
    /// 关键词召回（附 BM25 分数）
    Keyword(f32),
    /// RRF 融合（附融合后分数）
    Rrf(f32),
}

/// 单条检索命中
#[derive(Debug, Clone)]
pub struct SearchHit {
    /// 命中的分块
    pub chunk: Chunk,
    /// 融合 / 最终分数（越大越相关）
    pub score: f32,
    /// 命中来源
    pub source: SearchSource,
}

/// 混合检索结果（向量 + 关键词 + RRF 融合 top-k）
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    /// 向量召回 top-k
    pub vector_hits: Vec<SearchHit>,
    /// 关键词召回 top-k
    pub keyword_hits: Vec<SearchHit>,
    /// RRF 融合 top-k
    pub fused_hits: Vec<SearchHit>,
}

// ---------------------------------------------------------------------------
// RagEngine
// ---------------------------------------------------------------------------

/// RAG 引擎 — 编排分块 / 嵌入 / 向量召回 / FTS5 关键词召回 / RRF 融合
///
/// 线程安全（内部状态由 `RwLock` / `Arc` 保护），可在多线程间共享。
///
/// ## 用法
/// ```no_run
/// use sparkfox_knowledge::{Chunker, InMemoryVectorStore, MockEmbedder, RagEngine};
/// use std::sync::Arc;
///
/// let engine = RagEngine::new(
///     Arc::new(InMemoryVectorStore::new()),
///     Arc::new(MockEmbedder::new()),
///     Chunker::new(),
/// ).unwrap();
/// engine.index_document("doc1", "知识库内容…").unwrap();
/// let hits = engine.rrf_search("查询", 10).unwrap();
/// ```
pub struct RagEngine {
    /// SQLite 连接（管理 sf_chunks 表 + sf_chunks_fts FTS5 虚表）
    conn: Connection,
    /// 向量存储后端
    vector_store: Arc<dyn VectorStore>,
    /// 嵌入器
    embedder: Arc<dyn Embedder>,
    /// 分块器
    chunker: Chunker,
}

impl RagEngine {
    /// 创建 RAG 引擎（内存 SQLite + 自定义后端）
    ///
    /// 内部创建内存 SQLite 连接管理 `sf_chunks` 表与 `sf_chunks_fts` FTS5 虚表。
    /// 向量存储与嵌入器由调用方注入（解耦具体后端）。
    pub fn new(
        vector_store: Arc<dyn VectorStore>,
        embedder: Arc<dyn Embedder>,
        chunker: Chunker,
    ) -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| Error::storage(format!("打开内存 SQLite 失败: {e}"), "RagEngine::new"))?;
        let engine = Self {
            conn,
            vector_store,
            embedder,
            chunker,
        };
        engine.init_schema()?;
        Ok(engine)
    }

    /// 初始化 sf_chunks 表 + sf_chunks_fts FTS5 虚表
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sf_chunks (
                id TEXT PRIMARY KEY,
                doc_id TEXT NOT NULL,
                chunk_idx INTEGER NOT NULL,
                content TEXT NOT NULL,
                start_offset INTEGER NOT NULL,
                end_offset INTEGER NOT NULL,
                metadata TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_sf_chunks_doc ON sf_chunks(doc_id);

            CREATE VIRTUAL TABLE IF NOT EXISTS sf_chunks_fts USING fts5(
                content,
                chunk_id UNINDEXED,
                tokenize='unicode61'
            );
            "#,
        )?;
        log::debug!("RagEngine schema 初始化完成（sf_chunks + sf_chunks_fts）");
        Ok(())
    }

    /// 索引文档：分块 → 嵌入 → 写入向量存储 + sf_chunks + sf_chunks_fts
    ///
    /// 返回生成的分块列表（含 ID 与偏移）。
    /// 同一 `doc_id` 重复索引会先删除旧分块（含向量 + FTS5 索引）。
    pub fn index_document(&self, doc_id: &str, content: &str) -> Result<Vec<Chunk>> {
        // 步骤 1：删除旧分块（幂等重索引）
        self.delete_document(doc_id)?;

        // 步骤 2：分块
        let chunks = self.chunker.chunk_with_doc_id(content, doc_id);
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        // 步骤 3：逐块嵌入 + 写入
        let metadata_json = serde_json::json!({
            "doc_id": doc_id,
            "indexed_at": now_iso8601(),
        })
        .to_string();

        for chunk in &chunks {
            // 写 sf_chunks
            self.conn.execute(
                "INSERT INTO sf_chunks (id, doc_id, chunk_idx, content, start_offset, end_offset, metadata) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    chunk.id,
                    doc_id,
                    chunk.metadata.index as i64,
                    chunk.content,
                    chunk.start_offset as i64,
                    chunk.end_offset as i64,
                    metadata_json,
                ],
            )?;

            // 写 sf_chunks_fts（content + chunk_id）
            self.conn.execute(
                "INSERT INTO sf_chunks_fts (content, chunk_id) VALUES (?1, ?2)",
                rusqlite::params![chunk.content, chunk.id],
            )?;

            // 嵌入 + 写向量存储
            let vec = self.embedder.embed(&chunk.content)?;
            self.vector_store.upsert(&chunk.id, &vec)?;
        }

        log::info!(
            "索引文档 doc_id={} 完成：{} 块（每块 {} 字符上限）",
            doc_id,
            chunks.len(),
            self.chunker.chunk_size()
        );
        Ok(chunks)
    }

    /// 删除文档的所有分块（含向量 + FTS5 索引）
    pub fn delete_document(&self, doc_id: &str) -> Result<()> {
        // 查出所有 chunk_id（用于删 FTS5 与向量）
        let chunk_ids: Vec<String> = {
            let mut stmt = self.conn.prepare(
                "SELECT id FROM sf_chunks WHERE doc_id = ?1",
            )?;
            let rows = stmt.query_map(rusqlite::params![doc_id], |r| r.get::<_, String>(0))?;
            let mut ids = Vec::new();
            for r in rows {
                ids.push(r?);
            }
            ids
        };

        // 删 sf_chunks
        self.conn.execute(
            "DELETE FROM sf_chunks WHERE doc_id = ?1",
            rusqlite::params![doc_id],
        )?;

        // 删 sf_chunks_fts（FTS5 删除语法：DELETE FROM ... WHERE chunk_id = ?）
        for id in &chunk_ids {
            self.conn.execute(
                "DELETE FROM sf_chunks_fts WHERE chunk_id = ?1",
                rusqlite::params![id],
            )?;
            // 向量存储无法按 id 删除（trait 未暴露 delete），v1.0.0 容忍残留
            // v1.1.0+ 在 VectorStore trait 增加 delete 方法
        }

        if !chunk_ids.is_empty() {
            log::debug!("删除文档 doc_id={} 的 {} 个分块", doc_id, chunk_ids.len());
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // 检索 API
    // -----------------------------------------------------------------------

    /// 向量召回 top-k（spec 1.0 Task 3.3）
    ///
    /// 流程：query → embed → VectorStore.search → 按 chunk_id 加载内容
    pub fn vector_search(&self, query: &str, k: usize) -> Result<Vec<SearchHit>> {
        let query_vec = self.embedder.embed(query)?;
        let hits = self.vector_store.search(&query_vec, k)?;
        let mut results = Vec::with_capacity(hits.len());
        for (chunk_id, score) in hits {
            if let Some(chunk) = self.load_chunk(&chunk_id)? {
                results.push(SearchHit {
                    chunk,
                    score,
                    source: SearchSource::Vector(score),
                });
            }
        }
        Ok(results)
    }

    /// 关键词召回 top-k（spec 1.0 Task 3.4，FTS5）
    ///
    /// FTS5 `rank` 越小越相关（BM25 距离），取负转为分数（越大越相关）。
    pub fn keyword_search(&self, query: &str, k: usize) -> Result<Vec<SearchHit>> {
        // FTS5 查询：对中文用双引号包裹整体短语，避免被当布尔操作符解析
        // 简单策略：用 OR 连接各 token，提升召回率
        let fts_query = self.build_fts_query(query);
        let mut stmt = self.conn.prepare(
            "SELECT chunk_id, rank FROM sf_chunks_fts \
             WHERE sf_chunks_fts MATCH ?1 \
             ORDER BY rank \
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![fts_query, k as i64], |r| {
            let chunk_id: String = r.get(0)?;
            let rank: f32 = r.get(1)?;
            Ok((chunk_id, rank))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (chunk_id, rank) = row?;
            // FTS5 rank 为负的 BM25 分数（越小越相关），取负转为正分数
            let score = -rank;
            if let Some(chunk) = self.load_chunk(&chunk_id)? {
                results.push(SearchHit {
                    chunk,
                    score,
                    source: SearchSource::Keyword(score),
                });
            }
        }
        Ok(results)
    }

    /// RRF 融合（spec 1.0 Task 3.5）
    ///
    /// 公式：`score_rrf(d) = Σ 1 / (k + rank_i(d))`，`k = 60`
    ///
    /// 输入为已完成的向量 / 关键词召回结果（各自已排序）。
    /// 输出为融合后按 RRF 分数降序的 top-k。
    pub fn rrf_fuse(
        &self,
        vector_hits: Vec<SearchHit>,
        keyword_hits: Vec<SearchHit>,
        k: usize,
    ) -> Vec<SearchHit> {
        const RRF_K: f32 = 60.0;

        // chunk_id → (累计 rrf 分数, 最佳原始分数, 最佳来源, chunk)
        let mut scores: HashMap<String, (f32, f32, SearchSource, Option<Chunk>)> =
            HashMap::new();

        // 向量通道排名（从 1 开始）
        for (rank, hit) in vector_hits.iter().enumerate() {
            let rrf_score = 1.0 / (RRF_K + (rank as f32 + 1.0));
            scores
                .entry(hit.chunk.id.clone())
                .and_modify(|(s, best, src, c)| {
                    *s += rrf_score;
                    if hit.score > *best {
                        *best = hit.score;
                        *src = hit.source;
                    }
                    *c = Some(hit.chunk.clone());
                })
                .or_insert((
                    rrf_score,
                    hit.score,
                    hit.source,
                    Some(hit.chunk.clone()),
                ));
        }

        // 关键词通道排名
        for (rank, hit) in keyword_hits.iter().enumerate() {
            let rrf_score = 1.0 / (RRF_K + (rank as f32 + 1.0));
            scores
                .entry(hit.chunk.id.clone())
                .and_modify(|(s, best, src, c)| {
                    *s += rrf_score;
                    if hit.score > *best {
                        *best = hit.score;
                        *src = hit.source;
                    }
                    *c = Some(hit.chunk.clone());
                })
                .or_insert((
                    rrf_score,
                    hit.score,
                    hit.source,
                    Some(hit.chunk.clone()),
                ));
        }

        // 转换为 SearchHit 列表并按 RRF 分数降序
        let mut fused: Vec<SearchHit> = scores
            .into_iter()
            .filter_map(|(_chunk_id, (rrf, _, _, chunk_opt))| {
                chunk_opt.map(|chunk| SearchHit {
                    chunk,
                    score: rrf,
                    source: SearchSource::Rrf(rrf),
                })
            })
            .collect();
        fused.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        fused.truncate(k);
        fused
    }

    /// 混合检索：向量 top-N + 关键词 top-N → RRF 融合 top-k（spec 1.0 Task 3.5）
    ///
    /// `N = max(50, k * 5)`（召回扩大，融合后截断到 k）
    pub fn hybrid_search(&self, query: &str, k: usize) -> Result<HybridSearchResult> {
        let recall_n = std::cmp::max(50, k * 5);
        let vector_hits = self.vector_search(query, recall_n)?;
        let keyword_hits = self.keyword_search(query, recall_n)?;
        let fused_hits = self.rrf_fuse(vector_hits.clone(), keyword_hits.clone(), k);
        Ok(HybridSearchResult {
            vector_hits,
            keyword_hits,
            fused_hits,
        })
    }

    /// RRF 混合检索的便捷封装（仅返回融合后 top-k）
    pub fn rrf_search(&self, query: &str, k: usize) -> Result<Vec<SearchHit>> {
        Ok(self.hybrid_search(query, k)?.fused_hits)
    }

    // -----------------------------------------------------------------------
    // 内部辅助
    // -----------------------------------------------------------------------

    /// 按 chunk_id 加载分块内容
    fn load_chunk(&self, chunk_id: &str) -> Result<Option<Chunk>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, doc_id, chunk_idx, content, start_offset, end_offset \
             FROM sf_chunks WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![chunk_id], |r| {
            let id: String = r.get(0)?;
            let doc_id: String = r.get(1)?;
            let chunk_idx: i64 = r.get(2)?;
            let content: String = r.get(3)?;
            let start_offset: i64 = r.get(4)?;
            let end_offset: i64 = r.get(5)?;
            Ok((id, doc_id, chunk_idx, content, start_offset, end_offset))
        })?;

        if let Some(row) = rows.next() {
            let (id, doc_id, chunk_idx, content, start_offset, end_offset) = row?;
            Ok(Some(Chunk {
                id,
                content,
                start_offset: start_offset as usize,
                end_offset: end_offset as usize,
                metadata: crate::chunk::ChunkMetadata {
                    doc_id,
                    index: chunk_idx as usize,
                    char_count: (end_offset - start_offset) as usize,
                },
            }))
        } else {
            Ok(None)
        }
    }

    /// 构建 FTS5 查询表达式
    ///
    /// 策略：将查询拆分为 token，用 OR 连接，提升召回率。
    /// 中文按字符切分（FTS5 unicode61 tokenizer 按非字母数字切分）。
    /// 每个 token 用双引号包裹，避免特殊字符被当布尔操作符。
    fn build_fts_query(&self, query: &str) -> String {
        // 按空白字符切分 token；FTS5 unicode61 tokenizer 自行处理标点
        let tokens: Vec<String> = query
            .split(|c: char| c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
            .collect();
        if tokens.is_empty() {
            // 退化为整体短语匹配（双引号包裹，转义内部双引号）
            format!("\"{}\"", query.replace('"', "\"\""))
        } else {
            tokens.join(" OR ")
        }
    }

    /// 当前向量数
    pub fn vector_count(&self) -> usize {
        self.vector_store.len()
    }

    /// 当前分块数（sf_chunks 表行数）
    pub fn chunk_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sf_chunks",
            [],
            |r| r.get(0),
        )?;
        Ok(count as usize)
    }
}

/// 当前时间 ISO 8601 字符串（无外部 chrono 依赖，用 SystemTime 格式化）
fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 简化格式：Unix 秒数（v1.0.0 不引入 chrono 依赖）
    format!("t{secs}")
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> RagEngine {
        RagEngine::new(
            Arc::new(InMemoryVectorStore::new()),
            Arc::new(MockEmbedder::new()),
            Chunker::with_config(50, 5, "\n\n"),
        )
        .expect("RagEngine 创建失败")
    }

    #[test]
    fn test_cosine_sim_basic() {
        assert!((cosine_sim(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!((cosine_sim(&[1.0, 0.0], &[0.0, 1.0])).abs() < 1e-6);
        assert_eq!(cosine_sim(&[], &[]), 0.0);
        assert_eq!(cosine_sim(&[0.0, 0.0], &[1.0, 0.0]), 0.0);
    }

    #[test]
    fn test_mock_embedder_deterministic() {
        let e = MockEmbedder::new();
        let v1 = e.embed("hello").unwrap();
        let v2 = e.embed("hello").unwrap();
        assert_eq!(v1, v2, "相同文本应产生相同向量");
        assert_eq!(v1.len(), 64);
    }

    #[test]
    fn test_in_memory_vector_store_upsert_search() {
        let store = InMemoryVectorStore::new();
        store.upsert("a", &[1.0, 0.0, 0.0]).unwrap();
        store.upsert("b", &[0.0, 1.0, 0.0]).unwrap();
        store.upsert("c", &[1.0, 1.0, 0.0]).unwrap();
        let hits = store.search(&[1.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0, "a");
        assert!(hits[0].1 > 0.99);
    }

    #[test]
    fn test_index_and_vector_search() {
        let engine = make_engine();
        let chunks = engine
            .index_document("doc1", "机器学习是人工智能的一个分支。深度学习是机器学习的子领域。")
            .unwrap();
        assert!(chunks.len() >= 1);
        assert_eq!(engine.chunk_count().unwrap(), chunks.len());

        // 向量检索
        let hits = engine.vector_search("机器学习", 5).unwrap();
        assert!(!hits.is_empty(), "向量检索应返回结果");
        assert!(hits[0].score > 0.0);
        match hits[0].source {
            SearchSource::Vector(s) => assert!(s > 0.0),
            _ => panic!("source 应为 Vector"),
        }
    }

    #[test]
    fn test_keyword_search_fts5() {
        let engine = make_engine();
        engine
            .index_document("doc1", "Rust 是一门系统级编程语言，强调内存安全与并发性能。")
            .unwrap();
        engine
            .index_document("doc2", "Python 是动态类型语言，适合数据科学。")
            .unwrap();

        let hits = engine.keyword_search("Rust", 10).unwrap();
        assert!(!hits.is_empty(), "关键词检索应返回结果");
        // top-1 应来自 doc1（含 "Rust"）
        assert!(
            hits[0].chunk.metadata.doc_id == "doc1",
            "top-1 应为 doc1，实际: {}",
            hits[0].chunk.metadata.doc_id
        );
        match hits[0].source {
            SearchSource::Keyword(_) => {}
            _ => panic!("source 应为 Keyword"),
        }
    }

    #[test]
    fn test_rrf_fuse_basic() {
        let engine = make_engine();
        // 构造两个通道的命中（chunk_id 部分重叠）
        let mk_chunk = |id: &str, doc: &str| Chunk {
            id: id.to_string(),
            content: format!("content-{id}"),
            start_offset: 0,
            end_offset: 10,
            metadata: crate::chunk::ChunkMetadata {
                doc_id: doc.to_string(),
                index: 0,
                char_count: 10,
            },
        };

        let vector_hits = vec![
            SearchHit {
                chunk: mk_chunk("c1", "d1"),
                score: 0.9,
                source: SearchSource::Vector(0.9),
            },
            SearchHit {
                chunk: mk_chunk("c2", "d1"),
                score: 0.8,
                source: SearchSource::Vector(0.8),
            },
        ];
        let keyword_hits = vec![
            SearchHit {
                chunk: mk_chunk("c2", "d1"),
                score: 5.0,
                source: SearchSource::Keyword(5.0),
            },
            SearchHit {
                chunk: mk_chunk("c3", "d1"),
                score: 3.0,
                source: SearchSource::Keyword(3.0),
            },
        ];

        let fused = engine.rrf_fuse(vector_hits, keyword_hits, 10);
        assert_eq!(fused.len(), 3, "应有 3 个唯一 chunk");

        // c2 同时出现在两路 → RRF 分数最高
        assert_eq!(fused[0].chunk.id, "c2", "c2 应排第一（双路命中）");
        match fused[0].source {
            SearchSource::Rrf(s) => {
                // c2: 1/(60+2) + 1/(60+1) = 1/62 + 1/61 ≈ 0.0325
                assert!(s > 0.03, "c2 RRF 分数应 > 0.03，实际 {s}");
            }
            _ => panic!("source 应为 Rrf"),
        }
    }

    #[test]
    fn test_hybrid_search() {
        let engine = make_engine();
        engine
            .index_document("doc1", "Rust 语言内存安全。Rust 性能优秀。")
            .unwrap();
        engine
            .index_document("doc2", "Go 语言并发模型优秀。")
            .unwrap();

        let result = engine.hybrid_search("Rust 语言", 5).unwrap();
        assert!(!result.vector_hits.is_empty());
        assert!(!result.keyword_hits.is_empty());
        assert!(!result.fused_hits.is_empty());

        // 融合后 top-1 应含 "Rust"
        assert!(
            result.fused_hits[0].chunk.content.contains("Rust"),
            "融合 top-1 应含 Rust"
        );
    }

    #[test]
    fn test_delete_document() {
        let engine = make_engine();
        engine
            .index_document("doc1", "待删除文档内容。")
            .unwrap();
        assert_eq!(engine.chunk_count().unwrap(), 1);

        engine.delete_document("doc1").unwrap();
        assert_eq!(engine.chunk_count().unwrap(), 0);

        // FTS5 也应清空
        let hits = engine.keyword_search("待删除", 10).unwrap();
        assert!(hits.is_empty(), "删除后 FTS5 应无结果");
    }

    #[test]
    fn test_reindex_idempotent() {
        let engine = make_engine();
        let content = "重新索引测试。同一文档多次索引不应产生重复分块。";
        engine.index_document("doc1", content).unwrap();
        let count1 = engine.chunk_count().unwrap();
        engine.index_document("doc1", content).unwrap();
        let count2 = engine.chunk_count().unwrap();
        assert_eq!(count1, count2, "重新索引不应增加分块数");
    }

    #[test]
    fn test_empty_query_vector_search() {
        let engine = make_engine();
        engine.index_document("doc1", "内容").unwrap();
        // 空查询不应 panic
        let hits = engine.vector_search("", 5).unwrap();
        // MockEmbedder 对空文本返回零向量 → cosine 全 0 → 仍返回结果但 score=0
        // InMemoryVectorStore 会返回所有向量（按 score 降序，全 0）
        assert!(hits.len() <= 1);
    }

    #[test]
    fn test_build_fts_query() {
        let engine = make_engine();
        assert_eq!(engine.build_fts_query("hello world"), "\"hello\" OR \"world\"");
        assert_eq!(engine.build_fts_query("测试"), "\"测试\"");
        // 空格分隔的中文
        assert_eq!(engine.build_fts_query("机器 学习"), "\"机器\" OR \"学习\"");
    }
}
