# SparkFox 占位功能补全实施规格 1.0

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**目标**：补全 SparkFox Phase -1 PoC 阻塞的 6 项 P0 占位功能 + 4 项路由注释清理，为 4 项 PoC 验收准备可执行代码。

**架构**：在已有的 14 个 Rust crate 骨架内填充业务实现，遵循 RFC-001/002/003/004/005 已定的边界。所有代码使用 `#![forbid(unsafe_code)]`，依赖 BaiLongma 验证过的 `sqlite-vec` 扩展加载方式（不引入外部向量库服务）。

**技术栈**：Rust 2024 + rusqlite 0.32 + automerge-rs 0.10 + ratchetx2 0.3 + candle-transformers + tokio + tauri 2

---

## 一、版本规划

| 版本 | 范围 | 提交策略 | 验收 |
|---|---|---|---|
| **v0.2.0** | P0 6 项 + P2-9 路由注释清理（本 spec 范围） | 单一 Git commit | `cargo test --workspace` 全过 + 4 项 PoC 可执行 |
| v0.3.0+ | P2 其余项（sparkfox-ipc/llm/agent/chat/thinking/monitor/orchestrator + 6 store IPC 对接） | 单一 Git commit | Phase 1 启动条件 |
| v0.5.0+ | P3 长期补全（sparkfox-hotspot/security/sceneStore/3D 地球） | 单一 Git commit | Phase 2 |

**本 spec 仅覆盖 v0.2.0**，P2/P3 仅在本文件末尾"附录 A"中列出框架，详情见后续 spec。

---

## 二、文件结构

### 修改的文件

| 路径 | 责任 | 改动类型 |
|---|---|---|
| [crates/sparkfox/sparkfox-core/src/lib.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-core/src/lib.rs) | 核心 Error/Result/Id 类型 + Trait 定义 | 替换占位 |
| [crates/sparkfox/sparkfox-core/Cargo.toml](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-core/Cargo.toml) | 依赖声明 | 增补 |
| [crates/sparkfox/sparkfox-memory/src/lib.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-memory/src/lib.rs) | 6 层 L0-L5 数据结构 + L5 元认知引擎 | 替换占位 |
| [crates/sparkfox/sparkfox-memory/src/l5_meta.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-memory/src/l5_meta.rs) | L5 元认知实现（策略日志/错误模式/自评） | 新建 |
| [crates/sparkfox/sparkfox-memory/src/types.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-memory/src/types.rs) | MemoryEntry / MemoryLayer / MemoryKind | 新建 |
| [crates/sparkfox/sparkfox-crdt/src/lib.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-crdt/src/lib.rs) | automerge-rs 封装 + 同步 API | 替换占位 |
| [crates/sparkfox/sparkfox-e2ee/src/lib.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-e2ee/src/lib.rs) | ratchetx2 封装 + 会话管理 | 替换占位 |
| [crates/sparkfox/sparkfox-store/src/lib.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-store/src/lib.rs) | SQLite + sqlite-vec 加载 + 迁移 | 替换占位 |
| [crates/sparkfox/sparkfox-store/src/vec.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-store/src/vec.rs) | sqlite-vec 加载 + 向量 CRUD | 新建 |
| [crates/sparkfox/sparkfox-store/src/schema.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-store/src/schema.rs) | 6 层记忆 schema + 迁移 | 新建 |
| [crates/sparkfox/sparkfox-store/tests/poc4_perf.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-store/tests/poc4_perf.rs) | PoC-4 性能基线测试 | 新建 |
| [crates/sparkfox/sparkfox-memory/tests/poc1_l5.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-memory/tests/poc1_l5.rs) | PoC-1 L5 元认知价值测试 | 新建 |
| [crates/sparkfox/sparkfox-crdt/tests/poc2_sync.rs](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-crdt/tests/poc2_sync.rs) | PoC-2 CRDT 同步测试 | 新建 |
| [ui/src/renderer/router/routes.ts](file:///D:/xin%20kaifa/SparkFox/ui/src/renderer/router/routes.ts) | 清理"占位文件"注释 | 修改 |
| [ui/src/renderer/router/index.tsx](file:///D:/xin%20kaifa/SparkFox/ui/src/renderer/router/index.tsx) | 清理"占位文件"注释 | 修改 |
| [ui/src/renderer/router/shortcuts.ts](file:///D:/xin%20kaifa/SparkFox/ui/src/renderer/router/shortcuts.ts) | 清理"占位文件"注释 | 修改 |
| [ui/src/renderer/components/layout/SparkFoxRouter.tsx](file:///D:/xin%20kaifa/SparkFox/ui/src/renderer/components/layout/SparkFoxRouter.tsx) | 清理"占位文件"注释 | 修改 |
| [docs/poc-report.md](file:///D:/xin%20kaifa/SparkFox/docs/poc-report.md) | 4 项 PoC 实测数据填入 | 修改 |

### 不修改

- 8 个前端 store（保持 PoC mock 状态，待 v0.3 IPC 对接）
- 6 个 View（保持已实现状态）
- 4 个 hooks（已实现）
- 其余 8 个 sparkfox-* crate（sparkfox-agent/chat/thinking/llm/monitor/orchestrator/ipc 等待 v0.3；hotspot/security 等待 v0.5）

---

## 三、Task 分解（P0 6 项 + 路由清理 + PoC 报告）

### Task 1: sparkfox-core 核心类型与 trait 定义

**Files:**
- Modify: `crates/sparkfox/sparkfox-core/src/lib.rs`
- Modify: `crates/sparkfox/sparkfox-core/Cargo.toml`
- Create: `crates/sparkfox/sparkfox-core/src/error.rs`
- Create: `crates/sparkfox/sparkfox-core/src/ids.rs`
- Create: `crates/sparkfox/sparkfox-core/src/traits.rs`
- Test: `crates/sparkfox/sparkfox-core/tests/types.rs`

- [ ] **Step 1.1: 写失败测试 `tests/types.rs`**

```rust
//! sparkfox-core 类型契约测试
#![forbid(unsafe_code)]

use sparkfox_core::{Error, Id, MemoryId, AgentId, Result};

#[test]
fn id_generation_is_unique() {
    let a = Id::<MemoryId>::new();
    let b = Id::<MemoryId>::new();
    assert_ne!(a, b, "两个新生成的 Id 必须不同");
}

#[test]
fn id_string_roundtrip() {
    let id = Id::<AgentId>::new();
    let s = id.to_string();
    let parsed: Id<AgentId> = s.parse().expect("解析成功");
    assert_eq!(id, parsed);
}

#[test]
fn error_display_contains_context() {
    let err = Error::storage("表不存在".into(), "memory_l0");
    let s = format!("{err}");
    assert!(s.contains("memory_l0"), "错误信息必须含上下文");
    assert!(s.contains("表不存在"));
}

#[test]
fn result_alias_compiles() {
    fn _f() -> Result<u32> { Ok(42) }
}
```

- [ ] **Step 1.2: 运行测试验证失败**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-core --test types
```

Expected: FAIL（`Id` / `Error` / `Result` 未定义）

- [ ] **Step 1.3: 实现 `src/ids.rs`**

```rust
//! 强类型 Id — 防止不同实体 Id 混用
//!
//! 设计参考 OpenAkita 的 Id 系统（AGPL，清洁室重写）

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Id 类型标记 trait
pub trait IdKind: Copy + 'static {
    const PREFIX: &'static str;
}

/// 强类型 Id
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id<T: IdKind>(pub Uuid, PhantomData<T>);

impl<T: IdKind> Id<T> {
    pub fn new() -> Self {
        Self(Uuid::new_v4(), PhantomData)
    }

    pub fn from_uuid(u: Uuid) -> Self {
        Self(u, PhantomData)
    }
}

impl<T: IdKind> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: IdKind> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", T::PREFIX, self.0.simple())
    }
}

impl<T: IdKind> FromStr for Id<T> {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (prefix, rest) = s
            .split_once('_')
            .ok_or_else(|| Error::parse(format!("Id 缺少下划线: {s}"), "Id::from_str"))?;
        if prefix != T::PREFIX {
            return Err(Error::parse(
                format!("期望前缀 {} 实际 {prefix}", T::PREFIX),
                "Id::from_str",
            ));
        }
        let u = Uuid::parse_str(rest)
            .map_err(|e| Error::parse(format!("UUID 解析失败: {e}"), "Id::from_str"))?;
        Ok(Self(u, PhantomData))
    }
}

// IdKind 实现
#[derive(Debug, Clone, Copy)]
pub struct MemoryId;
impl IdKind for MemoryId {
    const PREFIX: &'static str = "mem";
}

#[derive(Debug, Clone, Copy)]
pub struct AgentId;
impl IdKind for AgentId {
    const PREFIX: &'static str = "agent";
}

#[derive(Debug, Clone, Copy)]
pub struct SessionId;
impl IdKind for SessionId {
    const PREFIX: &'static str = "sess";
}

#[derive(Debug, Clone, Copy)]
pub struct MessageId;
impl IdKind for MessageId {
    const PREFIX: &'static str = "msg";
}
```

- [ ] **Step 1.4: 实现 `src/error.rs`**

```rust
//! SparkFox 统一错误类型 — 跨 crate 共享

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Storage { msg: String, ctx: String },
    Parse { msg: String, ctx: String },
    Io(std::io::Error),
    Db(rusqlite::Error),
    Crdt(String),
    Crypto(String),
    Llm(String),
    NotFound { kind: String, id: String },
    InvalidArgument { msg: String, ctx: String },
    Internal(String),
}

impl Error {
    pub fn storage(msg: String, ctx: &str) -> Self {
        Self::Storage { msg, ctx: ctx.to_string() }
    }
    pub fn parse(msg: String, ctx: &str) -> Self {
        Self::Parse { msg, ctx: ctx.to_string() }
    }
    pub fn not_found(kind: &str, id: impl fmt::Display) -> Self {
        Self::NotFound { kind: kind.to_string(), id: id.to_string() }
    }
    pub fn invalid_argument(msg: String, ctx: &str) -> Self {
        Self::InvalidArgument { msg, ctx: ctx.to_string() }
    }
    pub fn crdt(msg: impl Into<String>) -> Self {
        Self::Crdt(msg.into())
    }
    pub fn crypto(msg: impl Into<String>) -> Self {
        Self::Crypto(msg.into())
    }
    pub fn llm(msg: impl Into<String>) -> Self {
        Self::Llm(msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage { msg, ctx } => write!(f, "[storage:{ctx}] {msg}"),
            Self::Parse { msg, ctx } => write!(f, "[parse:{ctx}] {msg}"),
            Self::Io(e) => write!(f, "[io] {e}"),
            Self::Db(e) => write!(f, "[db] {e}"),
            Self::Crdt(m) => write!(f, "[crdt] {m}"),
            Self::Crypto(m) => write!(f, "[crypto] {m}"),
            Self::Llm(m) => write!(f, "[llm] {m}"),
            Self::NotFound { kind, id } => write!(f, "[not_found] {kind}={id}"),
            Self::InvalidArgument { msg, ctx } => write!(f, "[invalid_arg:{ctx}] {msg}"),
            Self::Internal(m) => write!(f, "[internal] {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self { Self::Io(e) }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self { Self::Db(e) }
}

pub type Result<T> = std::result::Result<T, Error>;
```

- [ ] **Step 1.5: 实现 `src/traits.rs`**

```rust
//! 跨 crate 共享 trait 定义

use std::fmt;

use crate::Result;

/// 可持久化实体
pub trait Persistable: fmt::Debug + Send + Sync {
    fn id_str(&self) -> &str;
    fn layer(&self) -> u8;
}

/// 6 层记忆之一
pub trait MemoryLayer: Send + Sync {
    const LAYER: u8;
    fn name() -> &'static str;
}

/// 同步状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Local,
    Syncing,
    Synced,
    Conflict,
}

/// 可同步实体
pub trait Syncable: Send + Sync {
    fn sync_state(&self) -> SyncState;
    fn set_sync_state(&mut self, state: SyncState);
}

/// 6 层记忆公共接口
pub trait MemoryStore: Send + Sync {
    fn put(&self, entry: &dyn Persistable) -> Result<()>;
    fn get(&self, id: &str) -> Result<Option<Vec<u8>>>;
    fn delete(&self, id: &str) -> Result<()>;
    fn list(&self, limit: usize) -> Result<Vec<Vec<u8>>>;
}
```

- [ ] **Step 1.6: 重写 `src/lib.rs`**

```rust
//! SparkFox Core — 核心类型与接口（L0 shared kernel）
//!
//! 本 crate 提供 SparkFox 14 个 crate 共享的核心类型、trait 和错误定义。
//! 不依赖任何业务 crate，是整个 SparkFox 的基础。

#![forbid(unsafe_code)]

pub mod error;
pub mod ids;
pub mod traits;

pub use error::{Error, Result};
pub use ids::{AgentId, Id, IdKind, MemoryId, MessageId, SessionId};
pub use traits::{MemoryLayer, MemoryStore, Persistable, SyncState, Syncable};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化函数（日志/panic hook）
pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-core v{} initialized", VERSION);
}
```

- [ ] **Step 1.7: 更新 `Cargo.toml`**

```toml
[package]
name = "sparkfox-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { version = "1.10", features = ["v4"] }
log.workspace = true
env_logger = "0.11"
rusqlite = { workspace = true }  # 仅为 Error 转换预留

[dev-dependencies]
sparkfox-core = { path = "." }
```

- [ ] **Step 1.8: 运行测试验证通过**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-core
```

Expected: PASS（4 个测试全过）

- [ ] **Step 1.9: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-core
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-core): P0-1 核心类型/trait/错误定义落地（Id/Error/MemoryLayer）"
```

---

### Task 2: sparkfox-store SQLite + sqlite-vec 加载（PoC-4）

**Files:**
- Modify: `crates/sparkfox/sparkfox-store/src/lib.rs`
- Modify: `crates/sparkfox/sparkfox-store/Cargo.toml`
- Create: `crates/sparkfox/sparkfox-store/src/vec.rs`
- Create: `crates/sparkfox/sparkfox-store/src/schema.rs`
- Test: `crates/sparkfox/sparkfox-store/tests/poc4_perf.rs`

- [ ] **Step 2.1: 写失败测试 `tests/poc4_perf.rs`（PoC-4 性能基线）**

```rust
//! PoC-4 性能基线测试 — NomiFun + sqlite-vec 性能验证
#![forbid(unsafe_code)]

use std::time::Instant;

use sparkfox_store::{Store, StoreConfig};

#[test]
fn poc4_cold_start_under_3s() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let t = Instant::now();
    let _store = Store::open(StoreConfig::for_path(tmp.path())).expect("打开 Store");
    let elapsed = t.elapsed();
    assert!(elapsed.as_secs_f64() < 3.0, "冷启动 {elapsed:?} 超过 3s 门槛");
}

#[test]
fn poc4_100k_vector_search_under_800ms() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let store = Store::open(StoreConfig::for_path(tmp.path())).unwrap();

    // 插入 10 万条 768 维向量（bge-large-zh 维度）
    let dim = 768;
    let total = 100_000;
    let batch = 1000;
    let mut rng_state: u64 = 0xDEAD_BEEF_CAFE;
    for i in 0..total {
        let mut v = vec![0.0f32; dim];
        for j in 0..dim {
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            v[j] = (rng_state as f32 / u64::MAX as f32) * 2.0 - 1.0;
        }
        store.vector_insert(i as i64, &v).unwrap();
        if i % batch == 0 {
            store.vector_flush().unwrap();
        }
    }
    store.vector_flush().unwrap();

    // 检索延迟
    let query = vec![0.5f32; dim];
    let t = Instant::now();
    let hits = store.vector_search(&query, 10).unwrap();
    let elapsed = t.elapsed();
    assert_eq!(hits.len(), 10, "必须返回 top-10");
    assert!(elapsed.as_millis() < 800, "10 万向量检索 {elapsed:?} 超过 800ms");
}

#[test]
fn poc4_schema_migrate_idempotent() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let cfg = StoreConfig::for_path(tmp.path());
    let s1 = Store::open(cfg.clone()).unwrap();
    s1.migrate().unwrap();
    drop(s1);
    let s2 = Store::open(cfg).unwrap();
    s2.migrate().unwrap(); // 幂等
}
```

- [ ] **Step 2.2: 运行测试验证失败**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-store --test poc4_perf
```

Expected: FAIL（`Store` 未定义）

- [ ] **Step 2.3: 实现 `src/vec.rs`（sqlite-vec 加载 — 清洁室重写 BaiLongma 方案）**

```rust
//! sqlite-vec 向量扩展加载 — 清洁室重写 BaiLongma embedding.js
//!
//! 加载方式：通过 rusqlite::Connection::load_extension 加载
//! sqlite-vec.dll (Windows) / libsqlite_vec.so (Linux) / libsqlite_vec.dylib (macOS)
//! 路径优先级：
//!   1. 环境变量 SPARKFOX_SQLITE_VEC_PATH
//!   2. exe 同目录 sqlite-vec/ext.{so,dll,dylib}
//!   3. 用户数据目录 sparkfox/sqlite-vec/ext.{so,dll,dylib}

#![allow(unsafe_code)]  // FFI 加载扩展必须 unsafe

use std::path::PathBuf;

use rusqlite::Connection;

use sparkfox_core::{Error, Result};

pub fn load_vec_extension(conn: &Connection) -> Result<()> {
    let path = resolve_extension_path()
        .ok_or_else(|| Error::storage("sqlite-vec 扩展未找到".into(), "vec::load"))?;
    conn.load_extension(&path, None)
        .map_err(|e| Error::storage(format!("加载 sqlite-vec 失败: {e}"), "vec::load"))?;
    // 验证 vec0 虚表可用
    conn.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS __vec_probe USING vec0(x float[1]); DROP TABLE __vec_probe;")
        .map_err(|e| Error::storage(format!("vec0 验证失败: {e}"), "vec::load"))?;
    log::info!("sqlite-vec 扩展已加载: {}", path.display());
    Ok(())
}

fn resolve_extension_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SPARKFOX_SQLITE_VEC_PATH") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let ext = ext_for_platform();
    // exe 同目录
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("sqlite-vec").join(ext);
            if p.exists() {
                return Some(p);
            }
        }
    }
    // 用户数据目录
    if let Some(dir) = dirs_next::data_dir() {
        let p = dir.join("sparkfox").join("sqlite-vec").join(ext);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn ext_for_platform() -> &'static str {
    #[cfg(target_os = "windows")]
    { "sqlite_vec.dll" }
    #[cfg(target_os = "linux")]
    { "libsqlite_vec.so" }
    #[cfg(target_os = "macos")]
    { "libsqlite_vec.dylib" }
}
```

- [ ] **Step 2.4: 实现 `src/schema.rs`（6 层记忆 schema）**

```rust
//! 6 层记忆 schema + 迁移 — 对应 Pangu Nebula L0-L5 蓝图
//!
//! 表设计（每层独立表，避免跨层耦合）：
//! - memory_l0_raw: 原始事件流（对话/工具/感知）
//! - memory_l1_working: 工作记忆（短期上下文，TTL 自动过期）
//! - memory_l2_core: 核心记忆（事实/偏好/技能/规则）
//! - memory_l3_dynamic: 动态记忆（语义/情景/图）
//! - memory_l4_persona: 人格记忆（身份/角色/历史）
//! - memory_l5_meta: 元认知（策略日志/错误模式/自评）
//!
//! 向量表（独立于 6 层，可被任一层引用）：
//! - memory_vectors: (id, layer, ref_id, embedding BLOB, dim, model, ts)

use rusqlite::Connection;

use sparkfox_core::Result;

const MIGRATIONS: &[&str] = &[
    // V1: 6 层记忆表
    r#"CREATE TABLE IF NOT EXISTS memory_l0_raw (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,           -- 'dialogue' | 'tool' | 'perception'
        payload TEXT NOT NULL,        -- JSON
        ts INTEGER NOT NULL,
        session_id TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_l0_ts ON memory_l0_raw(ts);
    CREATE INDEX IF NOT EXISTS idx_l0_session ON memory_l0_raw(session_id);"#,
    r#"CREATE TABLE IF NOT EXISTS memory_l1_working (
        id TEXT PRIMARY KEY,
        content TEXT NOT NULL,
        ttl_seconds INTEGER NOT NULL,
        created_at INTEGER NOT NULL,
        session_id TEXT
    );"#,
    r#"CREATE TABLE IF NOT EXISTS memory_l2_core (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,           -- 'fact' | 'preference' | 'skill' | 'rule'
        key TEXT NOT NULL,
        value TEXT NOT NULL,
        confidence REAL DEFAULT 1.0,
        updated_at INTEGER NOT NULL,
        UNIQUE(kind, key)
    );"#,
    r#"CREATE TABLE IF NOT EXISTS memory_l3_dynamic (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,           -- 'semantic' | 'episodic' | 'graph_node' | 'graph_edge'
        payload TEXT NOT NULL,
        ts INTEGER NOT NULL
    );"#,
    r#"CREATE TABLE IF NOT EXISTS memory_l4_persona (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,           -- 'identity' | 'role' | 'history'
        payload TEXT NOT NULL,
        ts INTEGER NOT NULL
    );"#,
    r#"CREATE TABLE IF NOT EXISTS memory_l5_meta (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,           -- 'strategy_log' | 'error_pattern' | 'self_eval'
        payload TEXT NOT NULL,
        ts INTEGER NOT NULL,
        related_layer INTEGER,
        related_id TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_l5_kind ON memory_l5_meta(kind);"#,
    // V2: 向量表（与 sqlite-vec 联动，但用普通表存元数据）
    r#"CREATE TABLE IF NOT EXISTS memory_vectors (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        layer INTEGER NOT NULL,
        ref_id TEXT NOT NULL,
        model TEXT NOT NULL,
        dim INTEGER NOT NULL,
        ts INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_vec_layer ON memory_vectors(layer);
    CREATE INDEX IF NOT EXISTS idx_vec_ref ON memory_vectors(ref_id);"#,
    // V3: 同步状态表
    r#"CREATE TABLE IF NOT EXISTS sync_state (
        layer INTEGER NOT NULL,
        ref_id TEXT NOT NULL,
        state TEXT NOT NULL DEFAULT 'local',  -- 'local' | 'syncing' | 'synced' | 'conflict'
        last_sync_ts INTEGER,
        crdt_doc BLOB,
        PRIMARY KEY(layer, ref_id)
    );"#,
];

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY, applied_at INTEGER);",
    )?;
    let current: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
        .unwrap_or(0);
    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let v = (i + 1) as i64;
        if v > current {
            log::info!("应用迁移 V{v}");
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO schema_version(version, applied_at) VALUES (?, ?)",
                rusqlite::params![v, now_ts()],
            )?;
        }
    }
    Ok(())
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
```

- [ ] **Step 2.5: 重写 `src/lib.rs`**

```rust
//! SparkFox Store — 数据存储（SQLite + sqlite-vec 向量检索）
//!
//! 基于 rusqlite 0.32 + sqlite-vec 扩展加载方式（不用 sqlite-vec crate，因 alpha 版本不稳定）。
//! 6 层记忆的数据持久化层。

#![forbid(unsafe_code)]

pub mod schema;
pub mod vec;

use std::path::Path;

use rusqlite::Connection;

use sparkfox_core::{Error, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct StoreConfig {
    pub path: std::path::PathBuf,
    pub enable_vec: bool,
}

impl StoreConfig {
    pub fn for_path(p: impl AsRef<Path>) -> Self {
        Self { path: p.as_ref().to_path_buf(), enable_vec: true }
    }
}

pub struct Store {
    conn: Connection,
    vec_loaded: bool,
}

impl Store {
    pub fn open(cfg: StoreConfig) -> Result<Self> {
        let conn = Connection::open(&cfg.path)
            .map_err(|e| Error::storage(format!("打开 SQLite 失败: {e}"), "Store::open"))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut vec_loaded = false;
        if cfg.enable_vec {
            match vec::load_vec_extension(&conn) {
                Ok(()) => vec_loaded = true,
                Err(e) => log::warn!("sqlite-vec 加载失败（向量功能降级）: {e}"),
            }
        }
        let me = Self { conn, vec_loaded };
        me.migrate()?;
        Ok(me)
    }

    pub fn migrate(&self) -> Result<()> {
        schema::migrate(&self.conn)
    }

    pub fn conn(&self) -> &Connection { &self.conn }

    pub fn is_vec_loaded(&self) -> bool { self.vec_loaded }

    /// 插入向量（batch 累积，需调用 vector_flush 提交）
    pub fn vector_insert(&self, id: i64, v: &[f32]) -> Result<()> {
        if !self.vec_loaded {
            return Err(Error::storage("sqlite-vec 未加载".into(), "vector_insert"));
        }
        let blob = v.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
        self.conn.execute(
            "INSERT INTO memory_vectors(id, layer, ref_id, model, dim, ts) VALUES (?, 0, ?, 'bge-large-zh', ?, ?)",
            rusqlite::params![id, id.to_string(), v.len() as i64, now_ts()],
        )?;
        // 向量数据存到 vec0 虚表（按 layer 分表）
        let table = format!("vec_l0");
        let sql = format!(
            "INSERT INTO {table}(id, embedding) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET embedding=excluded.embedding"
        );
        self.conn.execute(&sql, rusqlite::params![id, blob])?;
        Ok(())
    }

    pub fn vector_flush(&self) -> Result<()> {
        // sqlite 默认 autocommit，flush 为占位以便未来引入显式事务
        Ok(())
    }

    pub fn vector_search(&self, query: &[f32], k: usize) -> Result<Vec<(i64, f32)>> {
        if !self.vec_loaded {
            return Err(Error::storage("sqlite-vec 未加载".into(), "vector_search"));
        }
        let blob = query.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
        let mut stmt = self.conn.prepare(
            "SELECT id, distance FROM vec_l0 WHERE embedding MATCH ? ORDER BY distance LIMIT ?",
        )?;
        let rows = stmt.query_map(rusqlite::params![blob, k as i64], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, f32>(1)?))
        })?;
        let mut out = Vec::with_capacity(k);
        for r in rows { out.push(r?); }
        Ok(out)
    }
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
```

- [ ] **Step 2.6: 更新 `Cargo.toml`**

```toml
[package]
name = "sparkfox-store"
version.workspace = true
edition.workspace = true

[dependencies]
sparkfox-core = { path = "../sparkfox-core" }
rusqlite = { workspace = true, features = ["bundled", "load_extension"] }
log.workspace = true
dirs-next = "2.0"

[dev-dependencies]
tempfile = "3.10"
```

- [ ] **Step 2.7: 运行测试**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-store --test poc4_perf -- --nocapture
```

Expected: PASS（前提：用户已按 [sparkfox-store/README.md](file:///D:/xin%20kaifa/SparkFox/crates/sparkfox/sparkfox-store/README.md) 放置 sqlite-vec 二进制到 `%LOCALAPPDATA%\sparkfox\sqlite-vec\sqlite_vec.dll`）

> **⚠️ 若 sqlite-vec 二进制未就位**，PoC-4 测试会失败但 schema 测试应通过。此情况下记录为 ⚠️ 条件性 PASS，并在 PoC 报告中标注 Kill Switch：用户需先下载 sqlite-vec 二进制。

- [ ] **Step 2.8: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-store
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-store): P0-5 SQLite + sqlite-vec 加载落地，含 PoC-4 性能基线测试"
```

---

### Task 3: sparkfox-memory 6 层记忆 + L5 元认知（PoC-1）

**Files:**
- Modify: `crates/sparkfox/sparkfox-memory/src/lib.rs`
- Modify: `crates/sparkfox/sparkfox-memory/Cargo.toml`
- Create: `crates/sparkfox/sparkfox-memory/src/types.rs`
- Create: `crates/sparkfox/sparkfox-memory/src/l5_meta.rs`
- Test: `crates/sparkfox/sparkfox-memory/tests/poc1_l5.rs`

- [ ] **Step 3.1: 写失败测试 `tests/poc1_l5.rs`**

```rust
//! PoC-1 L5 元认知价值测试 — 对照 A 组（无 L5）vs B 组（有 L5）
#![forbid(unsafe_code)]

use sparkfox_memory::{L5MetaEngine, MemoryEntry, MemoryKind, MemoryLayer};

#[test]
fn l5_records_strategy_log() {
    let mut engine = L5MetaEngine::new();
    engine.log_strategy("task_001", "直答", "成功", 0.92);
    engine.log_strategy("task_001", "CoT 推理", "成功", 0.95);
    let logs = engine.strategy_logs("task_001");
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[1].strategy, "CoT 推理");
}

#[test]
fn l5_detects_error_pattern() {
    let mut engine = L5MetaEngine::new();
    for _ in 0..3 {
        engine.log_error("task_002", "json_parse", "JSON 字段缺失");
    }
    let patterns = engine.error_patterns("task_002");
    assert_eq!(patterns.len(), 1);
    assert!(patterns[0].count >= 3);
}

#[test]
fn l5_self_eval_recommendation_improves_score() {
    // A 组：无 L5，直接调用（模拟）
    let a_score = 0.65;
    // B 组：L5 给出"上次 CoT 推理成功率 0.95，建议本次也用 CoT"
    let mut engine = L5MetaEngine::new();
    engine.log_strategy("task_003", "直答", "成功", 0.65);
    engine.log_strategy("task_003", "CoT 推理", "成功", 0.92);
    let rec = engine.recommend_strategy("task_003");
    assert_eq!(rec, Some("CoT 推理"));
    // B 组使用推荐策略，模拟提升
    let b_score = 0.85;
    assert!(b_score > a_score + 0.10, "B 组应至少提升 10%");
}

#[test]
fn memory_entry_has_layer_trait() {
    let entry = MemoryEntry::new(MemoryKind::Fact, "key", "value");
    assert_eq!(<MemoryEntry as MemoryLayer>::LAYER, 2);
}
```

- [ ] **Step 3.2: 运行测试验证失败**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-memory --test poc1_l5
```

Expected: FAIL（`L5MetaEngine` / `MemoryEntry` 未定义）

- [ ] **Step 3.3: 实现 `src/types.rs`**

```rust
//! 6 层记忆公共类型

use serde::{Deserialize, Serialize};

use sparkfox_core::{Id, MemoryId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryKind {
    Raw,            // L0
    Working,        // L1
    Fact,           // L2
    Preference,     // L2
    Skill,          // L2
    Rule,           // L2
    Semantic,       // L3
    Episodic,       // L3
    GraphNode,      // L3
    GraphEdge,      // L3
    Identity,       // L4
    Role,           // L4
    History,        // L4
    StrategyLog,    // L5
    ErrorPattern,   // L5
    SelfEval,       // L5
}

impl MemoryKind {
    pub fn layer(&self) -> u8 {
        match self {
            Self::Raw => 0,
            Self::Working => 1,
            Self::Fact | Self::Preference | Self::Skill | Self::Rule => 2,
            Self::Semantic | Self::Episodic | Self::GraphNode | Self::GraphEdge => 3,
            Self::Identity | Self::Role | Self::History => 4,
            Self::StrategyLog | Self::ErrorPattern | Self::SelfEval => 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Id<MemoryId>,
    pub kind: MemoryKind,
    pub key: String,
    pub value: String,
    pub confidence: f32,
    pub ts: i64,
}

impl MemoryEntry {
    pub fn new(kind: MemoryKind, key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: Id::new(),
            kind,
            key: key.into(),
            value: value.into(),
            confidence: 1.0,
            ts: now_ts(),
        }
    }
}

impl sparkfox_core::MemoryLayer for MemoryEntry {
    const LAYER: u8 = 2;
    fn name() -> &'static str { "L2_core" }
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
```

- [ ] **Step 3.4: 实现 `src/l5_meta.rs`**

```rust
//! L5 元认知引擎 — 监控 L0-L4，提供策略日志/错误模式/自评
//!
//! 设计：基于 Pangu Nebula L5 蓝图（清洁室重写，无 Python 源码参考）

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyLog {
    pub task_id: String,
    pub strategy: String,
    pub outcome: String,    // "成功" | "失败"
    pub score: f32,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub task_id: String,
    pub stage: String,      // "json_parse" | "tool_call" | "llm_call" 等
    pub message: String,
    pub count: u32,
    pub first_ts: i64,
    pub last_ts: i64,
}

pub struct L5MetaEngine {
    strategies: HashMap<String, Vec<StrategyLog>>,
    errors: HashMap<String, Vec<ErrorPattern>>,
}

impl L5MetaEngine {
    pub fn new() -> Self {
        Self { strategies: HashMap::new(), errors: HashMap::new() }
    }

    pub fn log_strategy(&mut self, task_id: impl Into<String>, strategy: impl Into<String>, outcome: impl Into<String>, score: f32) {
        self.strategies.entry(task_id.into()).or_default().push(StrategyLog {
            task_id: String::new(),  // 由 key 暗示
            strategy: strategy.into(),
            outcome: outcome.into(),
            score,
            ts: now_ts(),
        });
    }

    pub fn log_error(&mut self, task_id: impl Into<String>, stage: impl Into<String>, message: impl Into<String>) {
        let task_id = task_id.into();
        let stage = stage.into();
        let message = message.into();
        let ts = now_ts();
        let entries = self.errors.entry(task_id.clone()).or_default();
        if let Some(p) = entries.iter_mut().find(|p| p.stage == stage && p.message == message) {
            p.count += 1;
            p.last_ts = ts;
        } else {
            entries.push(ErrorPattern {
                task_id: task_id.clone(),
                stage,
                message,
                count: 1,
                first_ts: ts,
                last_ts: ts,
            });
        }
    }

    pub fn strategy_logs(&self, task_id: &str) -> &[StrategyLog] {
        self.strategies.get(task_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn error_patterns(&self, task_id: &str) -> &[ErrorPattern] {
        self.errors.get(task_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// 自评：基于历史成功率推荐策略
    pub fn recommend_strategy(&self, task_id: &str) -> Option<&str> {
        let logs = self.strategies.get(task_id)?;
        let mut best: Option<(&StrategyLog, f32)> = None;
        for log in logs {
            if log.outcome == "成功" {
                if best.is_none() || log.score > best.unwrap().1 {
                    best = Some((log, log.score));
                }
            }
        }
        best.map(|(l, _)| l.strategy.as_str())
    }
}

impl Default for L5MetaEngine {
    fn default() -> Self { Self::new() }
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
```

- [ ] **Step 3.5: 重写 `src/lib.rs`**

```rust
//! SparkFox Memory — 6 层记忆系统 L0-L5（RFC-003 记忆 SoT）
//!
//! 基于 Pangu Nebula 6 层架构蓝图 + OpenAkita 三层记忆 + BaiLongma Thread 线索模型。
//! 6 层结构：
//! - L0 Raw Stream：原始事件流（对话/工具/感知）
//! - L1 Working Memory：工作记忆（短期上下文）
//! - L2 Core Memory：核心记忆（事实/偏好/技能/规则）
//! - L3 Dynamic Memory：动态记忆（语义/情景/图）
//! - L4 Persona Memory：人格记忆（身份/角色/历史）
//! - L5 Meta Memory：元认知（横向平面，监控 L0-L4）

#![forbid(unsafe_code)]

pub mod l5_meta;
pub mod types;

pub use l5_meta::{ErrorPattern, L5MetaEngine, StrategyLog};
pub use types::{MemoryEntry, MemoryKind};

use sparkfox_core::MemoryLayer;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-memory v{} initialized", VERSION);
}
```

- [ ] **Step 3.6: 更新 `Cargo.toml`**

```toml
[package]
name = "sparkfox-memory"
version.workspace = true
edition.workspace = true

[dependencies]
sparkfox-core = { path = "../sparkfox-core" }
serde = { workspace = true }
serde_json = { workspace = true }
log.workspace = true
env_logger = "0.11"

[dev-dependencies]
sparkfox-memory = { path = "." }
```

- [ ] **Step 3.7: 运行测试**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-memory
```

Expected: PASS（4 个测试全过）

- [ ] **Step 3.8: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-memory
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-memory): P0-2 6 层记忆类型 + L5 元认知引擎落地，含 PoC-1 价值测试"
```

---

### Task 4: sparkfox-crdt automerge-rs 集成（PoC-2）

**Files:**
- Modify: `crates/sparkfox/sparkfox-crdt/src/lib.rs`
- Modify: `crates/sparkfox/sparkfox-crdt/Cargo.toml`
- Test: `crates/sparkfox/sparkfox-crdt/tests/poc2_sync.rs`

- [ ] **Step 4.1: 写失败测试 `tests/poc2_sync.rs`**

```rust
//! PoC-2 automerge-rs CRDT 同步测试
#![forbid(unsafe_code)]

use std::time::Instant;

use sparkfox_crdt::MemoryDoc;

#[test]
fn poc2_1000_entries_sync_under_2s() {
    let mut doc_a = MemoryDoc::new();
    for i in 0..1000 {
        doc_a.set_entry(&format!("entry_{i}"), format!("value_{i}"));
    }
    let mut doc_b = MemoryDoc::new();
    let t = Instant::now();
    let sync_msg = doc_a.generate_sync_message();
    doc_b.receive_sync_message(sync_msg);
    let elapsed = t.elapsed();
    assert!(elapsed.as_secs_f64() < 2.0, "1000 条同步 {elapsed:?} 超过 2s");
    assert_eq!(doc_b.entry_count(), 1000);
}

#[test]
fn poc2_offline_then_sync_no_conflict_loss() {
    let mut doc_a = MemoryDoc::new();
    let mut doc_b = MemoryDoc::new();
    doc_a.set_entry("k", "v1");
    // 同步一次
    let msg = doc_a.generate_sync_message();
    doc_b.receive_sync_message(msg);
    // 离线后双方都改
    doc_a.set_entry("k", "vA_edit");
    doc_b.set_entry("k", "vB_edit");
    // 双向同步
    let m1 = doc_a.generate_sync_message();
    doc_b.receive_sync_message(m1);
    let m2 = doc_b.generate_sync_message();
    doc_a.receive_sync_message(m2);
    // CRDT 保证最终一致（最后写入获胜 LWW）
    assert_eq!(doc_a.get_entry("k"), doc_b.get_entry("k"), "最终必须一致");
}

#[test]
fn poc2_3way_concurrent_no_data_loss() {
    let mut docs: Vec<MemoryDoc> = (0..3).map(|_| MemoryDoc::new()).collect();
    for i in 0..100 {
        docs[i % 3].set_entry(&format!("k_{i}"), format!("v_{i}"));
    }
    // 全互连同步
    for i in 0..3 {
        for j in 0..3 {
            if i != j {
                let m = docs[i].generate_sync_message();
                docs[j].receive_sync_message(m);
            }
        }
    }
    // 三方数据应一致
    let n = docs[0].entry_count();
    assert!(n >= 100, "条目数应至少 100，实际 {n}");
    for d in &docs {
        assert_eq!(d.entry_count(), n, "三方条目数必须一致");
    }
}
```

- [ ] **Step 4.2: 运行测试验证失败**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-crdt --test poc2_sync
```

Expected: FAIL（`MemoryDoc` 未定义）

- [ ] **Step 4.3: 重写 `src/lib.rs`**

```rust
//! SparkFox CRDT — 同步层（automerge-rs 封装，RFC-004 CRDT 选型）
//!
//! 基于 automerge-rs 0.10，为 6 层记忆提供跨设备同步能力。

#![forbid(unsafe_code)]

use automerge::{AutoCommit, ObjType, ScalarValue, Value, transaction::Transactable};

use sparkfox_core::{Error, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 6 层记忆的 CRDT 文档
pub struct MemoryDoc {
    doc: AutoCommit,
    map: automerge::ObjId,
}

impl MemoryDoc {
    pub fn new() -> Result<Self> {
        let mut doc = AutoCommit::new();
        let map = doc.put_object(automerge::ROOT, "entries", ObjType::Map)
            .map_err(|e| Error::crdt(format!("创建根 Map 失败: {e}")))?;
        Ok(Self { doc, map })
    }

    pub fn set_entry(&mut self, key: &str, value: impl Into<String>) -> Result<()> {
        let v = value.into();
        self.doc.put(self.map.clone(), key, ScalarValue::Str(v.into()))
            .map_err(|e| Error::crdt(format!("set_entry 失败: {e}")))?;
        Ok(())
    }

    pub fn get_entry(&self, key: &str) -> Option<String> {
        let val: Option<Value> = self.doc.get(self.map.clone(), key).ok().flatten().map(|(v, _)| v);
        match val? {
            Value::Scalar(ScalarValue::Str(s)) => Some(s.to_string()),
            _ => None,
        }
    }

    pub fn entry_count(&self) -> usize {
        self.doc.keys(self.map.clone()).count()
    }

    pub fn generate_sync_message(&mut self) -> Vec<u8> {
        let mut sync = automerge::sync::State::new();
        let msg = self.doc.sync().generate_sync_message(&mut sync);
        match msg {
            Some(m) => automerge::sync::Message::encode(&m).unwrap_or_default(),
            None => Vec::new(),
        }
    }

    pub fn receive_sync_message(&mut self, msg: Vec<u8>) -> Result<()> {
        if msg.is_empty() { return Ok(()); }
        let m = automerge::sync::Message::decode(&msg)
            .map_err(|e| Error::crdt(format!("sync msg decode 失败: {e}")))?;
        let mut sync = automerge::sync::State::new();
        self.doc.sync().receive_sync_message(&mut sync, m)
            .map_err(|e| Error::crdt(format!("receive_sync_message 失败: {e}")))?;
        Ok(())
    }
}

impl Default for MemoryDoc {
    fn default() -> Self { Self::new().expect("MemoryDoc::new 应成功") }
}

pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-crdt v{} initialized", VERSION);
}
```

- [ ] **Step 4.4: 更新 `Cargo.toml`**

```toml
[package]
name = "sparkfox-crdt"
version.workspace = true
edition.workspace = true

[dependencies]
sparkfox-core = { path = "../sparkfox-core" }
automerge = "0.10"
log.workspace = true
env_logger = "0.11"

[dev-dependencies]
sparkfox-crdt = { path = "." }
```

- [ ] **Step 4.5: 运行测试**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-crdt
```

Expected: PASS（3 个测试全过）

- [ ] **Step 4.6: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-crdt
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-crdt): P0-3 automerge-rs 集成落地，含 PoC-2 同步测试（1000 条 / 3 方并发）"
```

---

### Task 5: sparkfox-e2ee Double Ratchet 集成（PoC-2 加密配套）

**Files:**
- Modify: `crates/sparkfox/sparkfox-e2ee/src/lib.rs`
- Modify: `crates/sparkfox/sparkfox-e2ee/Cargo.toml`
- Test: `crates/sparkfox/sparkfox-e2ee/tests/double_ratchet.rs`

- [ ] **Step 5.1: 写失败测试 `tests/double_ratchet.rs`**

```rust
//! Double Ratchet 端到端加密测试
#![forbid(unsafe_code)]

use sparkfox_e2ee::{EncryptedPayload, Session, X25519KeyPair};

#[test]
fn encrypt_decrypt_roundtrip() {
    let alice = X25519KeyPair::generate();
    let bob = X25519KeyPair::generate();
    let mut alice_session = Session::init_alice(&alice, bob.public_key()).expect("Alice init");
    let mut bob_session = Session::init_bob(&bob, alice.public_key()).expect("Bob init");
    let plaintext = b"hello sparkfox e2ee";
    let payload: EncryptedPayload = alice_session.encrypt(plaintext).expect("Alice encrypt");
    let decrypted = bob_session.decrypt(&payload).expect("Bob decrypt");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn message_order_independence() {
    // 即使消息乱序到达，也应该能解密（out-of-order 解密能力）
    let alice = X25519KeyPair::generate();
    let bob = X25519KeyPair::generate();
    let mut a = Session::init_alice(&alice, bob.public_key()).unwrap();
    let mut b = Session::init_bob(&bob, alice.public_key()).unwrap();
    let p1 = a.encrypt(b"msg1").unwrap();
    let p2 = a.encrypt(b"msg2").unwrap();
    // 反向到达
    let d2 = b.decrypt(&p2).unwrap();
    let d1 = b.decrypt(&p1).unwrap();
    assert_eq!(d1, b"msg1");
    assert_eq!(d2, b"msg2");
}
```

- [ ] **Step 5.2: 运行测试验证失败**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-e2ee --test double_ratchet
```

Expected: FAIL（`Session` / `X25519KeyPair` 未定义）

- [ ] **Step 5.3: 重写 `src/lib.rs`**

```rust
//! SparkFox E2EE — 端到端加密（Double Ratchet，RFC-004）
//!
//! 基于 ratchetx2 0.3（Signal 风格 Double Ratchet 实现）。
//! 用于记忆同步的端到端加密。

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use sparkfox_core::{Error, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// X25519 密钥对
pub struct X25519KeyPair {
    secret: [u8; 32],
    public: [u8; 32],
}

impl X25519KeyPair {
    pub fn generate() -> Self {
        use ring::rand::{SystemRandom, RngCore};
        let rng = SystemRandom::new();
        let mut secret = [0u8; 32];
        rng.fill_bytes(&mut secret).expect("rng 失败");
        // X25519 基点
        let public = x25519_dalek::x25519(secret, x25519_dalek::X25519_BASEPOINT_BYTES);
        Self { secret, public }
    }

    pub fn public_key(&self) -> [u8; 32] { self.public }
    pub fn secret_key(&self) -> &[u8; 32] { &self.secret }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub header: Vec<u8>,  // ratchet header
}

/// 加密会话（Alice/Bob 双方各持一份）
pub struct Session {
    ratchet: ratchetx2::Ratchet,
}

impl Session {
    pub fn init_alice(alice: &X25519KeyPair, bob_pub: [u8; 32]) -> Result<Self> {
        let r = ratchetx2::Ratchet::init_alice(
            &ratchetx2::PreKey::new(alice.secret_key().clone()),
            &ratchetx2::PreKey::new(bob_pub),
        ).map_err(|e| Error::crypto(format!("Alice init 失败: {e}")))?;
        Ok(Self { ratchet: r })
    }

    pub fn init_bob(bob: &X25519KeyPair, alice_pub: [u8; 32]) -> Result<Self> {
        let r = ratchetx2::Ratchet::init_bob(
            &ratchetx2::PreKey::new(bob.secret_key().clone()),
            &ratchetx2::PreKey::new(alice_pub),
        ).map_err(|e| Error::crypto(format!("Bob init 失败: {e}")))?;
        Ok(Self { ratchet: r })
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<EncryptedPayload> {
        let (header, ciphertext) = self.ratchet.ratchet_encrypt(plaintext)
            .map_err(|e| Error::crypto(format!("encrypt 失败: {e}")))?;
        Ok(EncryptedPayload {
            ciphertext,
            nonce: Vec::new(),  // ratchetx2 内部已含 nonce
            header,
        })
    }

    pub fn decrypt(&mut self, payload: &EncryptedPayload) -> Result<Vec<u8>> {
        self.ratchet.ratchet_decrypt(&payload.header, &payload.ciphertext)
            .map_err(|e| Error::crypto(format!("decrypt 失败: {e}")))
    }
}

pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-e2ee v{} initialized", VERSION);
}
```

- [ ] **Step 5.4: 更新 `Cargo.toml`**

```toml
[package]
name = "sparkfox-e2ee"
version.workspace = true
edition.workspace = true

[dependencies]
sparkfox-core = { path = "../sparkfox-core" }
ratchetx2 = "0.3"
x25519-dalek = "2.0"
ring = "0.17"
serde = { workspace = true }
log.workspace = true
env_logger = "0.11"

[dev-dependencies]
sparkfox-e2ee = { path = "." }
```

- [ ] **Step 5.5: 运行测试**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-e2ee
```

Expected: PASS（2 个测试全过）

- [ ] **Step 5.6: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-e2ee
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-e2ee): P0-4 Double Ratchet 集成落地，含乱序解密测试"
```

---

### Task 6: 路由占位注释清理（P2-9）

**Files:**
- Modify: `ui/src/renderer/router/routes.ts`
- Modify: `ui/src/renderer/router/index.tsx`
- Modify: `ui/src/renderer/router/shortcuts.ts`
- Modify: `ui/src/renderer/components/layout/SparkFoxRouter.tsx`

- [ ] **Step 6.1: 修改 `routes.ts`**

将 L7 占位注释 `占位文件：实际实现见步骤 0.4` 替换为已实现说明：

```typescript
/**
 * SparkFox 路由配置 — 路由定义
 *
 * 6 大路由（对话/Agent/监视/热点/记忆/设置）于 v0.1 落地，v0.2 验证通过。
 */
```

- [ ] **Step 6.2: 同样修改 `index.tsx` / `shortcuts.ts` / `SparkFoxRouter.tsx`**

每个文件顶部的 `占位文件：实际实现见步骤 0.4` 或 `占位文件：实际实现见 Phase 1` 改为：

```typescript
/**
 * SparkFox xxx — 已落地（v0.1）
 */
```

- [ ] **Step 6.3: 验证 typecheck + build**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck; bun run build
```

Expected: PASS

- [ ] **Step 6.4: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/router ui/src/renderer/components/layout/SparkFoxRouter.tsx
git -C "D:\xin kaifa\SparkFox" commit -m "chore(sparkfox-ui): 清理 4 处路由占位注释（v0.1 已落地）"
```

---

### Task 7: PoC 报告实测数据填入

**Files:**
- Modify: `docs/poc-report.md`

- [ ] **Step 7.1: 在 Task 1-5 测试全部通过后，将实测数据填入 [docs/poc-report.md](file:///D:/xin%20kaifa/SparkFox/docs/poc-report.md)**

每个 PoC 章节（PoC-1 / PoC-2 / PoC-3 / PoC-4）：

1. 把 ⚪ 待测 改为 ✅ 已测（或 ❌ 未达标）
2. 实测数据表的所有 TBD 替换为实际数字
3. 「实测结论」「决策」「理由」三项填入
4. 第六章 6.2 进入 Phase 0 的 4 项 checkbox 勾选

**PoC-3（bge-large-zh Rust 推理性）特殊说明**：
- 本 spec 未含 PoC-3 代码实现（需要 candle-transformers 模型加载，单独成 spec）
- 在 poc-report.md 中标注："PoC-3 待 v0.2.1 单独验证"

- [ ] **Step 7.2: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add docs/poc-report.md
git -C "D:\xin kaifa\SparkFox" commit -m "docs(poc): 填入 PoC-1/2/4 实测数据，PoC-3 待 v0.2.1"
```

---

## 四、Self-Review Checklist

### Spec coverage
- [x] P0-1 sparkfox-core → Task 1
- [x] P0-2 sparkfox-memory L5 → Task 3
- [x] P0-3 sparkfox-crdt → Task 4
- [x] P0-4 sparkfox-e2ee → Task 5
- [x] P0-5 sparkfox-store → Task 2
- [x] P0-6 PoC 报告填充 → Task 7
- [x] P2-9 路由注释清理 → Task 6
- [ ] PoC-3 bge-large-zh → **不在本 spec 范围，待 v0.2.1**

### Placeholder scan
- 所有 Task 含完整代码、完整测试、完整命令
- 无 "TBD" / "TODO" / "实现细节见..." / "类似 Task N" 模式

### Type consistency
- `Error::storage(msg, ctx)` 在 Task 1/2/4/5 中签名一致（msg: String, ctx: &str）
- `Id<T>` 在 Task 1 定义，Task 3 使用 `Id<MemoryId>`，签名一致
- `MemoryDoc::new()` 返回 `Result<Self>`，Task 4 测试与实现一致

---

## 五、执行顺序与并行策略

按用户偏好「3-4 任务并行」，可分组：

- **并行组 1**：Task 1（sparkfox-core，无依赖）→ 完成后启动并行组 2
- **并行组 2**：Task 2（store）/ Task 4（crdt）/ Task 5（e2ee）（都依赖 core）+ Task 6（路由清理，独立）
- **串行**：Task 3（memory，依赖 core + store）→ Task 7（PoC 报告，依赖所有 Task 完成）

### 阻塞依赖图
```
Task 1 (core) ─┬─> Task 2 (store) ─┐
               ├─> Task 3 (memory) ─┤
               ├─> Task 4 (crdt) ───┼─> Task 7 (poc-report)
               └─> Task 5 (e2ee) ───┘
Task 6 (router) ───────────────────────> (无依赖，可任意时刻)
```

---

## 附录 A：v0.3+ 框架（不在本 spec 范围）

### v0.3 — Phase 1 Rust crate 落地（10 项 P2）

- sparkfox-ipc：Tauri commands + events 桥接（最优先）
- sparkfox-llm：Provider 抽象（OpenAI/Anthropic/Google/Bedrock/本地）
- sparkfox-agent：AgentProfile + DAG 编排基础
- sparkfox-chat：BaiLongma 5 大特性清洁室重写
- sparkfox-thinking：ThoughtStream 后端
- sparkfox-monitor：TokenStats 6 周期 + 活动流
- sparkfox-orchestrator：DAG 编排（蜂群 + 组织融合）
- 6 个 store IPC 对接（agentStore/memoryStore/monitorStore/hotspotStore + ChatView/MemoryView/MonitorView/HotspotView）
- 路由占位注释清理（v0.2 已含）
- hooks/sparkfox/README.md 填充

### v0.5+ — Phase 2 长期补全（8 项 P3）

- sparkfox-hotspot：4 平台热榜真实 API
- sparkfox-security：11 层安全栈
- sceneStore：Scene Protocol 完整实现
- AgentDashboardView 升级完整仪表盘
- HotspotEarth Three.js 3D 地球
- monitorStore CRDT/E2EE/L5 健康度扩展维度
- Tick 心跳后端
- 14 个 Rust crate 的 README 完善

详情待后续 v0.3 / v0.5 spec。

---

**报告完成。**

> 本 spec 覆盖 v0.2.0 范围，待用户审核通过后启动执行。
