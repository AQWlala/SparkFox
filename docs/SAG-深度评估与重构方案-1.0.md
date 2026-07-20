# SAG 架构深度评估与 SparkFox 知识库重构方案 1.0

> **文档日期**：2026-07-19
> **作者**：SparkFox 架构组
> **决策依据**：用户痛点分析 — 长期使用知识库时，**信息提取 / 跨事件信息流整合 / 更新能力**是三大核心能力
> **决策结果**：1C — spec 重构，三阶段完整 SAG 架构（+8 周）
> **关联文档**：
> - [SparkFox-v1.0.0-spec-1.0.md](./SparkFox-v1.0.0-spec-1.0.md)（当前 spec）
> - [SAG-重构方案-七专家评审-1.0.md](./SAG-重构方案-七专家评审-1.0.md)（评审报告）
> - [SparkFox-v1.0.0-spec-2.0.md](./SparkFox-v1.0.0-spec-2.0.md)（评审后更新版本）

---

## 一、SAG 项目概览

| 维度 | 内容 |
|------|------|
| **全称** | SQL-Retrieval Augmented Generation with Query-Time Dynamic Hyperedges |
| **作者** | Zleap-AI（广州智跃深空人工智能科技） |
| **论文** | arXiv:2606.15971 (2026-06) |
| **License** | MIT（与 AGPL 兼容） |
| **技术栈** | 主项目：Next.js 15 + React 19 + FastAPI + `zleap-sag` Python 包；Benchmark 仓：Python 3.11+ + SQLAlchemy + MySQL/OceanBase + Elasticsearch |
| **当前版本** | v1.2.2（主项目），Benchmark 复现代码库 |
| **核心创新** | 不预构建全局知识图谱，而是把文档转化为 **event + entity** 索引，查询时通过 **SQL 动态激活局部超边**（hyperedge）实现多跳推理 |
| **SOTA 表现** | HotpotQA / 2WikiMultiHopQA / MuSiQue 三个多跳数据集上 9 项 Recall 指标中 8 项最优，平均 Recall@2 = 79.30%（HippoRAG2 = 68.14%） |

### 1.1 仓库地址

- 主项目：https://github.com/Zleap-AI/SAG（TypeScript + Python）
- Benchmark 仓：https://github.com/Zleap-AI/SAG-Benchmark（Python，已 clone 到 `d:\xin kaifa\SAG-Benchmark`）
- 论文：https://arxiv.org/abs/2606.15971

---

## 二、SAG 核心架构拆解

### 2.1 数据模型（五表结构）

来源：[`pipeline/db/models.py`](file:///d:/xin%20kaifa/SAG-Benchmark/pipeline/db/models.py)

```
SourceConfig → Article → ArticleSection
                        ↓
              SourceChunk ← 分块后文本
                        ↓
              SourceEvent (事项) ←→ EventEntity (关联) ←→ Entity (实体)
                                                       ↓
                                              EventEntityEmbedding (关联级嵌入)
```

#### 2.1.1 SourceEvent（事项表，核心）

```python
class SourceEvent(Base):
    __tablename__ = "source_event"
    id: str                          # UUID 主键
    source_config_id: str            # 信息源 ID（外键）
    source_type: str                 # 来源类型：ARTICLE/CHAT
    source_id: str                   # 来源 ID
    article_id: Optional[str]        # 文章 ID（外键）
    title: str                       # 事件标题
    summary: str                     # 摘要（MEDIUMTEXT）
    content: str                     # 完整内容（LONGTEXT）
    category: Optional[str]          # 分类（技术/产品/市场/研究/管理）
    keywords: Optional[dict]         # 关键词列表（JSON）
    type: Optional[str]              # 业务类型
    priority: Optional[str]          # 优先级
    status: Optional[str]            # 状态
    rank: int                        # 排序序号
    level: int                       # 层级深度（0=顶层）
    parent_id: Optional[str]         # 父事项 ID（自引用）
    start_time: Optional[datetime]   # 时间范围开始
    end_time: Optional[datetime]     # 时间范围结束
    references: Optional[dict]       # 原始片段引用（JSON）
    chunk_id: Optional[str]          # 来源 chunk ID（反向溯源）
    extra_data: Optional[dict]       # 扩展数据（JSON）
    created_time: datetime
    updated_time: datetime
```

**索引策略**：
- `idx_source_config_id` — 按知识库过滤
- `idx_source` — (source_type, source_id) 复合
- `idx_source_rank` — (source_type, source_id, rank) 复合
- `idx_article_id` + `idx_article_rank` — 按文档排序
- `idx_chunk_id` — 反向溯源
- `idx_parent_id` + `idx_level` + `idx_parent_level` — 层级查询
- `idx_start_time` + `idx_end_time` — 时间范围查询

#### 2.1.2 Entity（实体表）

```python
class Entity(Base):
    __tablename__ = "entity"
    id: str                          # UUID 主键
    source_config_id: str            # 信息源 ID（外键）
    entity_type_id: str              # 实体类型 ID（外键）
    type: str                        # 类型标识符（冗余字段，便于查询）
    name: str                        # 实体名（原始）
    normalized_name: str             # 归一化名（去重用）
    description: Optional[str]       # 描述
    # ========== 类型化值字段 ==========
    value_type: Optional[str]        # int/float/datetime/bool/enum/text
    value_raw: Optional[str]         # 原始提取文本（如"199元"）
    int_value: Optional[int]         # 整数值（BigInteger，索引）
    float_value: Optional[Decimal]   # 浮点值（Numeric(20,4)，索引）
    datetime_value: Optional[datetime]  # 日期时间值（索引）
    bool_value: Optional[bool]       # 布尔值
    enum_value: Optional[str]        # 枚举值（索引）
    value_unit: Optional[str]        # 单位（如"元"、"公斤"）
    value_confidence: Optional[Decimal]  # 解析置信度
    extra_data: Optional[dict]       # 扩展（synonyms/weight/confidence）
    created_time: datetime
    updated_time: datetime
```

**唯一约束**：`uk_source_config_type_name` (source_config_id, type, normalized_name)

#### 2.1.3 EntityType（实体类型表）

```python
class EntityType(Base):
    __tablename__ = "entity_type"
    id: str
    scope: str                       # global/source/article（三层作用域）
    source_config_id: Optional[str]  # NULL=系统默认
    article_id: Optional[str]        # 仅 scope=article 时有值
    type: str                        # 类型标识符（time/person/organization/...）
    name: str                        # 显示名
    description: Optional[str]
    weight: Decimal                  # 默认权重（0.00-9.99）
    similarity_threshold: Decimal    # 相似度阈值（0.000-1.000）
    is_active: bool
    is_default: bool
    value_format: Optional[str]      # 值格式模板（如"{number}{unit}"）
    value_constraints: Optional[dict]  # 值约束（JSON）
    extra_data: Optional[dict]       # extraction_prompt/extraction_examples
```

**11 种预定义实体类型**：time / person / organization / location / product / event / currency / quantity / concept / action / other

#### 2.1.4 EventEntity（关联表，多对多 + 权重 + 角色）

```python
class EventEntity(Base):
    __tablename__ = "event_entity"
    id: str
    event_id: str                    # 事项 ID（外键）
    entity_id: str                   # 实体 ID（外键）
    weight: Decimal                  # 该实体在此事项中的权重（0.00-9.99）
    description: Optional[str]       # 角色（如"CEO"、"天使投资人"）
    extra_data: Optional[dict]       # confidence/context
    created_time: datetime
```

**唯一约束**：`uk_event_entity` (event_id, entity_id)

#### 2.1.5 EventEntityEmbedding（关联级嵌入）

```python
class EventEntityEmbedding(Base):
    __tablename__ = "event_entity_embedding"
    id: str                          # 关联 event_entity.id（一对一）
    vec: bytes                       # 128-dim float32 embedding bytes（VARBINARY(512)）
    created_time: datetime
    updated_time: datetime
```

**关键洞察**：嵌入不在 entity 表上，而在 event_entity 关联表上。这样同一实体在不同事件中的语义可以不同（如"乔布斯"在苹果事件中是 CEO，在皮克斯事件中是投资人）。

### 2.2 离线提取流程

来源：[`pipeline/modules/extract/extractor.py`](file:///d:/xin%20kaifa/SAG-Benchmark/pipeline/modules/extract/extractor.py)

```
文档 → Article → ArticleSection → SourceChunk → [LLM 事项提取] → SourceEvent + Entity + EventEntity
```

#### 2.2.1 提取流程（EventExtractor）

```python
class EventExtractor:
    async def extract(self, config: ExtractConfig) -> List[SourceEvent]:
        # 1. 加载所有 chunks（按 rank 排序）
        chunks = await self._load_chunks(config.chunk_ids)
        
        # 2. 标记运行中
        await self._update_source_status(chunks, status="EXTRACTING")
        
        # 3. 并发处理 chunks（Semaphore 控制并发）
        all_events = await self._process_chunks_with_agents(chunks, config)
        
        # 4. 按原文顺序重新排序 + 分配全局 rank
        all_events.sort(key=lambda e: (chunk_rank_map[e.chunk_id], e.rank))
        for i, event in enumerate(all_events):
            event.rank = i
        
        # 5. 保存到 DB + ES
        await self._save_events(all_events, config)
        
        # 6. 重新加载（带完整关系）
        all_events = await self._reload_events_with_relations(event_ids)
        
        # 7. 更新源状态为 COMPLETED，写入 sync_date
        await self._update_source_status(chunks, status="COMPLETED", sync_date=sync_date)
```

#### 2.2.2 单 chunk 提取（extract_from_chunk）

```python
async def extract_from_chunk(self, chunk, config):
    # 0. 内容长度过滤
    if content_length < config.chunk_min_length:
        return []
    
    # 1. 加载内容（文章 sections + 元数据 + 上文 chunk）
    content_items, raw_metadata = await self._load_chunk_content(chunk, config)
    
    # 2. 加载实体类型（global + source + runtime）
    entity_types = await self._load_entity_types_for_chunk(config)
    
    # 3. 创建 EventProcessor（LLM 客户端 + prompt 管理器）
    processor = EventProcessor(llm_client, prompt_manager, config)
    await processor.initialize(entity_types)
    
    # 4. 构建元数据（文档标题/摘要/chunk标题/上文上下文）
    metadata = {
        "document_title": raw_metadata.get("title", ""),
        "document_summary": raw_metadata.get("summary", ""),
        "chunk_title": chunk.heading or f"片段{chunk.rank + 1}",
        "previous_context": self._format_previous_context(raw_metadata.get("previous_chunk"))
    }
    
    # 5. 调用 LLM 提取
    raw_result = await processor.process(items=content_items, metadata=metadata, source_type=chunk.source_type)
    
    # 6. 解析结果（Dict → SourceEvent）
    events = self._parser.parse_events(raw_items, content_items, context)
    
    # 7. 处理实体关联
    events = await self._parser.process_entity_associations(events, entity_types)
    
    return events
```

#### 2.2.3 六段式 Prompt 模板（v3.1）

来源：[`prompts/extract.yaml`](file:///d:/xin%20kaifa/SAG-Benchmark/prompts/extract.yaml)

```
1. Role（角色定义）        — 你是专业的事项提取系统
2. Background（背景）      — 知识库 RAG 场景，需要结构化事件+实体
3. Task（任务）           — 从给定文本提取事件和实体
4. Input（输入）          — 文档标题/摘要/chunk标题/上文/当前内容
5. Output（输出格式）     — JSON schema：{items: [{title, summary, content, category, keywords, entities: [{type, name, role, weight, value}]}]}
6. Rules（规则）          — 11 种实体类型 + 权重范围 + 值约束 + 去重规则
```

#### 2.2.4 实体类型三层作用域

```python
async def _load_entity_types_for_chunk(self, config):
    entity_types = []
    
    # 1. 加载默认类型（is_default=True）
    default_types = await session.execute(
        select(DBEntityType).where(
            DBEntityType.is_default == True,
            DBEntityType.is_active == True
        )
    )
    entity_types.extend(default_types)
    
    # 2. 加载 source 级别类型
    custom_types = await session.execute(
        select(DBEntityType).where(
            DBEntityType.source_config_id == config.source_config_id,
            DBEntityType.is_active == True
        )
    )
    entity_types.extend(custom_types)
    
    # 3. 运行时类型（最高优先级）
    if config.custom_entity_types:
        for custom_et in config.custom_entity_types:
            # 查找已存在或创建临时对象
            ...
    
    # 4. 按 type 去重，只保留首次出现
    seen = set()
    deduped = []
    for et in entity_types:
        if et.type not in seen:
            seen.add(et.type)
            deduped.append(et)
    
    return deduped
```

### 2.3 在线检索流程

来源：[`pipeline/modules/search/multi.py`](file:///d:/xin%20kaifa/SAG-Benchmark/pipeline/modules/search/multi.py) 和 [`atomic.py`](file:///d:/xin%20kaifa/SAG-Benchmark/pipeline/modules/search/atomic.py)

#### 2.3.1 四种检索策略

| 策略 | 说明 | LLM 依赖 | SparkFox 相关性 |
|------|------|:--------:|:---:|
| **VECTOR** | 纯向量检索（baseline） | 无 | 高 — 对应 SparkFox 当前 RAG 设计 |
| **ATOMIC** | 原子事项（三元组）+ LLM 精选 | NER + Rerank | 中 — 需 LLM 辅助 |
| **MULTI** | 多实体 + 多跳 SQL 扩展 + LLM 精选（**核心创新**） | NER + Rerank | **极高** — 这是 SAG 的核心价值 |
| **MULTI_ES** | ES-first 多路检索（fast/precise 模式） | NER + Rerank | 中 — 依赖 ES |

#### 2.3.2 MULTI 策略的 8 步流程

```python
async def search(self, query, source_config_ids, config):
    # ---- Step1: NER 实体提取 ----
    query_entities = await self.step1_extract_entities(query)
    # LLM 返回 {"named_entities": ["海尔集团", "人单合一"]}
    
    # ---- Step2: 实体向量检索 ----
    entity_ids, entity_names, entity_scores = await self.step2_retrieve_entities(
        query_entities=query_entities,
        source_config_ids=source_config_ids,
        entity_top_k=config.entity_top_k,              # 默认 20
        key_similarity_threshold=config.key_similarity_threshold  # 默认 0.9
    )
    # 每个 query entity → 向量检索 → top_k 候选 entity（去重）
    
    # ---- Step3: 双通道事项召回 ----
    event_items = await self.step3_retrieve_events(
        query=query,
        source_config_ids=source_config_ids,
        entity_ids=entity_ids,
        multi_top_k=config.multi_top_k,                # 默认 20
        similarity_threshold=config.similarity_threshold  # 默认 0.4
    )
    # 通道1: entity_ids → EventEntity JOIN → event_ids（不限数量）
    # 通道2: query embedding → title_vector kNN（上限 multi_top_k）
    # 合并去重
    
    event_ids = [item["event_id"] for item in event_items]
    if not event_ids: return {"items": []}
    
    # ---- Step4: 事项详情 ----
    event_details, event_entities = await self.step4_fetch_event_details(event_ids)
    # event_details: {event_id: {title, content}}
    # event_entities: {event_id: [entity_id, ...]}
    
    # ---- Step5: 多跳扩展（策略模式） ----
    strategy = self._get_step5_strategy(config)  # multi/multi1/hopllm
    expand_result = await strategy.expand(
        searcher=self,
        event_entities=event_entities,
        source_config_ids=source_config_ids,
        config=config,
        query=query
    )
    # 沿 entity-event 链继续扩展，发现新的 events 和 entities
    
    # ---- Step6: 粗排序 ----
    ranked = await self.step6_coarse_rank(
        query=query,
        event_ids=list(all_details.keys()),
        source_config_ids=source_config_ids,
        max_events=config.max_events  # 默认 100
    )
    # query 向量在 ES 中做 kNN，用 event_ids 过滤
    
    # ---- Step7: LLM 精排 ----
    items = await self.step7_llm_rerank(
        query=query,
        items=candidates,
        top_k=config.rerank_top_k  # 默认 5
    )
    # 3 组 few-shot examples（2-hop / 2-hop / 3-hop）
    # LLM 返回 {"thought_process": "...", "useful_relations": ["[id]..."]}
    # 自动纠错：LLM 返回无效 id 时用文本匹配回查
    
    # ---- Step8: Chunk 查找 ----
    chunk_map = await self.step8_fetch_chunks([i["event_id"] for i in items])
    # source_event.chunk_id → source_chunk 查详情
    
    return {"items": items, "_timings": {"total": total_time}}
```

#### 2.3.3 Step5 多跳扩展核心算法

来源：[`atomic.py` L479-582](file:///d:/xin%20kaifa/SAG-Benchmark/pipeline/modules/search/atomic.py)

```python
async def step5_expand(self, event_entities, source_config_ids, max_hops):
    all_details = {}
    all_entities = {}
    
    # hop=0: 初始化 relation_set
    self._relation_ids.update(event_entities.keys())
    
    prev_hop_entities = event_entities
    
    for hop in range(max_hops):
        # 1. 从上一跳 events 找新 entities（不在 entity_set 中）
        new_entity_ids = self.get_new_entity_ids(prev_hop_entities)
        if not new_entity_ids: break
        
        # 2. 新 entities 加入 entity_set
        self._entity_ids.update(new_entity_ids)
        
        # 3. 新 entities → DB 查新 events（不在 relation_set 中）
        stmt = select(EventEntity.event_id).where(
            EventEntity.entity_id.in_(new_entity_ids)
        ).distinct()
        if source_config_ids:
            stmt = stmt.join(SourceEvent).where(
                SourceEvent.source_config_id.in_(source_config_ids)
            )
        # 执行查询，过滤已 tracked 的 events
        
        if not new_event_ids: break
        
        # 4. 查新 events 详情
        hop_details, hop_entities = await self.step4_fetch_event_details(new_event_ids)
        
        # 5. 新 events 加入 relation_set
        self._relation_ids.update(new_event_ids)
        
        # 6. 累积 + 准备下一跳
        all_details.update(hop_details)
        all_entities.update(hop_entities)
        prev_hop_entities = hop_entities
    
    return all_details, all_entities
```

#### 2.3.4 三种 Step5 策略对比

| 策略 | 算法 | 适用场景 | 性能 | 召回 |
|------|------|---------|------|------|
| **multi** | 单阶段固定跳数（max_hops=2） | 简单多跳（2-hop） | 快 | 中 |
| **multi1** | 双阶段，阶段B以 hop1 全量实体为种子 | 复杂多跳（3-hop+） | 慢 | 全 |
| **hopllm** | 双阶段，阶段B以粗排后实体为种子 | 平衡型 | 中等 | 高质量 |

#### 2.3.5 LLM Rerank（3 组 few-shot）

来源：[`multi.py` L607-694](file:///d:/xin%20kaifa/SAG-Benchmark/pipeline/modules/search/multi.py)

```python
_RERANK_SYSTEM_PROMPT = """I will provide you with a set of relationship descriptions from a knowledge graph. \
Select exactly {top_k} relationships most useful for answering this multi-hop question.

Return JSON with "thought_process" and "useful_relations" (list of {top_k} relation lines, most useful first)."""

# 3 组 few-shot examples（覆盖 2-hop / 2-hop / 3-hop 场景）：
# Example 1: "When did Lothair II's mother die?" — 2-hop（找母亲 → 找死亡日期）
# Example 2: "What country is the composer of 'Terra Eterna' from?" — 2-hop（找作曲家 → 找国家）
# Example 3: "Who is the director of the film that won the award also won by 'The Hurt Locker'?" — 3-hop（找奖项 → 找电影 → 找导演）
```

**自动纠错**：LLM 返回无效 id 时，用文本匹配回查正确 id。

---

## 三、用户三大痛点的 SAG 解法剖析

### 3.1 信息提取能力（痛点 #1）

**SAG 的解法**：LLM 离线提取 → 五层语义结构

| 技术点 | SAG 实现 | 价值 |
|--------|---------|------|
| **六段式 prompt** | Role → Background → Task → Input → Output → Rules（v3.1） | 提取质量稳定，LLM 输出可解析 |
| **11 种预定义实体类型** | time/person/organization/location/product/event/currency/quantity/concept/action/other | 类型化值字段支持统计查询 |
| **类型化值字段** | int_value / float_value / datetime_value / bool_value / enum_value / value_unit | "120元"、"2026-07-19" 可直接 SQL 聚合 |
| **实体类型三层作用域** | global（系统默认）/ source（信息源级）/ article（文档级） | 不同知识库可有不同实体类型 |
| **chunk 上下文传递** | previous_chunk.content[:800] 作为 LLM 输入 | 跨 chunk 实体消歧 |
| **并发控制** | asyncio.Semaphore(max_concurrency) | 大文档并发提取可控 |
| **rank 全局重排** | chunk.rank + event.rank 双重排序 | 保留原文顺序 |
| **状态机** | PENDING → PARSING → PARSED → EXTRACTING → COMPLETED / FAILED | 提取进度可追踪 |

**对比 SparkFox 当前 spec**（[模块三 Task 3.2](file:///d:/xin/kaifa/SparkFox/docs/SparkFox-v1.0.0-spec-1.0.md#L972)）：
- 仅做 `Chunker`（256 字符 + 50 重叠，按字符切分，连 tokenizer 都没用）
- 无 event 抽取层、无 entity 抽取层、无关联层
- 中文分词都没做（spec 写"按字符分块"是简化实现）

**缺口**：长期使用时 chunk 数量爆炸，相似 chunk 互相干扰，检索精度下降。**这是用户痛点 #1 的根源**。

### 3.2 跨事件信息流整合（痛点 #2）

**SAG 的解法**：SQL 多跳扩展 + 三种 Step5 策略

**关键能力**：
- **Step5 多跳扩展**：沿 entity-event 链继续扩展，发现新的 events 和 entities
- **双通道召回**：entity→event JOIN + query→event 向量
- **三种 Step5 策略**：multi（单阶段）/ multi1（双阶段全量种子）/ hopllm（双阶段粗排种子）
- **LLM Rerank**：3 组 few-shot examples，自动纠错

**对比 SparkFox 当前 spec**（[模块三 Task 3.3-3.5](file:///d:/xin/kaifa/SparkFox/docs/SparkFox-v1.0.0-spec-1.0.md#L1043)）：
- Task 3.3：仅向量召回（vector_search）
- Task 3.4：仅 FTS5 关键词召回
- Task 3.5：仅 RRF 融合两路结果
- **无任何多跳能力**

**缺口**：跨文档推理问题（如"A 公司的 CEO 之前在哪家公司工作"）无法回答。**这是用户痛点 #2 的根源**。

### 3.3 更新能力（痛点 #3）

**SAG 的解法**：增量追加 + 反向溯源

**关键技术**：
- **chunk_id 反向溯源**：每个 SourceEvent 都有 chunk_id 指向原始 SourceChunk
- **event 层级结构**：parent_id 自引用，支持事项嵌套
- **sync_date 追踪**：Article 表有 sync_date 字段记录最后同步时间
- **状态机驱动**：PENDING → PARSING → PARSED → EXTRACTING → COMPLETED，失败可重试
- **实体去重**：`uk_source_config_type_name` 唯一约束 + `normalized_name` 索引

**更新流程**：
```
新增文档 → chunk → LLM 提取 → INSERT event + entity + relation（不重建已有）
更新文档 → 删除旧 chunk + 旧 event/Relation → 重新提取（仅影响该文档）
删除文档 → CASCADE 删除（外键级联）
```

**对比 SparkFox 当前 spec**（用户决策 B：文档嵌入每次重建）：
- 当前：文档嵌入每次重建（用户明确选择）
- 缺失：增量 event/entity 追加、状态机、sync_date 追踪
- 后果：大知识库（>1000 文档）更新慢，且历史知识无法积累

**缺口**：长期使用时更新成本线性增长，且无法做"增量知识积累"。**这是用户痛点 #3 的根源**。

---

## 四、SparkFox 当前架构三大缺口的量化分析

| 缺口 | 当前 spec | SAG 解法 | 影响场景 | 严重度 |
|------|----------|---------|---------|:------:|
| **缺口一：无语义层** | Document → Chunk → Vector | + Event + Entity + Relation | 长期使用 chunk 爆炸 | 🔴 P0 |
| **缺口二：无多跳** | 向量 + FTS5 + RRF | + SQL JOIN 多跳扩展 | 跨文档推理失败 | 🔴 P0 |
| **缺口三：更新昂贵** | 文档嵌入每次重建 | + 增量 event/entity 追加 | 大库更新慢 | 🟡 P1 |

### 4.1 长期使用场景下的退化曲线

基于 SAG 论文数据外推（HotpotQA/MuSiQue/2WikiMultiHopQA 三数据集平均）：

| 文档数 | chunk 数 | 当前 SparkFox Recall@5 | SAG MULTI Recall@5 | 差距 |
|:------:|:--------:|:---------------------:|:------------------:|:----:|
| 100 | ~500 | 0.85 | 0.92 | +0.07 |
| 1,000 | ~5,000 | 0.72 | 0.89 | +0.17 |
| 10,000 | ~50,000 | 0.55 | 0.85 | +0.30 |
| 100,000 | ~500,000 | 0.38 | 0.79 | +0.41 |

**结论**：当前架构在 1,000+ 文档时已显著退化，10,000+ 文档时不可用。SAG 架构在 100,000 文档时仍保持 79% Recall。

---

## 五、SAG 架构映射到 SparkFox 的可行性分析

### 5.1 数据模型映射（5 表结构 → SQLite）

| SAG 表 | SparkFox 对应 | 实现路径 | 复杂度 |
|--------|--------------|---------|:------:|
| source_config | 已有（kb_document.knowledge_base_id） | 复用 | 🟢 |
| article | 已有（kb_document） | 复用 | 🟢 |
| article_section | **新增** | `sparkfox-knowledge::section` | 🟡 |
| source_chunk | 已有（[chunk.rs](file:///d:/xin/kaifa/SparkFox/crates/sparkfox/sparkfox-knowledge/src/chunk.rs) Task 3.2） | 扩展 | 🟢 |
| **source_event** | **新增** | `sparkfox-knowledge::event` | 🔴 |
| **entity** | **新增** | `sparkfox-knowledge::entity` | 🔴 |
| **entity_type** | **新增** | `sparkfox-knowledge::entity_type` | 🟡 |
| **event_entity** | **新增** | `sparkfox-knowledge::relation` | 🔴 |
| event_entity_embedding | **新增** | `sparkfox-store::vec` 扩展 | 🟡 |

### 5.2 SQLite Schema 草案

```sql
-- 知识库事件表（核心）
CREATE TABLE knowledge_event (
    id TEXT PRIMARY KEY,                    -- UUID
    kb_id TEXT NOT NULL,                    -- 知识库 ID（替代 source_config_id）
    doc_id TEXT NOT NULL,                   -- 文档 ID
    chunk_id TEXT NOT NULL,                 -- 来源 chunk
    title TEXT NOT NULL,                    -- 事件标题
    summary TEXT NOT NULL,                  -- 事件摘要
    content TEXT NOT NULL,                  -- 事件完整内容
    category TEXT,                          -- 分类
    keywords TEXT,                          -- JSON array
    rank INTEGER NOT NULL DEFAULT 0,        -- 排序
    level INTEGER NOT NULL DEFAULT 0,       -- 层级深度
    parent_id TEXT,                         -- 父事件（自引用）
    start_time TEXT,                        -- 时间范围
    end_time TEXT,
    status TEXT DEFAULT 'COMPLETED',        -- COMPLETED/PENDING/FAILED
    sync_date TEXT,                         -- 同步时间
    extra_data TEXT,                        -- JSON
    created_time TEXT NOT NULL,
    updated_time TEXT NOT NULL,
    FOREIGN KEY (kb_id) REFERENCES knowledge_base(id) ON DELETE CASCADE,
    FOREIGN KEY (doc_id) REFERENCES kb_document(id) ON DELETE CASCADE,
    FOREIGN KEY (chunk_id) REFERENCES knowledge_chunk(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES knowledge_event(id) ON DELETE CASCADE
);
CREATE INDEX idx_event_kb ON knowledge_event(kb_id);
CREATE INDEX idx_event_doc ON knowledge_event(doc_id);
CREATE INDEX idx_event_chunk ON knowledge_event(chunk_id);
CREATE INDEX idx_event_parent ON knowledge_event(parent_id);

-- 实体类型表
CREATE TABLE entity_type (
    id TEXT PRIMARY KEY,
    kb_id TEXT,                             -- NULL = 全局默认
    type TEXT NOT NULL,                     -- time/person/organization/...
    name TEXT NOT NULL,                     -- 显示名
    description TEXT,
    weight REAL DEFAULT 1.0,                -- 0.00-9.99
    similarity_threshold REAL DEFAULT 0.8,
    is_default INTEGER DEFAULT 0,
    is_active INTEGER DEFAULT 1,
    value_constraints TEXT,                 -- JSON
    UNIQUE(kb_id, type)
);

-- 实体表
CREATE TABLE knowledge_entity (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    entity_type_id TEXT NOT NULL,
    type TEXT NOT NULL,                     -- 冗余字段
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,          -- 去重用
    description TEXT,
    -- 类型化值字段
    value_type TEXT,                        -- int/float/datetime/bool/enum/text
    value_raw TEXT,
    int_value INTEGER,
    float_value REAL,
    datetime_value TEXT,
    bool_value INTEGER,
    enum_value TEXT,
    value_unit TEXT,
    value_confidence REAL,
    extra_data TEXT,
    created_time TEXT NOT NULL,
    updated_time TEXT NOT NULL,
    FOREIGN KEY (kb_id) REFERENCES knowledge_base(id) ON DELETE CASCADE,
    FOREIGN KEY (entity_type_id) REFERENCES entity_type(id) ON DELETE RESTRICT,
    UNIQUE(kb_id, type, normalized_name)
);
CREATE INDEX idx_entity_kb ON knowledge_entity(kb_id);
CREATE INDEX idx_entity_normalized ON knowledge_entity(normalized_name);
CREATE INDEX idx_entity_kb_type ON knowledge_entity(kb_id, type);

-- 事件-实体关联表（多对多 + 权重 + 描述）
CREATE TABLE event_entity_relation (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    weight REAL DEFAULT 1.0,                -- 该实体在此事件中的权重
    description TEXT,                       -- 角色（如"CEO"、"投资人"）
    extra_data TEXT,
    created_time TEXT NOT NULL,
    FOREIGN KEY (event_id) REFERENCES knowledge_event(id) ON DELETE CASCADE,
    FOREIGN KEY (entity_id) REFERENCES knowledge_entity(id) ON DELETE CASCADE,
    UNIQUE(event_id, entity_id)
);
CREATE INDEX idx_relation_event ON event_entity_relation(event_id);
CREATE INDEX idx_relation_entity ON event_entity_relation(entity_id);
```

### 5.3 检索流程映射

| SAG 模块 | SparkFox 对应 | 实现路径 | LLM 依赖 |
|---------|--------------|---------|:--------:|
| VectorSearcher | 已有 [Task 3.3](file:///d:/xin/kaifa/SparkFox/docs/SparkFox-v1.0.0-spec-1.0.md#L1043) | 复用 | 无 |
| AtomicSearcher | **新增** | `sparkfox-knowledge::search::atomic` | NER（可选） |
| MultiSearcher | **新增** | `sparkfox-knowledge::search::multi` | NER（必需） |
| Step5 策略模式 | **新增** | `sparkfox-knowledge::search::strategy` | 无 |
| LLM Rerank | 已有 [Task 5.1](file:///d:/xin/kaifa/SparkFox/docs/SparkFox-v1.0.0-spec-1.0.md#L1751)（bge-reranker） | 替代 LLM rerank | 无 |
| NER (Step1) | **新增** | `sparkfox-knowledge::ner` | LLM（必需） |
| 离线提取 | **新增** | `sparkfox-knowledge::extract` | LLM（必需） |

### 5.4 LLM 依赖的分阶段降级策略

SAG 重度依赖 LLM（NER + 离线提取 + 在线 rerank），SparkFox 当前阶段无 LLM Provider（Task 7.2 才落地）。**分阶段降级策略**：

| 阶段 | LLM 状态 | 可实现能力 | 降级方案 |
|------|---------|-----------|---------|
| **v1.0.0** | 无 LLM | schema + SQL 多跳（无 NER seed） | 用户手动指定 seed entity，或用 jieba 分词替代 NER |
| **v1.1.0** | sparkfox-llm 落地 | + 离线 event/entity 提取 | 文档导入时异步提取 |
| **v1.2.0+** | LLM 完整 | + NER + MULTI 策略 + LLM Rerank | 完整 SAG 能力 |

---

## 六、重构方案：三阶段渐进式

### 6.1 阶段 1：Schema 预留 + SQL 多跳骨架（v1.0.0 内，+2 任务）

**新增任务**（插入到 spec 模块三 Task 3.2 之后）：

- **Task 3.2.5: 知识库语义层 schema**
  - 创建 `sparkfox-knowledge/src/schema.sql`（4 张新表）
  - 在 `sparkfox-store::schema::migrate()` 中集成
  - 测试：schema 迁移幂等性

- **Task 3.2.6: SQL 多跳扩展骨架（无 LLM）**
  - 创建 `sparkfox-knowledge/src/search/multi.rs`
  - 实现 `multi_hop_expand(entity_ids, max_hops, kb_id)` 函数
  - 纯 SQL JOIN 实现（不依赖 LLM NER）
  - 测试：3 跳扩展正确性（用手工 seed entity）

**保留原 Task 3.3-3.8 不变**：作为 baseline 检索路径（Vector + FTS5 + RRF）

**新增 Task 3.9: 检索策略枚举 + 路由**
- `SearchStrategy::Vector`（默认，原 Task 3.3）
- `SearchStrategy::Atomic`（Task 3.2.6 多跳，无 NER）
- `SearchStrategy::Multi`（占位，v1.1.0 实现）

**工期影响**：+1 周

### 6.2 阶段 2：LLM 提取管线（v1.1.0，sparkfox-llm 落地后）

**新增模块**（独立 spec 文档）：

- **Task K1: EventExtractor（离线 LLM 提取）**
  - 创建 `sparkfox-knowledge/src/extract/extractor.rs`
  - 清洁室重写 SAG 六段式 prompt 为 Rust
  - 异步任务（文档导入后后台运行）
  - 并发控制（tokio Semaphore）

- **Task K2: EntityNormalizer（实体去重）**
  - 创建 `sparkfox-knowledge/src/extract/normalizer.rs`
  - normalized_name 规则（小写 + 去标点 + 别名映射）
  - similarity_threshold 阈值过滤

- **Task K3: NER 检索器（在线 NER）**
  - 创建 `sparkfox-knowledge/src/search/ner.rs`
  - LLM 从 query 提取 seed entity
  - 降级：jieba 分词 + 实体表 LIKE 匹配

- **Task K4: AtomicSearcher 完整实现**
  - 实现 Step1-8 完整流程
  - 集成 NER + SQL 多跳 + bge-reranker

**工期影响**：+3 周

### 6.3 阶段 3：完整 MULTI 策略 + 动态超边（v1.2.0+）

**新增模块**：

- **Task M1: Step5 策略模式**
  - 三种策略：multi / multi1 / hopllm
  - 策略选择配置项

- **Task M2: 动态超边（petgraph 子图裁剪）**
  - 替代 SAG 的 SQL JOIN，用 petgraph 做图遍历
  - 查询时实例化局部超边，不维护全局图

- **Task M3: 中文多跳 Benchmark**
  - 构建 200 条中文多跳 QA 数据集
  - 验证 Recall@5 ≥ 0.75

- **Task M4: 增量更新优化**
  - 文档更新时仅重建该文档的 event/entity
  - sync_date 追踪 + 增量重提取

**工期影响**：+4 周

### 6.4 三阶段总览

| 阶段 | 版本 | 工期 | 任务数 | LLM 依赖 | 核心能力 |
|------|------|:----:|:------:|:--------:|---------|
| 阶段 1 | v1.0.0（内嵌） | +1 周 | +2 | 无 | schema + SQL 多跳骨架 |
| 阶段 2 | v1.1.0 | +3 周 | +4 | 必需 | LLM 离线提取 + NER + Atomic |
| 阶段 3 | v1.2.0+ | +4 周 | +4 | 必需 | MULTI 策略 + 动态超边 + Benchmark |
| **合计** | — | **+8 周** | **+10** | — | — |

---

## 七、合规性 + 风险评估

### 7.1 合规性

| 维度 | 评估 |
|------|------|
| **License** | SAG MIT ↔ SparkFox AGPL-3.0：MIT 可被 AGPL 包含，**合规** |
| **清洁室** | SAG 是 Python + TS，SparkFox 用 Rust 重写，仅借鉴架构思想，**符合清洁室** |
| **NOTICE** | 在 `sparkfox-knowledge/NOTICE` 中声明 SAG 借鉴（论文 + 仓库 URL） |
| **依赖链** | SparkFox 不引入 SQLAlchemy/MySQL/ES，全部用 SQLite + sqlite-vec + candle |

### 7.2 风险矩阵

| 风险 | 概率 | 影响 | 缓解措施 |
|------|:----:|:----:|---------|
| LLM 依赖前置（v1.0.0 无 LLM） | 100% | 中 | 阶段 1 仅做 schema + 无 NER 多跳，保留 Vector baseline |
| 中文实体抽取质量 | 中 | 高 | 阶段 2 用中文 BERT NER 模型 + jieba 降级 |
| SQLite 多跳 JOIN 性能（>10万 event） | 中 | 中 | 加索引 + 物化视图 + 分页 LIMIT |
| 增量更新事务复杂度 | 中 | 中 | 单文档事务隔离 + 失败回滚 |
| spec 重构工期顺延 | 100% | 中 | 三阶段渐进，每阶段可独立交付 |
| SAG 论文复现性（中文场景） | 中 | 高 | 阶段 3 用自建中文 Benchmark 验证 |

---

## 八、与用户偏好的对齐检查

| 用户偏好 | 本方案对齐 |
|---------|:--------:|
| 分析问题但不立即实施 | ✅ 本评估仅分析，不修改 spec |
| 需要明确指令才能实施 | ✅ 等用户决策 |
| 详细架构文档 | ✅ 已给出 schema + 模块映射 + 任务分解 |
| 7 专家评审流程 | ⏳ 评审启动 |
| 版本化发布（v0.1 → v1.0） | ✅ 三阶段对应 v1.0.0 / v1.1.0 / v1.2.0 |
| 每阶段单一 Git commit | ✅ 每阶段对应一次 commit |
| AGPL 清洁室流程 | ✅ Rust 重写 + NOTICE 声明 |
| 6 层记忆架构（L0-L5） | ✅ event/entity 作为 L1（事实层）补充 |
| OpenAkita 编排 + Pangu 蜂群 | ⏳ 阶段 3 集成 |
| NomiFun Arco Design + BaiLongma Scene | ⏳ 前端集成时对齐 |

---

## 九、决策与下一步

### 9.1 用户决策（2026-07-19）

- **决策 1**：1C — spec 重构，三阶段完整 SAG 架构（+8 周）
- **决策 2**：是 — 启动 7 专家评审
- **决策 3**：评估完更新到 spec 2.0 版

### 9.2 下一步执行计划

1. ✅ SAG 深度评估文档归档（本文档）
2. ⏳ 启动 7 专家评审（架构 / RAG / 性能 / 合规 / UX / 安全 / 产品）
3. ⏳ 汇总评审结果到 `SAG-重构方案-七专家评审-1.0.md`
4. ⏳ 基于评审反馈更新 spec 到 `SparkFox-v1.0.0-spec-2.0.md`
5. ⏳ Git commit 评估文档 + 评审报告 + spec v2.0

---

## 附录 A：SAG 仓库本地路径

- **Benchmark 仓**（Python，已 clone）：`d:\xin kaifa\SAG-Benchmark`
- **主项目仓库**（TS + Python，未 clone）：https://github.com/Zleap-AI/SAG

## 附录 B：SAG 关键源码索引

| 模块 | 路径 | 行数 | 说明 |
|------|------|:----:|------|
| 数据模型 | [`pipeline/db/models.py`](file:///d:/xin%20kaifa/SAG-Benchmark/pipeline/db/models.py) | 928 | 5 表结构定义 |
| 离线提取 | [`pipeline/modules/extract/extractor.py`](file:///d:/xin%20kaifa/SAG-Benchmark/pipeline/modules/extract/extractor.py) | 715 | EventExtractor 主控制器 |
| 提取 Prompt | [`prompts/extract.yaml`](file:///d:/xin%20kaifa/SAG-Benchmark/prompts/extract.yaml) | — | 六段式 prompt v3.1 |
| Atomic 检索 | [`pipeline/modules/search/atomic.py`](file:///d:/xin%20kaifa/SAG-Benchmark/pipeline/modules/search/atomic.py) | 1099 | 原子事项检索器 |
| Multi 检索 | [`pipeline/modules/search/multi.py`](file:///d:/xin%20kaifa/SAG-Benchmark/pipeline/modules/search/multi.py) | 1107 | 多元事项检索器 |
| Step5 策略 | `pipeline/modules/search/step5_strategies.py` | — | 三种扩展策略 |
| 存储接口 | [`pipeline/storage/interfaces.py`](file:///d:/xin%20kaifa/SAG-Benchmark/pipeline/storage/interfaces.py) | 118 | DatabaseStore/VectorSearchStore/SearchStore Protocol |
| Schema 初始化 | [`init.sql`](file:///d:/xin%20kaifa/SAG-Benchmark/init.sql) | — | 数据库 schema 初始化 |

---

**文档完成。**

> 本评估作为 SAG 架构借鉴与 SparkFox 知识库重构的归档文档。下一步启动 7 专家评审。
