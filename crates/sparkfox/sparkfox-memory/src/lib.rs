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
pub mod layer;
pub mod types;

pub use l5_meta::{ErrorPattern, L5MetaEngine, StrategyLog};
pub use layer::MemoryLayer;
pub use types::{MemoryEntry, MemoryKind};

// 注意：原 v1.0.0 此处 `pub use sparkfox_core::MemoryLayer;` 重导出的是 **trait**。
// v2.0 A-01 P0 修复后，`sparkfox_memory::MemoryLayer` 指向本 crate `layer::MemoryLayer` **枚举**，
// 用于 SAG 表映射 / vector_insert 表名选择。
// `sparkfox_core::MemoryLayer` trait（const LAYER + name()）仍保留，通过全限定路径引用。

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-memory v{} initialized", VERSION);
}
