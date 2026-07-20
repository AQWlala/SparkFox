//! RAG 端到端集成测试（spec 1.0 Task 3.7）
//!
//! ## 测试场景
//! 1. **纯 MockEmbedder 流程**：分块 → 嵌入 → 向量召回 → FTS5 关键词召回 → RRF 融合 → 引用注入
//! 2. **BgeEmbedder 流程（可选）**：若本地存在 bge-small-zh-v1.5 模型文件则执行真实嵌入
//!    若模型文件不存在则 skip（不 fail）
//! 3. **sparkfox-store Store 适配**：通过 `StoreAdapter` 桥接 `sparkfox_store::Store`
//!    验证 RagEngine 与真实 Store 的互操作（需要 sqlite-vec 扩展加载成功）

#![forbid(unsafe_code)]

use std::sync::Arc;

use sparkfox_core::Result;
use sparkfox_knowledge::rag::{Embedder as LocalEmbedder, VectorStore as LocalVectorStore};
use sparkfox_knowledge::{
    inject_citations, Chunker, Citation, CitationSource, InMemoryVectorStore, MockEmbedder,
    RagEngine, SearchSource,
};

// ---------------------------------------------------------------------------
// 工具：桥接 sparkfox_embedding::Embedder → sparkfox_knowledge::rag::Embedder
// ---------------------------------------------------------------------------

/// Embedder 适配器：将 `sparkfox_embedding::Embedder` 桥接到本 crate 的 `Embedder` trait
struct EmbedderAdapter<E>(E);

impl<E: sparkfox_embedding::Embedder> LocalEmbedder for EmbedderAdapter<E> {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.0.embed(text)
    }
    fn dim(&self) -> usize {
        self.0.dim()
    }
    fn model_name(&self) -> &str {
        self.0.model_name()
    }
}

// ---------------------------------------------------------------------------
// 场景 1：纯 MockEmbedder 端到端流程（始终运行，无外部依赖）
// ---------------------------------------------------------------------------

/// 构造测试用 RAG 引擎（MockEmbedder + InMemoryVectorStore）
fn make_mock_engine() -> RagEngine {
    RagEngine::new(
        Arc::new(InMemoryVectorStore::new()),
        Arc::new(MockEmbedder::new()),
        Chunker::with_config(80, 10, "\n\n"),
    )
    .expect("RagEngine 创建失败")
}

#[test]
fn rag_e2e_mock_full_pipeline() {
    let engine = make_mock_engine();

    // 步骤 1：索引 5 篇文档
    let docs = [
        ("doc_rust", "Rust 是一门系统级编程语言，强调内存安全与并发性能。所有权机制是 Rust 的核心特性。"),
        ("doc_python", "Python 是动态类型语言，适合数据科学与机器学习。语法简洁易读。"),
        ("doc_go", "Go 语言由 Google 开发，主打并发模型与快速编译。goroutine 是轻量级线程。"),
        ("doc_ml", "机器学习是人工智能的分支，包括监督学习与无监督学习。深度学习使用神经网络。"),
        ("doc_web", "Web 开发涉及 HTML CSS JavaScript。前端框架包括 React Vue Angular。"),
    ];
    for (id, content) in &docs {
        let chunks = engine.index_document(id, content).expect("索引失败");
        assert!(!chunks.is_empty(), "文档 {id} 应至少分一块");
    }
    assert_eq!(engine.chunk_count().unwrap(), docs.len(), "每文档应一块");

    // 步骤 2：向量召回
    let vector_hits = engine.vector_search("Rust 语言", 5).expect("向量召回失败");
    assert!(!vector_hits.is_empty(), "向量召回应返回结果");
    for hit in &vector_hits {
        match hit.source {
            SearchSource::Vector(s) => assert!(s >= 0.0, "向量分数应非负"),
            _ => panic!("vector_search 的 source 应为 Vector"),
        }
    }

    // 步骤 3：关键词召回（FTS5）
    let keyword_hits = engine.keyword_search("Rust", 5).expect("关键词召回失败");
    assert!(!keyword_hits.is_empty(), "关键词召回应返回结果");
    // top-1 应为 doc_rust
    assert_eq!(
        keyword_hits[0].chunk.metadata.doc_id, "doc_rust",
        "关键词 'Rust' top-1 应为 doc_rust"
    );

    // 步骤 4：RRF 融合
    let fused = engine.rrf_fuse(vector_hits.clone(), keyword_hits.clone(), 10);
    assert!(!fused.is_empty(), "RRF 融合应返回结果");
    for hit in &fused {
        match hit.source {
            SearchSource::Rrf(s) => assert!(s > 0.0, "RRF 分数应正"),
            _ => panic!("rrf_fuse 的 source 应为 Rrf"),
        }
    }

    // 步骤 5：hybrid_search 一站式
    let hybrid = engine.hybrid_search("机器学习", 5).expect("混合检索失败");
    assert!(!hybrid.fused_hits.is_empty());

    // 步骤 6：引用注入
    let top_hit = &fused[0];
    let citation = Citation {
        kdoc_id: format!("kdoc_{}", top_hit.chunk.metadata.doc_id),
        chunk_id: top_hit.chunk.id.clone(),
        span_start: top_hit.chunk.start_offset,
        span_end: top_hit.chunk.end_offset,
        score: top_hit.score,
        source: match top_hit.source {
            SearchSource::Vector(_) => CitationSource::Vector,
            SearchSource::Keyword(_) => CitationSource::Keyword,
            SearchSource::Rrf(_) => CitationSource::Rrf,
        },
        page: None,
    };
    let marker = citation.to_marker();
    let text = format!("根据文档{marker}所述，Rust 性能优秀。");
    let injected = inject_citations(&text);
    assert_eq!(injected.text, "根据文档[1]所述，Rust 性能优秀。");
    assert_eq!(injected.citations.len(), 1);
    assert_eq!(injected.citations[0].citation.kdoc_id, citation.kdoc_id);
}

#[test]
fn rag_e2e_mock_chunking_long_document() {
    let engine = make_mock_engine();
    // 长文档：500 字符 → 多块
    let long_text: String = std::iter::repeat('x').take(500).collect();
    let chunks = engine.index_document("long_doc", &long_text).expect("索引失败");
    assert!(
        chunks.len() > 1,
        "500 字符文档（chunk_size=80）应分多块，实际 {} 块",
        chunks.len()
    );
    assert_eq!(engine.chunk_count().unwrap(), chunks.len());
}

#[test]
fn rag_e2e_mock_delete_and_reindex() {
    let engine = make_mock_engine();
    engine
        .index_document("doc1", "待删除文档内容。")
        .expect("索引失败");
    assert_eq!(engine.chunk_count().unwrap(), 1);

    // 删除
    engine.delete_document("doc1").expect("删除失败");
    assert_eq!(engine.chunk_count().unwrap(), 0);
    let hits = engine.keyword_search("待删除", 10).expect("检索失败");
    assert!(hits.is_empty(), "删除后应无结果");

    // 重新索引
    engine
        .index_document("doc1", "重新索引的内容。")
        .expect("重索引失败");
    assert_eq!(engine.chunk_count().unwrap(), 1);
}

#[test]
fn rag_e2e_mock_rrf_double_hit_ranks_higher() {
    let engine = make_mock_engine();
    engine
        .index_document("doc_overlap", "Rust 语言内存安全 Rust 性能优秀。")
        .expect("索引失败");
    engine
        .index_document("doc_unique", "Go 语言并发模型。")
        .expect("索引失败");

    // 查询 "Rust" → doc_overlap 应同时被向量与关键词命中 → RRF 分数更高
    let result = engine.hybrid_search("Rust", 5).expect("混合检索失败");
    assert!(!result.fused_hits.is_empty());
    // top-1 应为 doc_overlap
    assert_eq!(
        result.fused_hits[0].chunk.metadata.doc_id, "doc_overlap",
        "双路命中的文档应排第一"
    );
}

// ---------------------------------------------------------------------------
// 场景 2：BgeEmbedder 端到端流程（可选，模型文件存在时运行）
// ---------------------------------------------------------------------------

/// 尝试加载 BgeEmbedder，失败则返回 None（测试调用方应 skip）
fn try_load_bge_embedder() -> Option<Arc<EmbedderAdapter<sparkfox_embedding::BgeEmbedder>>> {
    use sparkfox_embedding::{BgeEmbedder, ModelVariant};
    match BgeEmbedder::try_load(ModelVariant::BgeSmallZh) {
        Ok(embedder) => {
            eprintln!("✓ BgeEmbedder 加载成功，执行真实嵌入 E2E 测试");
            Some(Arc::new(EmbedderAdapter(embedder)))
        }
        Err(e) => {
            eprintln!("⚠ BgeEmbedder 加载失败，跳过真实嵌入 E2E 测试: {e}");
            None
        }
    }
}

#[test]
fn rag_e2e_bge_real_embedding() {
    let embedder = match try_load_bge_embedder() {
        Some(e) => e,
        None => {
            eprintln!("skip: BgeEmbedder 模型文件未就绪（此跳过不是失败）");
            return;
        }
    };

    let engine = RagEngine::new(
        Arc::new(InMemoryVectorStore::new()),
        embedder,
        Chunker::with_config(200, 20, "\n\n"),
    )
    .expect("RagEngine 创建失败");

    // 索引 3 篇文档
    engine
        .index_document("doc1", "Rust 是系统级语言，内存安全。")
        .expect("索引失败");
    engine
        .index_document("doc2", "Python 适合数据科学。")
        .expect("索引失败");
    engine
        .index_document("doc3", "Go 主打并发与快速编译。")
        .expect("索引失败");

    // 向量检索：语义相近的应排前
    let hits = engine
        .vector_search("系统编程语言", 3)
        .expect("向量召回失败");
    assert!(!hits.is_empty());
    // doc1（Rust 系统级语言）应排第一
    assert_eq!(
        hits[0].chunk.metadata.doc_id, "doc1",
        "语义检索 top-1 应为 doc1（Rust），实际: {}",
        hits[0].chunk.metadata.doc_id
    );
}

// ---------------------------------------------------------------------------
// 场景 3：sparkfox-store Store 适配（需要 sqlite-vec 扩展）
// ---------------------------------------------------------------------------

/// StoreAdapter：将 sparkfox_store::Store 的 vector_insert/search 桥接到 VectorStore trait
///
/// 注意：
/// - `Store` 含 `rusqlite::Connection`，后者内部用 `RefCell` 故非 `Sync`。
///   本适配器用 `Mutex<Store>` 包裹，使其满足 `Send + Sync`。
/// - Store::vector_insert 需要 MemoryLayer 参数与 ref_id（字符串），
///   Store::vector_search 返回 (i64, f32)（distance，越小越相似）。
/// 适配时：
/// - chunk_id（字符串）→ ref_id（字符串）
/// - search 返回的 distance → 相似度 score = 1.0 - distance
struct StoreAdapter {
    store: std::sync::Mutex<sparkfox_store::Store>,
    model_name: String,
}

impl LocalVectorStore for StoreAdapter {
    fn upsert(&self, id: &str, vector: &[f32]) -> Result<()> {
        use sparkfox_memory::MemoryLayer;
        let store = self
            .store
            .lock()
            .map_err(|e| sparkfox_core::Error::storage(format!("Store 锁获取失败: {e}"), "StoreAdapter::upsert"))?;
        // L0Raw → vec_l0 表（原始分块向量）
        store.vector_insert(MemoryLayer::L0Raw, id, &self.model_name, vector)
    }

    fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        use sparkfox_memory::MemoryLayer;
        let store = self
            .store
            .lock()
            .map_err(|e| sparkfox_core::Error::storage(format!("Store 锁获取失败: {e}"), "StoreAdapter::search"))?;
        // Store::vector_search 返回 (i64 rowid, f32 distance)
        // 需要将 rowid → ref_id（字符串），并 distance → score = 1 - distance
        let raw_hits = store.vector_search(MemoryLayer::L0Raw, query, k)?;
        // rowid → ref_id 映射需要查 memory_vectors 表
        let conn = store.conn();
        let mut out = Vec::with_capacity(raw_hits.len());
        for (rowid, distance) in raw_hits {
            let ref_id: Option<String> = conn
                .query_row(
                    "SELECT ref_id FROM memory_vectors WHERE id = ?1",
                    rusqlite::params![rowid],
                    |r| r.get(0),
                )
                .ok();
            if let Some(id) = ref_id {
                // sqlite-vec cosine distance ∈ [0, 2]，相似度 = 1 - distance
                let score = 1.0 - distance;
                out.push((id, score));
            }
        }
        // 按相似度降序
        out.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(out)
    }

    fn len(&self) -> usize {
        let Ok(store) = self.store.lock() else {
            return 0;
        };
        let conn = store.conn();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_vectors WHERE layer = 0",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        count as usize
    }
}

#[test]
fn rag_e2e_store_adapter() {
    use sparkfox_store::{Store, StoreConfig};
    use tempfile::NamedTempFile;

    let tmp = NamedTempFile::new().expect("临时文件创建失败");
    let store = Store::open(StoreConfig::for_path(tmp.path())).expect("Store 打开失败");

    if !store.is_vec_loaded() {
        eprintln!("skip: sqlite-vec 扩展未加载，跳过 Store 适配测试（此跳过不是失败）");
        return;
    }

    let vector_store: Arc<dyn LocalVectorStore> = Arc::new(StoreAdapter {
        store: std::sync::Mutex::new(store),
        model_name: "bge-small-zh-v1.5".to_string(),
    });
    let embedder: Arc<dyn LocalEmbedder> = Arc::new(MockEmbedder::new());

    let engine = RagEngine::new(vector_store, embedder, Chunker::with_config(100, 10, "\n\n"))
        .expect("RagEngine 创建失败");

    engine
        .index_document("doc1", "Rust 语言内存安全。")
        .expect("索引失败");
    let hits = engine.vector_search("Rust", 5).expect("向量召回失败");
    assert!(!hits.is_empty(), "Store 适配后向量召回应返回结果");
}
