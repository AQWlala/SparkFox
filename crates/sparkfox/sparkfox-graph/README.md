# sparkfox-graph

> SparkFox 通用图遍历引擎 — GraphBackend trait 抽象 + PetgraphBackend + KnowledgeGraphBackend

## 功能

- **GraphBackend trait 抽象**：解耦图遍历逻辑与底层存储（A-02 P0 修复）
- **KnowledgeGraphBackend**：反向引用 sparkfox-knowledge 的 SAG 3 表（`entity` / `knowledge_event` / `event_entity_relation`），保持单一 SoT
- **PetgraphBackend**：基于 petgraph `StableDiGraph` 的内存图 + SQLite 持久化实现，支持安全删除
- **MDRM 5 维多跳遍历**：`multi_hop_traverse` 支持 5 种方向配置（OpenAkita 清洁室重写 + R-07 LIMIT 阀门）
- **实体抽取**（`extractor`，Task 8.13）：从文本抽取实体节点
- **关系抽取**（`relation`，Task 8.14）：从文本抽取实体间关系

## 架构

```
                ┌──────────────────────────────────────┐
                │       sparkfox-graph (本 crate)       │
                │  ┌────────────────────────────────┐  │
                │  │  GraphBackend trait            │  │
                │  └────────┬───────────────────────┘  │
                │           │                           │
                │     ┌─────┴──────┐                    │
                │     ▼            ▼                    │
                │  KnowledgeGraph  PetgraphBackend      │
                │  Backend          (petgraph + SQLite) │
                │  (反向引用 SAG)                        │
                └──────┬───────────────────────────────┘
                       │ 通过 trait 抽象，无直接依赖
                       ▼
            ┌──────────────────────┐
            │  sparkfox-knowledge  │
            │   (SAG SoT)          │
            └──────────────────────┘
```

**A-02 P0 修复要点**：sparkfox-graph 不再维护独立 `graph_node` / `graph_edge` 表，避免与 sparkfox-knowledge 的 SAG 表双轨冲突。

**依赖**：`sparkfox-core` / `async-trait` / `rusqlite` / `tokio` / `petgraph`

## 使用

```rust
use sparkfox_graph::{
    multi_hop_traverse, GraphBackend, GraphNode, PetgraphBackend, TraversalConfig,
};

// 内存图（petgraph 后端）
let mut backend = PetgraphBackend::new_in_memory();

let n1 = GraphNode { id: "n1".into(), name: "SparkFox".into(), node_type: "Project".into() };
let n2 = GraphNode { id: "n2".into(), name: "Tauri".into(), node_type: "Framework".into() };
backend.add_node(&n1)?;
backend.add_node(&n2)?;
backend.add_edge("n1", "n2", "depends_on", 1.0)?;

// 多跳遍历
let cfg = TraversalConfig::default();
let results = multi_hop_traverse(&backend, "n1", &cfg)?;
assert!(results.iter().any(|n| n.id == "n2"));
```

## 测试

```bash
# 单元测试（lib）
cargo test -p sparkfox-graph --lib

# 全部测试
cargo test -p sparkfox-graph
```

## 安全约束

- `#![forbid(unsafe_code)]` — 全 crate 禁用 unsafe
- 所有 SQL 查询使用参数化绑定，防止 SQL 注入
- 多跳遍历强制 LIMIT 阀门（R-07），防止图爆炸攻击

## 许可证

AGPL-3.0-only，详见工作区根 `LICENSE` 与 `NOTICE`。

## 致谢

- [petgraph](https://github.com/petgraph/petgraph)（MIT/Apache-2.0）— Rust 通用图数据结构
- SAG 论文 arXiv:2606.15971 + SAG-Benchmark（MIT）— 图遍历策略参考
- OpenAkita（清洁室重写，无代码直接复用）— MDRM 多跳遍历设计参考
