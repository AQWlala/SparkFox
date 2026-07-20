//! Tick 心跳后端 — 监控存活 + 自动重连
//!
//! Task 8.7：定期检查 sparkfox-store/llm/embedding 等组件存活；触发 Tauri
//! event `tick` 推送到前端；失败时记录日志，连续 `max_failures` 次失败后调用
//! `on_failure` 回调（用于触发重连）。
//!
//! # 设计
//! - `Heartbeat` 通过 `tokio::time::interval` 周期性触发回调
//! - 回调返回 `Ok(())` 视为存活，重置失败计数；返回 `Err` 视为一次失败
//! - 失败计数达到 `max_failures` 后调用 `on_failure` 并重置计数（避免连续触发）
//! - 共享状态用 `tokio::sync::Mutex`（异步可感知），便于跨任务查询 `last_beat`/`failure_count`
//!
//! # 使用
//! ```no_run
//! # use std::sync::Arc;
//! # use sparkfox_ipc::Heartbeat;
//! # async fn demo() {
//! let hb = Arc::new(Heartbeat::new(30));
//! let hb_clone = Arc::clone(&hb);
//! tokio::spawn(async move {
//!     hb_clone.run(
//!         || Ok::<(), std::io::Error>(()),
//!         || log::warn!("触发重连"),
//!     ).await;
//! });
//! # }
//! ```

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio::time::interval;

/// 心跳后端 — 周期性检查组件存活，失败 N 次触发重连回调
///
/// 共享状态（`last_beat` / `failure_count`）放在 `Arc<Mutex<_>>` 中，
/// 便于从其他任务查询最近一次心跳时间和当前失败计数。
pub struct Heartbeat {
    /// 心跳间隔（秒）
    interval_secs: u64,
    /// 最近一次成功心跳时间
    last_beat: Arc<Mutex<Instant>>,
    /// 当前连续失败计数
    failure_count: Arc<Mutex<u32>>,
    /// 触发重连的失败阈值
    max_failures: u32,
}

impl Heartbeat {
    /// 创建心跳实例，默认 `max_failures = 3`
    pub fn new(interval_secs: u64) -> Self {
        Self {
            interval_secs,
            last_beat: Arc::new(Mutex::new(Instant::now())),
            failure_count: Arc::new(Mutex::new(0)),
            max_failures: 3,
        }
    }

    /// 链式设置失败阈值
    #[must_use]
    pub fn with_max_failures(mut self, max: u32) -> Self {
        self.max_failures = max;
        self
    }

    /// 启动心跳循环（永不返回，需在 `tokio::spawn` 中调用）
    ///
    /// - `callback`：每次 tick 调用，返回 `Ok(())` 表示心跳成功，`Err` 表示失败
    /// - `on_failure`：连续 `max_failures` 次失败后触发（用于重连），触发后失败计数归零
    pub async fn run<F, E, G>(&self, callback: F, on_failure: G)
    where
        F: Fn() -> std::result::Result<(), E> + Send + Sync + 'static,
        E: std::fmt::Debug + Send + 'static,
        G: Fn() + Send + Sync + 'static,
    {
        let mut tick = interval(Duration::from_secs(self.interval_secs));
        loop {
            tick.tick().await;
            match callback() {
                Ok(()) => {
                    let mut last = self.last_beat.lock().await;
                    *last = Instant::now();
                    let mut count = self.failure_count.lock().await;
                    *count = 0;
                    log::debug!("心跳成功");
                }
                Err(e) => {
                    let mut count = self.failure_count.lock().await;
                    *count += 1;
                    log::warn!(
                        "心跳失败 ({}/{}): {:?}",
                        *count,
                        self.max_failures,
                        e
                    );
                    if *count >= self.max_failures {
                        log::error!("心跳连续失败 {} 次，触发重连", *count);
                        on_failure();
                        *count = 0;
                    }
                }
            }
        }
    }

    /// 最近一次心跳成功时间
    pub async fn last_beat(&self) -> Instant {
        *self.last_beat.lock().await
    }

    /// 当前连续失败次数
    pub async fn failure_count(&self) -> u32 {
        *self.failure_count.lock().await
    }
}

/// 单次心跳结果 — 通过 Tauri event `tick` 推送到前端
#[derive(Debug, Clone, serde::Serialize)]
pub struct TickResult {
    /// 心跳时间戳（UTC）
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 是否存活
    pub alive: bool,
    /// 心跳延迟（毫秒），失败时为 0
    pub latency_ms: u64,
    /// 被检查的组件名（store / llm / embedding / ...）
    pub component: String,
}

impl TickResult {
    /// 构造成功心跳结果
    #[must_use]
    pub fn ok(component: &str, latency_ms: u64) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            alive: true,
            latency_ms,
            component: component.to_string(),
        }
    }

    /// 构造失败心跳结果
    #[must_use]
    pub fn failed(component: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            alive: false,
            latency_ms: 0,
            component: component.to_string(),
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration as StdDuration;

    /// `with_max_failures` 应正确设置阈值
    #[test]
    fn test_with_max_failures() {
        let hb = Heartbeat::new(60).with_max_failures(5);
        assert_eq!(hb.max_failures, 5);
    }

    /// `new` 默认 `max_failures = 3`
    #[test]
    fn test_default_max_failures() {
        let hb = Heartbeat::new(60);
        assert_eq!(hb.max_failures, 3);
    }

    /// `last_beat` 初始值为构造时刻
    #[tokio::test]
    async fn test_last_beat_initial() {
        // 注意：`before` 必须先于 `Heartbeat::new` 捕获，因为 new 内部调用 `Instant::now()`
        let before = Instant::now();
        let hb = Heartbeat::new(60);
        let last = hb.last_beat().await;
        assert!(last >= before);
    }

    /// `failure_count` 初始为 0
    #[tokio::test]
    async fn test_failure_count_initial() {
        let hb = Heartbeat::new(60);
        assert_eq!(hb.failure_count().await, 0);
    }

    /// `TickResult::ok` 应构造存活结果
    #[test]
    fn test_tick_result_ok() {
        let r = TickResult::ok("store", 12);
        assert!(r.alive);
        assert_eq!(r.latency_ms, 12);
        assert_eq!(r.component, "store");
    }

    /// `TickResult::failed` 应构造失败结果
    #[test]
    fn test_tick_result_failed() {
        let r = TickResult::failed("llm");
        assert!(!r.alive);
        assert_eq!(r.latency_ms, 0);
        assert_eq!(r.component, "llm");
    }

    /// `TickResult` 应可序列化
    #[test]
    fn test_tick_result_serialize() {
        let r = TickResult::ok("embedding", 42);
        let json = serde_json::to_string(&r).expect("序列化失败");
        assert!(json.contains("\"alive\":true"));
        assert!(json.contains("\"component\":\"embedding\""));
        assert!(json.contains("\"latency_ms\":42"));
    }

    /// 心跳成功时应重置失败计数并更新 last_beat
    ///
    /// 由于 `run` 是无限循环，这里用短间隔 + 有限次数的方式验证：
    /// 通过 `tokio::time::timeout` 让循环跑几次后超时退出。
    #[tokio::test]
    async fn test_run_success_resets_count() {
        let hb = Arc::new(Heartbeat::new(1).with_max_failures(3));
        let hb_clone = Arc::clone(&hb);

        // 先把失败计数手动设为 2，验证成功后会归零
        {
            let mut c = hb.failure_count.lock().await;
            *c = 2;
        }

        let handle = tokio::spawn(async move {
            hb_clone.run(|| Ok::<(), std::io::Error>(()), || {}).await;
        });

        // 等待至少一次 tick（间隔 1s，等 1.5s 保证触发）
        tokio::time::sleep(StdDuration::from_millis(1500)).await;

        // 验证失败计数已归零
        let count = hb.failure_count().await;
        assert_eq!(count, 0, "成功后失败计数应归零");

        // last_beat 应被更新
        let last = hb.last_beat().await;
        let now = Instant::now();
        assert!(now.duration_since(last) <= StdDuration::from_secs(2));

        // 关闭后台任务
        handle.abort();
    }

    /// 连续失败达到阈值时应触发 on_failure 并重置计数
    #[tokio::test]
    async fn test_run_failure_triggers_reconnect() {
        let hb = Arc::new(Heartbeat::new(1).with_max_failures(2));
        let hb_clone = Arc::clone(&hb);

        let trigger_count = Arc::new(AtomicU32::new(0));
        let trigger_clone = Arc::clone(&trigger_count);
        let on_failure = move || {
            trigger_clone.fetch_add(1, Ordering::SeqCst);
        };

        let handle = tokio::spawn(async move {
            hb_clone.run(
                || Err::<(), std::io::Error>(std::io::Error::other("boom")),
                on_failure,
            )
            .await;
        });

        // 间隔 1s，max_failures=2，等 3s 保证至少触发一次重连
        tokio::time::sleep(StdDuration::from_millis(3000)).await;

        let triggers = trigger_count.load(Ordering::SeqCst);
        assert!(triggers >= 1, "应至少触发一次重连回调，实际 {}", triggers);

        // 触发后失败计数应被重置（可能已经开始下一轮，但不会 >= max_failures）
        let count = hb.failure_count().await;
        assert!(
            count < 2,
            "重连触发后失败计数应小于阈值，实际 {}",
            count
        );

        handle.abort();
    }
}
