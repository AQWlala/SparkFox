//! B 站热门 fetcher — v1.0.0 占位实现

use async_trait::async_trait;
use sparkfox_core::{Error, Result};

use super::{HotspotItem, Platform, PlatformFetcher};

/// B 站热门 fetcher
pub struct BilibiliFetcher;

#[async_trait]
impl PlatformFetcher for BilibiliFetcher {
    fn platform(&self) -> Platform {
        Platform::Bilibili
    }

    async fn fetch_top(&self, _limit: usize) -> Result<Vec<HotspotItem>> {
        // v1.0.0 占位：无网络请求，返回 Err 说明未实现
        // v1.1.0+ 接入真实 B 站热门 API（api.bilibili.com/x/web-interface/popular）
        Err(Error::internal("B站热榜 v1.1.0+ 实现，当前 v1.0.0 占位"))
    }
}
