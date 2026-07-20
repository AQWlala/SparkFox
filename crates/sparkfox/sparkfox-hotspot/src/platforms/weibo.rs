//! 微博热搜 fetcher — v1.0.0 占位实现

use async_trait::async_trait;
use sparkfox_core::{Error, Result};

use super::{HotspotItem, Platform, PlatformFetcher};

/// 微博热搜 fetcher
pub struct WeiboFetcher;

#[async_trait]
impl PlatformFetcher for WeiboFetcher {
    fn platform(&self) -> Platform {
        Platform::Weibo
    }

    async fn fetch_top(&self, _limit: usize) -> Result<Vec<HotspotItem>> {
        // v1.0.0 占位：无网络请求，返回 Err 说明未实现
        // v1.1.0+ 接入真实微博热搜 API（s.weibo.com/top/summary）
        Err(Error::internal("微博热榜 v1.1.0+ 实现，当前 v1.0.0 占位"))
    }
}
