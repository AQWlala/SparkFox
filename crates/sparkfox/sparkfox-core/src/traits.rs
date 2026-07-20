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
