//! Sub-Step 11.1.1 — MULTI 8 步骨架（spec §三 11.1）
//!
//! ## 8 步流程定义
//! | Step | 名称                | 实现状态      | 输出字段                    |
//! |------|---------------------|---------------|-----------------------------|
//! | 1    | query 向量化        | ✅ mock embed | `MultiState.query_vec`     |
//! | 2    | query 实体抽取      | ✅ jieba+正则 | `MultiState.entities`      |
//! | 3    | 实体向量检索        | 🔧 stub       | `MultiState.entity_ids`    |
//! | 4    | 事件检索            | 🔧 stub       | `MultiState.candidates`    |
//! | 5    | 三策略占位          | 🔧 stub       | `MultiState.hits`          |
//! | 6    | 候选合并 + 去重     | 🔧 stub       | （并入 `hits`）            |
//! | 7    | Rerank 重排         | 🔧 stub       | （并入 `hits`）            |
//! | 8    | 返回 SearchResult   | ✅ 转换       | `SearchResult`             |
//!
//! ## MultiState
//! [`MultiState`] 是 8 步流程的中间状态容器，跨 step 传递。每个 step 接收
//! `MultiState` 并返回更新后的 `MultiState`（函数式风格，避免可变状态共享）。
//!
//! ## 与 10.8.2 的关系
//! 10.8.2 的 [`MultiStrategy`](super::MultiStrategy) 已实现简单 BFS 多跳扩展（max_hop=3
//! / score=1/hop 衰减）。11.1.1 在其之上引入 8 步骨架：
//! - Step1/Step2 真实实现（向量化 + 实体抽取）
//! - Step3-7 为 stub（占位待 11.2.x 实施 HnswIndex + 三策略 + Rerank）
//! - [`MultiStrategy`](super::MultiStrategy)::search 内部仍调用 10.8.2 的 BFS 作为
//!   Step5 的 "multi" 策略实现，10.8.2 的 7 个测试全部保持通过
//!
//! ## 后续 11.2.x 实施
//! - Step3：接入 HnswIndex（`sparkfox-store` 已提供）做 Top-K 实体向量检索
//! - Step4：基于 entity_ids 查询 `event_entity_relation` 表得到候选 event_ids
//! - Step5：三策略并行（multi / multi1 / hopllm）— multi 复用 10.8.2 BFS，
//!   multi1 加 LLM 辅助选路，hopllm 全程 LLM 推理
//! - Step6：合并三策略候选 + 按 event_id 去重
//! - Step7：调用 rerank 模型（如 bge-reranker）对候选重排

use crate::jieba_ner::{EntityMention, JiebaNer};
use crate::search::{SearchHit, SearchResult};

/// bge-small-zh embedding 维度
///
/// 后续 11.2.x 接入真实 embedding 模型时，此常量应与 `sparkfox-embedding::BgeEmbedder`
/// 的输出维度保持一致。当前 mock embedding 使用相同维度以保持 API 兼容。
const EMBED_DIM: usize = 384;

/// MULTI 8 步流程的中间状态容器
///
/// 跨 step 传递的不可变快照（每 step 返回新的 [`MultiState`]）。包含 8 步流程中
/// 逐步累积的字段：query → query_vec → entities → entity_ids → candidates → hits。
///
/// ## 字段来源
/// | 字段             | 来源 step            | 类型                |
/// |------------------|----------------------|---------------------|
/// | `query`          | 初始化               | `String`            |
/// | `query_vec`      | Step1                | `Vec<f32>`          |
/// | `entities`       | Step2                | `Vec<EntityMention>`|
/// | `entity_ids`     | Step3（stub 留空）   | `Vec<String>`       |
/// | `candidates`     | Step4（stub 留空）   | `Vec<String>`       |
/// | `hits`           | Step5-7（stub 留空） | `Vec<SearchHit>`    |
/// | `thought_process`| 全 step 累积         | `Vec<String>`       |
///
/// ## 用法
/// ```ignore
/// use sparkfox_knowledge::search::multi_step::{MultiState, step1_vectorize, step2_extract_entities};
///
/// let state = MultiState::new("张三去了北京");
/// let state = step1_vectorize(state);
/// let state = step2_extract_entities(state);
/// println!("抽取到 {} 个实体", state.entities.len());
/// ```
#[derive(Debug, Clone)]
pub struct MultiState {
    /// 原始 query 字符串（用户输入）
    pub query: String,
    /// Step1 输出：query 向量化结果（mock 384 维；11.2.x 接入 bge-small-zh）
    pub query_vec: Vec<f32>,
    /// Step2 输出：从 query 抽取的实体（PERSON / ORGANIZATION / LOCATION / TIME / NUMBER）
    pub entities: Vec<EntityMention>,
    /// Step3 输出：实体向量检索得到的 entity_id 列表（stub：留空，待 11.2.x 接入 HnswIndex）
    pub entity_ids: Vec<String>,
    /// Step4 输出：基于 entity_ids 查询 event_entity_relation 得到的候选 event_id（stub：留空）
    pub candidates: Vec<String>,
    /// Step5-7 输出：最终命中的 SearchHit 列表（stub：留空，待 11.2.x 实施三策略 + Rerank）
    pub hits: Vec<SearchHit>,
    /// 8 步流程的执行记录（每步 push 一条人类可读字符串，便于调试 / 日志）
    pub thought_process: Vec<String>,
}

impl MultiState {
    /// 创建初始 `MultiState`（仅 `query` 字段填充，其他字段为空）
    ///
    /// ## 参数
    /// - `query`: 用户输入的查询字符串
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            query_vec: Vec::new(),
            entities: Vec::new(),
            entity_ids: Vec::new(),
            candidates: Vec::new(),
            hits: Vec::new(),
            thought_process: Vec::new(),
        }
    }
}

/// Step1: query 向量化（mock embedding，384 维）
///
/// 当前实现为 **mock**：基于 query 字节哈希到 384 维向量，仅用于占位。
/// 11.2.x 将接入真实 embedding 模型（bge-small-zh，[`sparkfox_embedding::BgeEmbedder`]）。
///
/// ## 输入
/// - `state`: 含 `query` 字段
///
/// ## 输出
/// - 更新 `state.query_vec`（384 维 `Vec<f32>`）
/// - 追加 `thought_process` 一条 Step1 记录
pub fn step1_vectorize(mut state: MultiState) -> MultiState {
    state.query_vec = mock_embed(&state.query);
    state
        .thought_process
        .push("Step1: query 向量化（mock 384 维）".to_string());
    state
}

/// Step2: query 实体抽取（jieba + 正则）
///
/// 调用 [`JiebaNer::extract`] 从 query 抽取 PERSON / ORGANIZATION / LOCATION / TIME / NUMBER
/// 五类实体。每次调用都新建 [`JiebaNer`]（适合测试）；生产环境复用
/// [`step2_extract_entities_with_jieba`] 避免重复加载词典。
///
/// ## 输入
/// - `state`: 含 `query` 字段
///
/// ## 输出
/// - 更新 `state.entities`（`Vec<EntityMention>`）
/// - 追加 `thought_process` 一条 Step2 记录（含实体数量）
pub fn step2_extract_entities(state: MultiState) -> MultiState {
    let jieba = JiebaNer::new();
    step2_extract_entities_with_jieba(state, &jieba)
}

/// Step2 的复用版本：复用调用方持有的 [`JiebaNer`] 实例
///
/// 用于 [`MultiStrategy`](super::MultiStrategy) 内部 search 流程，避免每次 search
/// 都重新加载 jieba 词典（节省 ~50ms / 调用）。
///
/// ## 参数
/// - `state`: 含 `query` 字段
/// - `jieba`: 调用方持有的 [`JiebaNer`] 引用
pub fn step2_extract_entities_with_jieba(mut state: MultiState, jieba: &JiebaNer) -> MultiState {
    state.entities = jieba.extract(&state.query);
    state.thought_process.push(format!(
        "Step2: 实体抽取（{} 个实体）",
        state.entities.len()
    ));
    state
}

/// Step3: 实体向量检索（HnswIndex，返回 Top-K entities）— **stub**
///
/// 11.2.x 实施：基于 Step1 的 `query_vec` 在 HnswIndex 中检索 Top-K entities。
/// 当前留空（`entity_ids` 保持为空）。
pub fn step3_vector_search(mut state: MultiState) -> MultiState {
    state.entity_ids = Vec::new();
    state
        .thought_process
        .push("Step3: 实体向量检索（stub，11.2.x 实施 HnswIndex）".to_string());
    state
}

/// Step4: 事件检索（基于 entity_ids 查询 event_entity_relation）— **stub**
///
/// 11.2.x 实施：基于 Step3 的 `entity_ids` 在 `event_entity_relation` 表中查询
/// 候选 event_ids。当前留空（`candidates` 保持为空）。
pub fn step4_event_search(mut state: MultiState) -> MultiState {
    state.candidates = Vec::new();
    state
        .thought_process
        .push("Step4: 事件检索（stub，11.2.x 实施 event_entity_relation 查询）".to_string());
    state
}

/// Step5: 三策略占位（multi / multi1 / hopllm）— **stub**
///
/// 11.2.x 实施：在此步骤选择 multi / multi1 / hopllm 三种策略之一执行：
/// - **multi**：复用 10.8.2 的 BFS 多跳扩展（已在 [`MultiStrategy`](super::MultiStrategy)::search 中接入）
/// - **multi1**：在 multi 基础上 + LLM 辅助选路
/// - **hopllm**：全程 LLM 推理路径选择
///
/// 当前 free function 留空（`hits` 保持为空）。
/// [`MultiStrategy`](super::MultiStrategy)::search 内部直接调用 10.8.2 BFS 而不经过此 stub。
pub fn step5_strategies_placeholder(mut state: MultiState) -> MultiState {
    state
        .thought_process
        .push("Step5: 三策略占位（stub，11.2.x 实施 multi/multi1/hopllm）".to_string());
    state
}

/// Step6: 候选合并 + 去重 — **stub**
///
/// 11.2.x 实施：合并 Step5 各策略产生的候选 events，按 event_id 去重。
/// 当前留空（`hits` 保持不变）。
pub fn step6_merge_dedupe(mut state: MultiState) -> MultiState {
    state
        .thought_process
        .push("Step6: 候选合并 + 去重（stub，11.2.x 实施）".to_string());
    state
}

/// Step7: Rerank 重排 — **stub**
///
/// 11.2.x 实施：对合并后的候选 events 调用 rerank 模型（如 bge-reranker）重排，
/// 按重排序 score 降序取 top_k。当前留空（`hits` 保持不变）。
pub fn step7_rerank(mut state: MultiState) -> MultiState {
    state
        .thought_process
        .push("Step7: Rerank 重排（stub，11.2.x 实施 bge-reranker）".to_string());
    state
}

/// Step8: 返回 [`SearchResult`]
///
/// 将 [`MultiState`] 的 `hits` 字段包装为 [`SearchResult`] 返回。
/// `thought_process` 8 步记录保留在 [`MultiState`] 中（调用方可记录日志 / 暴露给前端）。
///
/// ## 参数
/// - `state`: 8 步流程执行完毕的 [`MultiState`]
///
/// ## 返回
/// [`SearchResult`]（`strategy_name="multi"`，`latency_ms=0` 由调用方覆写）
pub fn step8_build_result(state: MultiState) -> SearchResult {
    SearchResult {
        hits: state.hits,
        latency_ms: 0,
        strategy_name: "multi".to_string(),
    }
}

/// Mock embedding：将 query 哈希到 384 维 `Vec<f32>`
///
/// 简单确定性哈希：对每个维度 `i`，输出 `(query_bytes[i % len] as f32) / 255.0`
/// 乘以位置权重 `i / EMBED_DIM`，使向量值落在 `[0, 1)` 区间。
///
/// ## 为何不用零向量
/// 零向量会导致向量检索失效（无法区分不同 query）；哈希向量至少有区分度，
/// 便于 11.2.x 接入真实 embedding 前的单元测试。
fn mock_embed(query: &str) -> Vec<f32> {
    let bytes = query.as_bytes();
    let dim = EMBED_DIM;
    let mut vec = Vec::with_capacity(dim);
    if bytes.is_empty() {
        vec.resize(dim, 0.0);
        return vec;
    }
    for i in 0..dim {
        let b = bytes[i % bytes.len()] as f32;
        let position_weight = i as f32 / dim as f32;
        vec.push(b / 255.0 * position_weight);
    }
    vec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_embed_returns_384_dim() {
        let v = mock_embed("test");
        assert_eq!(v.len(), EMBED_DIM);
    }

    #[test]
    fn mock_embed_empty_query_returns_zeros() {
        let v = mock_embed("");
        assert_eq!(v.len(), EMBED_DIM);
        assert!(v.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn mock_embed_is_deterministic() {
        let v1 = mock_embed("张三");
        let v2 = mock_embed("张三");
        assert_eq!(v1, v2);
    }

    #[test]
    fn multi_state_new_initializes_empty_fields() {
        let s = MultiState::new("hello");
        assert_eq!(s.query, "hello");
        assert!(s.query_vec.is_empty());
        assert!(s.entities.is_empty());
        assert!(s.entity_ids.is_empty());
        assert!(s.candidates.is_empty());
        assert!(s.hits.is_empty());
        assert!(s.thought_process.is_empty());
    }
}
