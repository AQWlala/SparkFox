# sparkfox-store

> SparkFox 数据存储 — SQLite + sqlite-vec 向量检索（6 层记忆持久化层）

## 功能

- **SQLite 持久化**：基于 rusqlite 0.32（`bundled` feature，无需系统 SQLite），WAL 模式 + NORMAL 同步
- **sqlite-vec 扩展**：通过 FFI `load_extension` 加载 sqlite-vec 动态库（不依赖 sqlite-vec crate，alpha 版本不稳定）
- **6 层记忆动态表名**（A-03 P0 修复）：`vector_insert` 按 `MemoryLayer` 枚举选择对应向量表
  - `L0Raw` → `vec_l0`（原始 chunk）
  - `L3Episodic` → `vec_l3_event`（SAG event）
  - `L3Semantic` / `L3GraphNode` → `vec_l3_entity`（SAG entity）
  - `L3GraphEdge` / `L3EventEntity` → `vec_l3_event_entity`（SAG 关联级嵌入）
- **HNSW 索引**（`vector_index::hnsw`）：当前为内存 HashMap + 暴力 cosine 占位实现（待 hnswlib-rs 修复 Windows 兼容后切换）
- **sqlite-vec 索引**（`vector_index::sqlite_vec`）：基于 sqlite-vec 扩展的真实向量索引
- **Schema 迁移**：`migrate_all` 创建 6 层记忆表 + SAG 6 表（DDL 来自 sparkfox-knowledge::schema）

## 架构

```
┌──────────────────────────────────────────────────────────┐
│                    sparkfox-store                        │
│  ┌──────────────────────────────────────────────────┐    │
│  │  Store (Connection + vec_loaded)                 │    │
│  │  ┌────────────────┐  ┌────────────────────────┐  │    │
│  │  │  schema.rs     │  │  vec.rs                │  │    │
│  │  │  migrate_all() │  │  load_vec_extension()  │  │    │
│  │  └────────────────┘  └────────────────────────┘  │    │
│  │                                                  │    │
│  │  ┌──────────────────────────────────────────┐    │    │
│  │  │  vector_insert(layer, id, vec)           │    │    │
│  │  │  vector_search(layer, query, top_k)      │    │    │
│  │  └──────────────────────────────────────────┘    │    │
│  └──────────────────────────────────────────────────┘    │
│                                                          │
│  ┌──────────────────────────────────────────────────┐    │
│  │  vector_index/                                    │    │
│  │  ├── hnsw.rs      (占位：HashMap + cosine)       │    │
│  │  └── sqlite_vec.rs (真实：sqlite-vec 扩展)       │    │
│  └──────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
                          ▲
                          │ MemoryLayer 枚举
                ┌─────────┴─────────┐
                │  sparkfox-memory  │
                └───────────────────┘
```

**A-03 P0 修复**：`vector_insert` 引入 `sparkfox-memory::MemoryLayer` 枚举（L0Raw / L3Episodic / L3Semantic 等 10 变体），动态选择向量表。

**P-03/P-04 P0 修复**：SAG schema 6 表 DDL 定义在 `sparkfox-knowledge::schema`，本 crate 通过 `migrate_knowledge_schema` 引用 `ALL_SAG_DDL` 常量执行迁移。

**依赖**：`sparkfox-core` / `sparkfox-memory` / `sparkfox-knowledge` / `rusqlite`

## 使用

```rust
use sparkfox_store::{Store, StoreConfig};
use sparkfox_memory::MemoryLayer;

// 打开数据库（自动加载 sqlite-vec 扩展 + 迁移 schema）
let store = Store::open(StoreConfig::for_path("./data/sparkfox.db"))?;

if !store.is_vec_loaded() {
    log::warn!("sqlite-vec 加载失败，向量功能降级");
}

// 插入向量（按记忆层选择表）
let vec = vec![0.1_f32, 0.2, 0.3, /* ... */];
store.vector_insert(MemoryLayer::L3Episodic, "evt_001", &vec)?;

// 检索
let hits = store.vector_search(MemoryLayer::L3Episodic, &query_vec, 10)?;
for (id, score) in hits {
    println!("{}: {:.4}", id, score);
}
```

## 测试

```bash
# 单元测试（lib）
cargo test -p sparkfox-store --lib

# PoC-4 性能测试（sqlite-vec 缺失时自动 skip）
cargo test -p sparkfox-store --test poc4_perf

# 全部测试
cargo test -p sparkfox-store
```

## 安全约束

- `#![deny(unsafe_code)]`（非 `forbid`）—— `vec.rs` 必须通过 FFI 加载 sqlite-vec 扩展（`Connection::load_extension` 是 unsafe）。`forbid` 无法被子模块 `allow` 覆盖，`deny` 可以。除 `vec.rs` 外，本 crate 其余代码仍禁用 unsafe
- 所有 SQL 查询使用参数化绑定（`?` 占位符），防止 SQL 注入
- 数据库文件路径经 canonicalize 校验，防路径遍历

## 已知限制

- **hnswlib-rs 0.10 暂未加入**：依赖 `off64 0.9` 在 Windows 编译失败（`std::os::unix::FileExt` 缺失）。当前 HnswIndex 为暴力 cosine 占位实现，待 hnswlib-rs 修复 Windows 兼容后切换
- **sqlite-vec 扩展需手动分发**：随安装包附带 `sqlite_vec.dll` / `.so` / `.dylib`

## 许可证

AGPL-3.0-only，详见工作区根 `LICENSE`。

## 致谢

- [rusqlite](https://github.com/rusqlite/rusqlite)（MIT）— SQLite Rust 绑定
- [sqlite-vec](https://github.com/asg017/sqlite-vec)（MIT）— SQLite 向量扩展（早期 alpha）
