//! SparkFox Thinking — ThoughtStream 后端（流式思考过程推送）
//!
//! 参考 BaiLongma 的思考过程可视化功能（清洁室重写，未拷贝任何源代码）。
//!
//! 设计要点：
//! - 基于 `tokio::sync::broadcast` 多生产者多消费者广播通道
//! - 任意时刻可调用 `subscribe()` 获得一个新的接收端
//! - `publish()` 为非阻塞调用：若无订阅者则丢弃该条 thought
//! - `Thought` 携带时间戳与可选 `agent_id`，便于前端按 Agent 分组展示

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// 思考流推送器：内部封装一个 broadcast channel。
///
/// `buffer` 决定通道容量：当消费者跟不上时，旧消息会被丢弃。
/// 通常 256 即可满足前端实时可视化需求。
pub struct ThoughtStream {
    sender: broadcast::Sender<Thought>,
    capacity: usize,
}

/// 单条思考事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    /// 思考内容（自然语言文本）
    pub content: String,
    /// 当前所处阶段
    pub stage: ThoughtStage,
    /// 事件时间戳（UTC）
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 关联的 Agent ID（可选，用于多 Agent 场景按 Agent 分组）
    pub agent_id: Option<String>,
}

/// 思考阶段枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThoughtStage {
    /// 推理中
    Reasoning,
    /// 检索中
    Retrieving,
    /// 组织答案中
    Composing,
    /// 最终思考
    Final,
}

impl Thought {
    /// 构造一条带当前时间戳的 thought
    pub fn new(content: impl Into<String>, stage: ThoughtStage) -> Self {
        Self {
            content: content.into(),
            stage,
            timestamp: chrono::Utc::now(),
            agent_id: None,
        }
    }

    /// 设置 agent_id
    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }
}

impl ThoughtStream {
    /// 创建一个容量为 `buffer` 的思考流
    pub fn new(buffer: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer);
        Self { sender, capacity: buffer }
    }

    /// 订阅思考流，返回一个新的接收端
    pub fn subscribe(&self) -> broadcast::Receiver<Thought> {
        self.sender.subscribe()
    }

    /// 发布一条 thought 到所有当前订阅者
    ///
    /// 注意：若无订阅者或订阅者队列已满，该消息会被静默丢弃。
    pub fn publish(&self, thought: Thought) {
        let _ = self.sender.send(thought);
    }

    /// 当前队列中尚未被所有订阅者消费的消息数
    ///
    /// 注意：这不是订阅者数量。如需订阅者数量，请使用 `sender.receiver_count()`
    /// （本结构未直接暴露，因为 broadcast 内部计数仅作调试用途）。
    pub fn len(&self) -> usize {
        self.sender.len()
    }

    /// 队列中是否没有未消费的消息
    pub fn is_empty(&self) -> bool {
        self.sender.is_empty()
    }

    /// 通道容量（构造时指定的 buffer 大小）
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for ThoughtStream {
    fn default() -> Self {
        Self::new(256)
    }
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_and_subscribe_receives_thought() {
        // 验证 publish + subscribe 基本流程
        let stream = ThoughtStream::new(16);
        let mut rx = stream.subscribe();

        let thought = Thought::new("正在分析用户意图", ThoughtStage::Reasoning)
            .with_agent_id("agent-001");
        stream.publish(thought.clone());

        let received = rx.recv().await.expect("应当收到一条 thought");
        assert_eq!(received.content, thought.content);
        assert_eq!(received.stage, ThoughtStage::Reasoning);
        assert_eq!(received.agent_id.as_deref(), Some("agent-001"));
    }

    #[test]
    fn thought_serialization_roundtrip() {
        // 验证 Thought 的 serde 序列化/反序列化可往返
        let thought = Thought {
            content: "检索知识库".into(),
            stage: ThoughtStage::Retrieving,
            timestamp: chrono::Utc::now(),
            agent_id: Some("agent-7".into()),
        };

        let json = serde_json::to_string(&thought).expect("序列化失败");
        let decoded: Thought = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(decoded.content, thought.content);
        assert_eq!(decoded.stage, thought.stage);
        assert_eq!(decoded.agent_id, thought.agent_id);
        assert_eq!(decoded.timestamp, thought.timestamp);
    }

    #[test]
    fn thought_stage_serde() {
        // 验证 ThoughtStage 各变体能正确序列化
        for stage in [
            ThoughtStage::Reasoning,
            ThoughtStage::Retrieving,
            ThoughtStage::Composing,
            ThoughtStage::Final,
        ] {
            let json = serde_json::to_string(&stage).expect("序列化失败");
            let decoded: ThoughtStage = serde_json::from_str(&json).expect("反序列化失败");
            assert_eq!(stage, decoded);
        }
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive() {
        // 验证多个订阅者同时接收同一条 thought
        let stream = ThoughtStream::new(16);
        let mut rx1 = stream.subscribe();
        let mut rx2 = stream.subscribe();
        let mut rx3 = stream.subscribe();

        stream.publish(Thought::new("组织最终答案", ThoughtStage::Composing));

        for rx in [&mut rx1, &mut rx2, &mut rx3] {
            let received = rx.recv().await.expect("每个订阅者都应收到");
            assert_eq!(received.content, "组织最终答案");
            assert_eq!(received.stage, ThoughtStage::Composing);
        }
    }

    #[tokio::test]
    async fn len_reflects_queued_messages() {
        // 验证 len() 反映队列中尚未消费的消息数（非订阅者数）
        let stream = ThoughtStream::new(8);
        // 创建一个订阅者但暂不消费
        let _rx = stream.subscribe();
        assert!(stream.is_empty());

        stream.publish(Thought::new("msg1", ThoughtStage::Reasoning));
        stream.publish(Thought::new("msg2", ThoughtStage::Reasoning));
        // 队列中应有 2 条未消费消息
        assert_eq!(stream.len(), 2);

        // 容量保持不变
        assert_eq!(stream.capacity(), 8);
    }

    #[tokio::test]
    async fn publish_without_subscribers_is_silent() {
        // 验证无订阅者时 publish 不 panic
        let stream = ThoughtStream::new(4);
        stream.publish(Thought::new("无人接收", ThoughtStage::Final));
        // 不应 panic
    }

    #[test]
    fn default_buffer_is_256() {
        let stream = ThoughtStream::default();
        assert_eq!(stream.capacity(), 256);
    }

    #[test]
    fn thought_with_agent_id_builder() {
        let t = Thought::new("hi", ThoughtStage::Reasoning).with_agent_id("a1");
        assert_eq!(t.agent_id.as_deref(), Some("a1"));
        assert_eq!(t.content, "hi");

        let t2 = Thought::new("hi2", ThoughtStage::Final);
        assert!(t2.agent_id.is_none());
    }
}
