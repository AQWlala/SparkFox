//! Activity — 活动流（Agent 行为日志的实时广播 + 历史查询）
//!
//! 设计要点：
//! - `tokio::sync::broadcast` 用于实时推送（前端订阅）
//! - 内部维护一个容量有限的环形历史缓冲（`history_size`），便于新订阅者
//!   通过 `recent(n)` / `filter(...)` 拉取最近事件
//! - 当历史满时，按时间顺序丢弃最旧的一条

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// 单条活动事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// 事件唯一 ID（前端可用作 React key）
    pub id: String,
    /// 关联的 Agent ID
    pub agent_id: String,
    /// 活动类型
    pub activity_type: ActivityType,
    /// 人类可读消息
    pub message: String,
    /// 事件时间戳（UTC）
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 任意附加元数据（JSON）
    pub metadata: Option<serde_json::Value>,
}

/// 活动类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityType {
    /// Agent 启动
    AgentStarted,
    /// Agent 停止
    AgentStopped,
    /// 任务完成
    TaskCompleted,
    /// 任务失败
    TaskFailed,
    /// 记忆操作（写入/查询/更新）
    MemoryOp,
    /// 检索操作（RAG / 向量检索）
    RetrievalOp,
    /// 错误事件
    Error,
}

impl Activity {
    /// 用当前时间戳构造一条新活动
    pub fn new(
        id: impl Into<String>,
        agent_id: impl Into<String>,
        activity_type: ActivityType,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            agent_id: agent_id.into(),
            activity_type,
            message: message.into(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        }
    }

    /// 设置 metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// 活动流：broadcast + 有限历史
pub struct ActivityStream {
    sender: broadcast::Sender<Activity>,
    history: Vec<Activity>,
    history_size: usize,
    channel_capacity: usize,
}

impl ActivityStream {
    /// 创建一个活动流
    ///
    /// - `buffer`：broadcast 通道容量（实时订阅者队列长度）
    /// - `history_size`：保留的最近历史事件条数（0 表示不保留）
    pub fn new(buffer: usize, history_size: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer);
        Self {
            sender,
            history: Vec::with_capacity(history_size),
            history_size,
            channel_capacity: buffer,
        }
    }

    /// 订阅实时活动流
    pub fn subscribe(&self) -> broadcast::Receiver<Activity> {
        self.sender.subscribe()
    }

    /// 发布一条活动：广播给所有订阅者，并写入历史
    pub fn publish(&mut self, activity: Activity) {
        let _ = self.sender.send(activity.clone());
        if self.history_size == 0 {
            return;
        }
        if self.history.len() >= self.history_size {
            // 容量已满：丢弃最旧的一条
            self.history.remove(0);
        }
        self.history.push(activity);
    }

    /// 返回最近 `n` 条历史活动（按时间正序，最旧在前）
    pub fn recent(&self, n: usize) -> Vec<Activity> {
        if n == 0 || self.history.is_empty() {
            return Vec::new();
        }
        let start = self.history.len().saturating_sub(n);
        self.history[start..].to_vec()
    }

    /// 按活动类型过滤历史活动
    pub fn filter(&self, activity_type: ActivityType) -> Vec<Activity> {
        self.history
            .iter()
            .filter(|a| a.activity_type == activity_type)
            .cloned()
            .collect()
    }

    /// 当前历史长度
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// 历史容量上限
    pub fn history_capacity(&self) -> usize {
        self.history_size
    }

    /// 当前实时订阅者数量
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// broadcast 通道容量
    pub fn channel_capacity(&self) -> usize {
        self.channel_capacity
    }

    /// 清空历史（不影响订阅者）
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn publish_and_subscribe_receives_activity() {
        // 验证 publish + subscribe 基本流程
        let mut stream = ActivityStream::new(16, 100);
        let mut rx = stream.subscribe();

        let activity = Activity::new("a-1", "agent-1", ActivityType::AgentStarted, "Agent 启动");
        stream.publish(activity.clone());

        let received = rx.recv().await.expect("应当收到活动");
        assert_eq!(received.id, "a-1");
        assert_eq!(received.agent_id, "agent-1");
        assert_eq!(received.activity_type, ActivityType::AgentStarted);
        assert_eq!(received.message, "Agent 启动");
        assert!(received.metadata.is_none());
    }

    #[test]
    fn recent_returns_last_n_in_order() {
        // 验证 recent(n) 按时间正序返回最近 n 条
        let mut stream = ActivityStream::new(16, 100);
        for i in 0..5 {
            stream.publish(Activity::new(
                format!("a-{i}"),
                "agent-1",
                ActivityType::TaskCompleted,
                format!("任务 {i}"),
            ));
        }

        let last3 = stream.recent(3);
        assert_eq!(last3.len(), 3);
        assert_eq!(last3[0].id, "a-2");
        assert_eq!(last3[1].id, "a-3");
        assert_eq!(last3[2].id, "a-4");

        // n 大于历史长度时返回全部
        let all = stream.recent(100);
        assert_eq!(all.len(), 5);
        assert_eq!(all[0].id, "a-0");

        // n = 0 返回空
        assert!(stream.recent(0).is_empty());
    }

    #[test]
    fn filter_by_activity_type() {
        // 验证按类型过滤
        let mut stream = ActivityStream::new(16, 100);
        stream.publish(Activity::new(
            "a-1",
            "agent-1",
            ActivityType::AgentStarted,
            "启动",
        ));
        stream.publish(Activity::new(
            "a-2",
            "agent-1",
            ActivityType::TaskCompleted,
            "完成",
        ));
        stream.publish(Activity::new(
            "a-3",
            "agent-1",
            ActivityType::TaskFailed,
            "失败",
        ));
        stream.publish(Activity::new(
            "a-4",
            "agent-1",
            ActivityType::TaskCompleted,
            "完成2",
        ));
        stream.publish(Activity::new(
            "a-5",
            "agent-1",
            ActivityType::Error,
            "错误",
        ));

        let completed = stream.filter(ActivityType::TaskCompleted);
        assert_eq!(completed.len(), 2);
        assert_eq!(completed[0].id, "a-2");
        assert_eq!(completed[1].id, "a-4");

        let started = stream.filter(ActivityType::AgentStarted);
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].id, "a-1");

        let stopped = stream.filter(ActivityType::AgentStopped);
        assert!(stopped.is_empty());
    }

    #[test]
    fn history_capacity_limit_drops_oldest() {
        // 验证 history 容量限制：超出时丢弃最旧
        let mut stream = ActivityStream::new(16, 3);
        for i in 0..5 {
            stream.publish(Activity::new(
                format!("a-{i}"),
                "agent-1",
                ActivityType::MemoryOp,
                format!("op {i}"),
            ));
        }
        assert_eq!(stream.history_len(), 3);
        // 应保留 a-2、a-3、a-4
        let recent = stream.recent(10);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].id, "a-2");
        assert_eq!(recent[2].id, "a-4");
    }

    #[test]
    fn history_size_zero_means_no_history() {
        // history_size = 0 时不保留历史，但 publish 仍能广播
        let mut stream = ActivityStream::new(16, 0);
        let mut rx = stream.subscribe();
        stream.publish(Activity::new(
            "a-1",
            "agent-1",
            ActivityType::AgentStarted,
            "hi",
        ));

        assert_eq!(stream.history_len(), 0);
        assert!(stream.recent(10).is_empty());

        // 但订阅者仍能收到
        let received = futures_block_on(rx.recv());
        assert_eq!(received.unwrap().id, "a-1");
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive() {
        // 验证多个订阅者同时接收
        let mut stream = ActivityStream::new(16, 100);
        let mut rx1 = stream.subscribe();
        let mut rx2 = stream.subscribe();

        stream.publish(Activity::new(
            "a-1",
            "agent-1",
            ActivityType::AgentStarted,
            "启动",
        ));

        for rx in [&mut rx1, &mut rx2] {
            let received = rx.recv().await.expect("每个订阅者都应收到");
            assert_eq!(received.id, "a-1");
        }
    }

    #[test]
    fn activity_with_metadata_builder() {
        let a = Activity::new(
            "a-1",
            "agent-1",
            ActivityType::MemoryOp,
            "write",
        )
        .with_metadata(json!({ "layer": "L5", "tokens": 128 }));

        assert_eq!(a.id, "a-1");
        let meta = a.metadata.as_ref().expect("应有 metadata");
        assert_eq!(meta["layer"], "L5");
        assert_eq!(meta["tokens"], 128);
    }

    #[test]
    fn activity_serde_roundtrip() {
        let a = Activity {
            id: "a-1".into(),
            agent_id: "agent-1".into(),
            activity_type: ActivityType::TaskCompleted,
            message: "完成".into(),
            timestamp: chrono::Utc::now(),
            metadata: Some(json!({ "k": "v" })),
        };
        let json = serde_json::to_string(&a).expect("序列化失败");
        let decoded: Activity = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(decoded.id, a.id);
        assert_eq!(decoded.agent_id, a.agent_id);
        assert_eq!(decoded.activity_type, a.activity_type);
        assert_eq!(decoded.message, a.message);
        assert_eq!(decoded.timestamp, a.timestamp);
        assert_eq!(decoded.metadata, a.metadata);
    }

    #[test]
    fn activity_type_serde_roundtrip() {
        for t in [
            ActivityType::AgentStarted,
            ActivityType::AgentStopped,
            ActivityType::TaskCompleted,
            ActivityType::TaskFailed,
            ActivityType::MemoryOp,
            ActivityType::RetrievalOp,
            ActivityType::Error,
        ] {
            let json = serde_json::to_string(&t).expect("序列化失败");
            let decoded: ActivityType = serde_json::from_str(&json).expect("反序列化失败");
            assert_eq!(t, decoded);
        }
    }

    #[test]
    fn clear_history_works() {
        let mut stream = ActivityStream::new(16, 100);
        for i in 0..3 {
            stream.publish(Activity::new(
                format!("a-{i}"),
                "agent-1",
                ActivityType::MemoryOp,
                "op",
            ));
        }
        assert_eq!(stream.history_len(), 3);

        stream.clear_history();
        assert_eq!(stream.history_len(), 0);
        assert!(stream.recent(10).is_empty());
    }

    #[test]
    fn subscriber_count_tracks_subscriptions() {
        let stream = ActivityStream::new(8, 100);
        assert_eq!(stream.subscriber_count(), 0);

        let _rx1 = stream.subscribe();
        assert_eq!(stream.subscriber_count(), 1);

        let _rx2 = stream.subscribe();
        assert_eq!(stream.subscriber_count(), 2);
    }

    // 辅助：在同步测试中阻塞等待 future（避免把整个 mod 标记为 async）
    fn futures_block_on<F: std::future::Future>(f: F) -> F::Output {
        // tokio 的 test-util 提供的当前线程 runtime 足够处理 broadcast::recv
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("构建 runtime 失败")
            .block_on(f)
    }
}
