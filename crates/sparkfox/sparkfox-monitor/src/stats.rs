//! TokenStats — 6 周期用量统计
//!
//! 参考 OpenAkita TokenStatsView 思路（清洁室重写，未拷贝源代码）。
//!
//! 6 个统计周期：
//! - `Minute`：最近 1 分钟
//! - `Hour`：最近 1 小时
//! - `Day`：最近 24 小时
//! - `Week`：最近 7 天
//! - `Month`：最近 30 天
//! - `AllTime`：全部历史
//!
//! 5 个统计维度：`input_tokens` / `output_tokens` / `total_cost`（USD）/
//! `request_count` / 按模型分组。

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 6 个统计周期
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatsPeriod {
    /// 最近 1 分钟
    Minute,
    /// 最近 1 小时
    Hour,
    /// 最近 24 小时
    Day,
    /// 最近 7 天
    Week,
    /// 最近 30 天
    Month,
    /// 全部
    AllTime,
}

impl StatsPeriod {
    /// 返回该周期对应的时间窗长度（秒）。
    ///
    /// `AllTime` 返回 `None` 表示无时间窗限制。
    pub fn window_secs(&self) -> Option<i64> {
        match self {
            StatsPeriod::Minute => Some(60),
            StatsPeriod::Hour => Some(3_600),
            StatsPeriod::Day => Some(86_400),
            StatsPeriod::Week => Some(7 * 86_400),
            StatsPeriod::Month => Some(30 * 86_400),
            StatsPeriod::AllTime => None,
        }
    }

    /// 返回该周期的截止时间（now - window）。
    ///
    /// `AllTime` 返回 `None` 表示无时间窗限制，统计时不应过滤。
    fn cutoff(&self, now: chrono::DateTime<chrono::Utc>) -> Option<chrono::DateTime<chrono::Utc>> {
        self.window_secs()
            .map(|secs| now - chrono::Duration::seconds(secs))
    }
}

/// 单次统计结果
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TokenStats {
    /// 输入 token 数
    pub input_tokens: u64,
    /// 输出 token 数
    pub output_tokens: u64,
    /// 总成本（USD）
    pub total_cost: f64,
    /// 请求数
    pub request_count: u64,
}

impl TokenStats {
    /// 累加一条记录
    fn add_record(&mut self, r: &TokenRecord) {
        self.input_tokens += r.input_tokens;
        self.output_tokens += r.output_tokens;
        self.total_cost += r.cost;
        self.request_count += 1;
    }

    /// 合并另一份统计
    pub fn merge(&mut self, other: &TokenStats) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_cost += other.total_cost;
        self.request_count += other.request_count;
    }

    /// 总 token 数（输入 + 输出）
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// 单次 API 调用记录
struct TokenRecord {
    timestamp: chrono::DateTime<chrono::Utc>,
    input_tokens: u64,
    output_tokens: u64,
    cost: f64,
    model: String,
}

/// TokenStats 收集器：累积所有调用记录，按需查询任意周期/模型的统计
pub struct TokenStatsCollector {
    records: Vec<TokenRecord>,
}

impl Default for TokenStatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenStatsCollector {
    /// 创建一个空的收集器
    pub fn new() -> Self {
        Self { records: Vec::new() }
    }

    /// 记录一次 API 调用
    ///
    /// - `input`：输入 token 数
    /// - `output`：输出 token 数
    /// - `cost`：本次调用成本（USD）
    /// - `model`：模型名（如 `gpt-4o-mini`）
    pub fn record(&mut self, input: u64, output: u64, cost: f64, model: String) {
        self.records.push(TokenRecord {
            timestamp: chrono::Utc::now(),
            input_tokens: input,
            output_tokens: output,
            cost,
            model,
        });
    }

    /// 记录一次 API 调用（带自定义时间戳，便于测试与历史回放）
    pub fn record_at(
        &mut self,
        timestamp: chrono::DateTime<chrono::Utc>,
        input: u64,
        output: u64,
        cost: f64,
        model: String,
    ) {
        self.records.push(TokenRecord {
            timestamp,
            input_tokens: input,
            output_tokens: output,
            cost,
            model,
        });
    }

    /// 查询指定周期的汇总统计
    pub fn stats(&self, period: StatsPeriod) -> TokenStats {
        let now = chrono::Utc::now();
        let cutoff = period.cutoff(now);
        let mut stats = TokenStats::default();
        for r in &self.records {
            let in_window = cutoff.map_or(true, |c| r.timestamp >= c);
            if in_window {
                stats.add_record(r);
            }
        }
        stats
    }

    /// 查询指定周期、按模型分组的统计
    pub fn stats_by_model(&self, period: StatsPeriod) -> HashMap<String, TokenStats> {
        let now = chrono::Utc::now();
        let cutoff = period.cutoff(now);
        let mut by_model: HashMap<String, TokenStats> = HashMap::new();
        for r in &self.records {
            let in_window = cutoff.map_or(true, |c| r.timestamp >= c);
            if in_window {
                let entry = by_model.entry(r.model.clone()).or_default();
                entry.add_record(r);
            }
        }
        by_model
    }

    /// 总记录数
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// 是否没有任何记录
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// 清空所有记录
    pub fn clear(&mut self) {
        self.records.clear();
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }

    #[test]
    fn stats_period_window_secs_matches_spec() {
        // 校验 6 个周期的窗口长度符合 spec
        assert_eq!(StatsPeriod::Minute.window_secs(), Some(60));
        assert_eq!(StatsPeriod::Hour.window_secs(), Some(3_600));
        assert_eq!(StatsPeriod::Day.window_secs(), Some(86_400));
        assert_eq!(StatsPeriod::Week.window_secs(), Some(7 * 86_400));
        assert_eq!(StatsPeriod::Month.window_secs(), Some(30 * 86_400));
        assert_eq!(StatsPeriod::AllTime.window_secs(), None);
    }

    #[test]
    fn record_and_stats_all_time() {
        // 校验全周期统计：所有记录都应被计入
        let mut collector = TokenStatsCollector::new();
        collector.record(100, 50, 0.012, "gpt-4o-mini".into());
        collector.record(200, 100, 0.024, "gpt-4o-mini".into());
        collector.record(300, 150, 0.05, "gpt-4o".into());

        let stats = collector.stats(StatsPeriod::AllTime);
        assert_eq!(stats.input_tokens, 600);
        assert_eq!(stats.output_tokens, 300);
        assert_eq!(stats.total_tokens(), 900);
        assert_eq!(stats.request_count, 3);
        assert!((stats.total_cost - 0.086).abs() < 1e-9);
    }

    #[test]
    fn stats_by_period_filters_old_records() {
        // 校验旧记录被正确过滤
        let mut collector = TokenStatsCollector::new();
        let now = now();

        // 10 秒前 — 应被 Minute 周期包含
        collector.record_at(now - Duration::seconds(10), 100, 50, 0.01, "m1".into());
        // 2 分钟前 — 应被 Hour 周期包含，但被 Minute 排除
        collector.record_at(now - Duration::seconds(120), 200, 100, 0.02, "m1".into());
        // 2 小时前 — 应被 Day 周期包含，但被 Hour 排除
        collector.record_at(now - Duration::hours(2), 300, 150, 0.03, "m1".into());
        // 2 天前 — 应被 Week 周期包含，但被 Day 排除
        collector.record_at(now - Duration::days(2), 400, 200, 0.04, "m1".into());
        // 8 天前 — 应被 Month 周期包含，但被 Week 排除
        collector.record_at(now - Duration::days(8), 500, 250, 0.05, "m1".into());
        // 35 天前 — 应被 AllTime 包含，但被 Month 排除
        collector.record_at(now - Duration::days(35), 600, 300, 0.06, "m1".into());

        let minute = collector.stats(StatsPeriod::Minute);
        assert_eq!(minute.request_count, 1);
        assert_eq!(minute.input_tokens, 100);

        let hour = collector.stats(StatsPeriod::Hour);
        assert_eq!(hour.request_count, 2);
        assert_eq!(hour.input_tokens, 300);

        let day = collector.stats(StatsPeriod::Day);
        assert_eq!(day.request_count, 3);
        assert_eq!(day.input_tokens, 600);

        let week = collector.stats(StatsPeriod::Week);
        assert_eq!(week.request_count, 4);
        assert_eq!(week.input_tokens, 1_000);

        let month = collector.stats(StatsPeriod::Month);
        assert_eq!(month.request_count, 5);
        assert_eq!(month.input_tokens, 1_500);

        let all = collector.stats(StatsPeriod::AllTime);
        assert_eq!(all.request_count, 6);
        assert_eq!(all.input_tokens, 2_100);
    }

    #[test]
    fn stats_by_model_groups_correctly() {
        // 校验按模型分组
        let mut collector = TokenStatsCollector::new();
        collector.record(100, 50, 0.01, "gpt-4o-mini".into());
        collector.record(200, 100, 0.02, "gpt-4o".into());
        collector.record(150, 75, 0.015, "gpt-4o-mini".into());
        collector.record(300, 150, 0.03, "claude-3.5".into());

        let by_model = collector.stats_by_model(StatsPeriod::AllTime);
        assert_eq!(by_model.len(), 3);

        let mini = by_model.get("gpt-4o-mini").expect("应有 gpt-4o-mini");
        assert_eq!(mini.request_count, 2);
        assert_eq!(mini.input_tokens, 250);
        assert_eq!(mini.output_tokens, 125);

        let gpt4o = by_model.get("gpt-4o").expect("应有 gpt-4o");
        assert_eq!(gpt4o.request_count, 1);
        assert_eq!(gpt4o.input_tokens, 200);

        let claude = by_model.get("claude-3.5").expect("应有 claude-3.5");
        assert_eq!(claude.request_count, 1);
        assert_eq!(claude.input_tokens, 300);
    }

    #[test]
    fn stats_by_model_period_filter() {
        // 校验 stats_by_model 同样应用周期过滤
        let mut collector = TokenStatsCollector::new();
        let now = now();

        collector.record_at(now - Duration::seconds(30), 100, 50, 0.01, "m1".into());
        collector.record_at(now - Duration::days(5), 200, 100, 0.02, "m1".into());

        let by_model_minute = collector.stats_by_model(StatsPeriod::Minute);
        let mini_minute = by_model_minute.get("m1").expect("m1 应存在");
        assert_eq!(mini_minute.request_count, 1);
        assert_eq!(mini_minute.input_tokens, 100);

        let by_model_week = collector.stats_by_model(StatsPeriod::Week);
        let mini_week = by_model_week.get("m1").expect("m1 应存在");
        assert_eq!(mini_week.request_count, 2);
        assert_eq!(mini_week.input_tokens, 300);
    }

    #[test]
    fn empty_collector_returns_zero_stats() {
        let collector = TokenStatsCollector::new();
        assert!(collector.is_empty());
        assert_eq!(collector.len(), 0);

        let stats = collector.stats(StatsPeriod::AllTime);
        assert_eq!(stats, TokenStats::default());

        let by_model = collector.stats_by_model(StatsPeriod::AllTime);
        assert!(by_model.is_empty());
    }

    #[test]
    fn clear_drops_all_records() {
        let mut collector = TokenStatsCollector::new();
        collector.record(100, 50, 0.01, "m1".into());
        collector.record(200, 100, 0.02, "m1".into());
        assert_eq!(collector.len(), 2);

        collector.clear();
        assert!(collector.is_empty());
        assert_eq!(collector.stats(StatsPeriod::AllTime).request_count, 0);
    }

    #[test]
    fn token_stats_merge_works() {
        // 校验 TokenStats::merge
        let mut a = TokenStats {
            input_tokens: 100,
            output_tokens: 50,
            total_cost: 0.01,
            request_count: 1,
        };
        let b = TokenStats {
            input_tokens: 200,
            output_tokens: 100,
            total_cost: 0.02,
            request_count: 2,
        };
        a.merge(&b);
        assert_eq!(a.input_tokens, 300);
        assert_eq!(a.output_tokens, 150);
        assert!((a.total_cost - 0.03).abs() < 1e-9);
        assert_eq!(a.request_count, 3);
    }

    #[test]
    fn stats_period_serde_roundtrip() {
        for p in [
            StatsPeriod::Minute,
            StatsPeriod::Hour,
            StatsPeriod::Day,
            StatsPeriod::Week,
            StatsPeriod::Month,
            StatsPeriod::AllTime,
        ] {
            let json = serde_json::to_string(&p).expect("序列化失败");
            let decoded: StatsPeriod = serde_json::from_str(&json).expect("反序列化失败");
            assert_eq!(p, decoded);
        }
    }

    #[test]
    fn token_stats_serde_roundtrip() {
        let s = TokenStats {
            input_tokens: 123,
            output_tokens: 456,
            total_cost: 0.789,
            request_count: 42,
        };
        let json = serde_json::to_string(&s).expect("序列化失败");
        let decoded: TokenStats = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(s, decoded);
    }
}
