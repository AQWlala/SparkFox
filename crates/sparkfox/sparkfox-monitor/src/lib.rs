//! SparkFox Monitor — 监视数据（TokenStats 6 周期 + 活动流）
//!
//! 参考 OpenAkita TokenStatsView 思路（清洁室重写，未拷贝源代码）。
//!
//! 两个核心子系统：
//! - [`stats`]：Token 用量统计，支持 6 个时间周期（Minute/Hour/Day/Week/Month/AllTime）
//!   与按模型分组。
//! - [`activity`]：Agent 活动事件流（broadcast + 有限历史），支持实时订阅与历史查询。

#![forbid(unsafe_code)]

pub mod activity;
pub mod stats;

pub use activity::{Activity, ActivityStream, ActivityType};
pub use stats::{StatsPeriod, TokenStats, TokenStatsCollector};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
