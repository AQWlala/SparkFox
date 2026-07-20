//! 抖音热榜 fetcher — v1.0.0 占位实现

use async_trait::async_trait;
use sparkfox_core::{Error, Result};

use super::{HotspotItem, Platform, PlatformFetcher};

/// 抖音热榜 fetcher
pub struct DouyinFetcher;

#[async_trait]
impl PlatformFetcher for DouyinFetcher {
    fn platform(&self) -> Platform {
        Platform::Douyin
    }

    async fn fetch_top(&self, _limit: usize) -> Result<Vec<HotspotItem>> {
        // v1.0.0 占位：无网络请求，返回 Err 说明未实现
        // v1.1.0+ 接入真实抖音热榜 API（aweme.snssdk.com/aweme/v1/hot/search/list）
        Err(Error::internal("抖音热榜 v1.1.0+ 实现，当前 v1.0.0 占位"))
    }
}
