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
