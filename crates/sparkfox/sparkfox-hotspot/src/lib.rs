//! SparkFox Hotspot — 4 平台热榜（微博/知乎/抖音/B站）
//!
//! NOTICE: NomiFun 热点设计借鉴，清洁室重写。
//!
//! v1.0.0：仅定义数据契约 + 4 平台占位 fetcher（返回 `Err`，不发起 HTTP 请求）。
//! v1.1.0+：接入真实平台热搜 API。

#![forbid(unsafe_code)]

pub mod platforms;

pub use platforms::{HotspotItem, HotspotSource, Platform, PlatformFetcher};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化日志（幂等）
pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-hotspot v{} initialized", VERSION);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_non_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn init_is_idempotent() {
        // 多次调用不应 panic
        init();
        init();
    }
}
