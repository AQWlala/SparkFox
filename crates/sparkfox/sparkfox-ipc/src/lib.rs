//! SparkFox IPC — Tauri IPC 桥
//!
//! Tauri 2 commands + events 桥接层，连接前端 React 与后端 Rust crate。
//! 6 个 store IPC 对接：agentStore / memoryStore / monitorStore / hotspotStore /
//! sceneStore + ChatView / MemoryView / MonitorView / HotspotView。
//!
//! - [`commands`]：前端调用的 Tauri command（`#[tauri::command]`）
//! - [`events`]：后端推送到前端的事件载荷（`serde::Serialize`）

#![forbid(unsafe_code)]

pub mod commands;
pub mod events;
pub mod tick;

pub use tick::{Heartbeat, TickResult};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化 IPC 模块（日志）
pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-ipc v{} initialized", VERSION);
}
