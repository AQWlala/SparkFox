//! 4 平台热榜数据契约 — 平台枚举、热榜条目结构、PlatformFetcher trait。
//!
//! v1.0.0 仅定义契约 + 占位 fetcher（返回 `Err`），不发起 HTTP 请求。
//! v1.1.0+ 接入真实平台 API（微博热搜 / 知乎热榜 / 抖音热榜 / B 站热门）。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use sparkfox_core::Result;

/// 热榜条目（跨平台统一结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotItem {
    /// 平台内唯一 ID（微博 mid / 知乎问题 ID / 抖音 item_id / B 站 aid 或 bvid）
    pub id: String,
    /// 标题
    pub title: String,
    /// 详情页 URL
    pub url: String,
    /// 热度数值（平台原始热度，跨平台不可比）
    pub heat: u64,
    /// 排名（从 1 开始）
    pub rank: u32,
    /// 来源平台
    pub source: HotspotSource,
    /// 封面图 URL（可选）
    pub cover_url: Option<String>,
    /// 发布时间（UTC，可选；部分平台不返回）
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 热榜来源平台（序列化透传给前端）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotspotSource {
    Weibo,
    Zhihu,
    Douyin,
    Bilibili,
}

impl HotspotSource {
    /// 中文展示名
    pub fn name(&self) -> &'static str {
        match self {
            Self::Weibo => "微博",
            Self::Zhihu => "知乎",
            Self::Douyin => "抖音",
            Self::Bilibili => "B站",
        }
    }
}

/// 平台枚举（用于 fetcher 路由）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Weibo,
    Zhihu,
    Douyin,
    Bilibili,
}

impl Platform {
    /// 转换为 HotspotSource
    pub fn source(&self) -> HotspotSource {
        match self {
            Self::Weibo => HotspotSource::Weibo,
            Self::Zhihu => HotspotSource::Zhihu,
            Self::Douyin => HotspotSource::Douyin,
            Self::Bilibili => HotspotSource::Bilibili,
        }
    }

    /// 平台中文名（委托 [`HotspotSource::name`]）
    pub fn name(&self) -> &'static str {
        self.source().name()
    }
}

/// 平台热榜 fetcher trait
///
/// v1.0.0：所有实现为占位（返回 `Err`），v1.1.0+ 接入真实 API。
#[async_trait]
pub trait PlatformFetcher: Send + Sync {
    /// 平台标识
    fn platform(&self) -> Platform;
    /// 拉取热榜前 `limit` 条（`limit == 0` 由实现决定，通常返回全部）
    async fn fetch_top(&self, limit: usize) -> Result<Vec<HotspotItem>>;
}

pub mod bilibili;
pub mod douyin;
pub mod weibo;
pub mod zhihu;

/// 返回 4 平台 fetcher 实例（每个平台一个 Box）
pub fn all_platforms() -> Vec<Box<dyn PlatformFetcher>> {
    vec![
        Box::new(weibo::WeiboFetcher),
        Box::new(zhihu::ZhihuFetcher),
        Box::new(douyin::DouyinFetcher),
        Box::new(bilibili::BilibiliFetcher),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_name_chinese() {
        assert_eq!(HotspotSource::Weibo.name(), "微博");
        assert_eq!(HotspotSource::Zhihu.name(), "知乎");
        assert_eq!(HotspotSource::Douyin.name(), "抖音");
        assert_eq!(HotspotSource::Bilibili.name(), "B站");
    }

    #[test]
    fn platform_to_source_roundtrip() {
        assert_eq!(Platform::Weibo.source(), HotspotSource::Weibo);
        assert_eq!(Platform::Zhihu.source(), HotspotSource::Zhihu);
        assert_eq!(Platform::Douyin.source(), HotspotSource::Douyin);
        assert_eq!(Platform::Bilibili.source(), HotspotSource::Bilibili);
    }

    #[test]
    fn platform_name_matches_source_name() {
        assert_eq!(Platform::Weibo.name(), HotspotSource::Weibo.name());
        assert_eq!(Platform::Zhihu.name(), HotspotSource::Zhihu.name());
        assert_eq!(Platform::Douyin.name(), HotspotSource::Douyin.name());
        assert_eq!(Platform::Bilibili.name(), HotspotSource::Bilibili.name());
    }

    #[test]
    fn all_platforms_returns_four_fetchers() {
        let fetchers = all_platforms();
        assert_eq!(fetchers.len(), 4, "all_platforms 必须返回 4 个 fetcher");
        let platforms: Vec<Platform> = fetchers.iter().map(|f| f.platform()).collect();
        assert!(platforms.contains(&Platform::Weibo), "缺少微博");
        assert!(platforms.contains(&Platform::Zhihu), "缺少知乎");
        assert!(platforms.contains(&Platform::Douyin), "缺少抖音");
        assert!(platforms.contains(&Platform::Bilibili), "缺少B站");
    }

    #[test]
    fn all_platforms_no_duplicates() {
        let fetchers = all_platforms();
        let platforms: Vec<Platform> = fetchers.iter().map(|f| f.platform()).collect();
        // 检查 4 个平台各出现一次（O(n²) 但 n=4，简单可靠，无需为 Platform 加 Ord/Hash derive）
        for i in 0..platforms.len() {
            for j in (i + 1)..platforms.len() {
                assert_ne!(platforms[i], platforms[j], "平台重复: {:?}", platforms[i]);
            }
        }
    }
}
