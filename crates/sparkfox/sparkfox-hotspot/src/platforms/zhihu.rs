//! 知乎热榜 fetcher — v1.0.0 占位实现

use async_trait::async_trait;
use sparkfox_core::{Error, Result};

use super::{HotspotItem, Platform, PlatformFetcher};

/// 知乎热榜 fetcher
pub struct ZhihuFetcher;

#[async_trait]
impl PlatformFetcher for ZhihuFetcher {
    fn platform(&self) -> Platform {
        Platform::Zhihu
    }

    async fn fetch_top(&self, _limit: usize) -> Result<Vec<HotspotItem>> {
        // v1.0.0 占位：无网络请求，返回 Err 说明未实现
        // v1.1.0+ 接入真实知乎热榜 API（zhihu.com/api/v3/feed/topstory/hot-lists/total）
        Err(Error::internal("知乎热榜 v1.1.0+ 实现，当前 v1.0.0 占位"))
    }
}
