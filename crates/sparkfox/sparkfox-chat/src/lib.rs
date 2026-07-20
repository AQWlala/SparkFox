//! SparkFox Chat — BaiLongma 5 大特性清洁室重写
//!
//! 5 大特性：
//! 1. 思考过程可视化（[`thinking`] 模块）
//! 2. 信息热点追踪（[`hotspot`] 模块）
//! 3. 引用追踪（[`citation`] 模块）
//! 4. 多轮上下文（[`ConversationContext`]）
//! 5. 工具调用（[`ToolCall`] / [`ToolCallStatus`]）
//!
//! NOTICE: BaiLongma MIT 协议，清洁室重写（未拷贝代码，仅借鉴功能思路）

#![forbid(unsafe_code)]

pub mod citation;
pub mod hotspot;
pub mod thinking;

pub use citation::{Citation, CitationSet};
pub use hotspot::{Hotspot, HotspotTracker, HotspotType};
pub use thinking::{ThinkingBlock, ThinkingStage, extract_thinking_blocks, render_thinking};

use serde::{Deserialize, Serialize};

/// 聊天消息（含多轮上下文与工具调用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 消息唯一 ID
    pub id: String,
    /// 角色
    pub role: MessageRole,
    /// 文本内容
    pub content: String,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 思考过程块
    pub thinking_blocks: Vec<ThinkingBlock>,
    /// 引用集合
    pub citations: CitationSet,
    /// 工具调用列表
    pub tool_calls: Vec<ToolCall>,
    /// 多轮上下文：父消息 ID（None 表示对话起点）
    pub parent_id: Option<String>,
}

impl ChatMessage {
    /// 创建一条简单的用户消息（不含思考 / 引用 / 工具调用）
    pub fn user(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role: MessageRole::User,
            content: content.into(),
            timestamp: chrono::Utc::now(),
            thinking_blocks: Vec::new(),
            citations: CitationSet::new(),
            tool_calls: Vec::new(),
            parent_id: None,
        }
    }

    /// 创建一条简单的助手消息
    pub fn assistant(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: chrono::Utc::now(),
            thinking_blocks: Vec::new(),
            citations: CitationSet::new(),
            tool_calls: Vec::new(),
            parent_id: None,
        }
    }
}

/// 消息角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    /// 用户
    User,
    /// 助手
    Assistant,
    /// 系统提示
    System,
    /// 工具返回
    Tool,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 调用唯一 ID
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 调用参数（通常为 JSON 对象）
    pub arguments: serde_json::Value,
    /// 调用结果（调用前为 None）
    pub result: Option<serde_json::Value>,
    /// 调用状态
    pub status: ToolCallStatus,
}

impl ToolCall {
    /// 创建一个 Pending 状态的工具调用
    pub fn pending(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
            result: None,
            status: ToolCallStatus::Pending,
        }
    }
}

/// 工具调用状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCallStatus {
    /// 已排队未开始
    Pending,
    /// 执行中
    Running,
    /// 成功完成
    Success,
    /// 失败
    Failed,
}

/// 多轮上下文管理
///
/// 维护消息历史，支持按线程（parent_id 链）回溯，
/// 并在超过 `max_turns` 时丢弃最早的旧消息（FIFO）。
#[derive(Debug)]
pub struct ConversationContext {
    messages: Vec<ChatMessage>,
    max_turns: usize,
}

impl ConversationContext {
    /// 创建指定最大轮数的上下文
    pub fn new(max_turns: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_turns: max_turns.max(1),
        }
    }

    /// 追加一条消息
    ///
    /// 若历史已达到 `max_turns`，会自动丢弃最早的消息。
    pub fn add(&mut self, message: ChatMessage) {
        self.messages.push(message);
        while self.messages.len() > self.max_turns {
            self.messages.remove(0);
        }
    }

    /// 返回全部历史消息（按时间顺序）
    pub fn history(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// 获取指定消息所在线程
    ///
    /// 从该消息出发，沿 `parent_id` 反向追溯到对话起点，
    /// 再按时间正序返回该链上的所有消息（含目标消息本身）。
    /// 若 `message_id` 不存在，返回空。
    pub fn thread(&self, message_id: &str) -> Vec<&ChatMessage> {
        // 先找到目标消息索引
        let Some(target_idx) = self.messages.iter().position(|m| m.id == message_id) else {
            return Vec::new();
        };
        let mut chain: Vec<&ChatMessage> = Vec::new();
        let mut cur = &self.messages[target_idx];
        chain.push(cur);
        while let Some(pid) = &cur.parent_id {
            let Some(parent_idx) = self.messages.iter().position(|m| &m.id == pid) else {
                break;
            };
            // 防止循环引用导致死循环
            if chain.iter().any(|m| std::ptr::eq(*m, &self.messages[parent_idx])) {
                break;
            }
            cur = &self.messages[parent_idx];
            chain.push(cur);
        }
        chain.reverse();
        chain
    }

    /// 清空历史
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// 当前消息数
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// 最大轮次
    pub fn max_turns(&self) -> usize {
        self.max_turns
    }
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_add_and_history() {
        let mut ctx = ConversationContext::new(10);
        ctx.add(ChatMessage::user("m1", "你好"));
        ctx.add(ChatMessage::assistant("m2", "你好，有什么可以帮你？"));
        assert_eq!(ctx.history().len(), 2);
        assert_eq!(ctx.history()[0].id, "m1");
        assert_eq!(ctx.history()[1].role, MessageRole::Assistant);
    }

    #[test]
    fn conversation_evicts_oldest_when_full() {
        let mut ctx = ConversationContext::new(2);
        ctx.add(ChatMessage::user("m1", "a"));
        ctx.add(ChatMessage::user("m2", "b"));
        ctx.add(ChatMessage::user("m3", "c"));
        assert_eq!(ctx.len(), 2);
        assert_eq!(ctx.history()[0].id, "m2");
        assert_eq!(ctx.history()[1].id, "m3");
    }

    #[test]
    fn conversation_thread_traces_parent_chain() {
        let mut ctx = ConversationContext::new(10);
        ctx.add(ChatMessage::user("u1", "问题 1"));
        // m2 父消息为 u1
        let mut m2 = ChatMessage::assistant("a1", "回答 1");
        m2.parent_id = Some("u1".into());
        ctx.add(m2);
        // m3 父消息为 a1
        let mut m3 = ChatMessage::user("u2", "追问");
        m3.parent_id = Some("a1".into());
        ctx.add(m3);

        let thread = ctx.thread("u2");
        assert_eq!(thread.len(), 3);
        assert_eq!(thread[0].id, "u1");
        assert_eq!(thread[1].id, "a1");
        assert_eq!(thread[2].id, "u2");
    }

    #[test]
    fn conversation_thread_unknown_message_returns_empty() {
        let ctx = ConversationContext::new(10);
        assert!(ctx.thread("nonexistent").is_empty());
    }

    #[test]
    fn conversation_clear() {
        let mut ctx = ConversationContext::new(5);
        ctx.add(ChatMessage::user("m1", "x"));
        ctx.clear();
        assert!(ctx.is_empty());
        assert_eq!(ctx.max_turns(), 5, "clear 不应影响 max_turns");
    }

    #[test]
    fn conversation_new_clamps_zero_max_turns() {
        let ctx = ConversationContext::new(0);
        assert_eq!(ctx.max_turns(), 1, "0 应被夹紧为 1");
    }

    #[test]
    fn chat_message_serialization_roundtrip() {
        let mut msg = ChatMessage::assistant("m1", "答案");
        msg.parent_id = Some("p1".into());
        msg.thinking_blocks.push(ThinkingBlock {
            id: "t1".into(),
            content: "推理".into(),
            stage: ThinkingStage::Reasoning,
            start_offset: 0,
            end_offset: 10,
            timestamp: chrono::Utc::now(),
        });
        msg.citations.add(Citation::new("doc1", "chk1", 0.8));
        msg.tool_calls.push(ToolCall::pending("tc1", "search", serde_json::json!({"q": "rust"})));

        let json = serde_json::to_string(&msg).unwrap();
        let back: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "m1");
        assert_eq!(back.role, MessageRole::Assistant);
        assert_eq!(back.parent_id.as_deref(), Some("p1"));
        assert_eq!(back.thinking_blocks.len(), 1);
        assert_eq!(back.citations.len(), 1);
        assert_eq!(back.tool_calls.len(), 1);
        assert_eq!(back.tool_calls[0].status, ToolCallStatus::Pending);
    }

    #[test]
    fn tool_call_pending_constructor() {
        let tc = ToolCall::pending("id1", "calculator", serde_json::json!({"x": 1, "y": 2}));
        assert_eq!(tc.id, "id1");
        assert_eq!(tc.name, "calculator");
        assert_eq!(tc.status, ToolCallStatus::Pending);
        assert!(tc.result.is_none());
    }

    #[test]
    fn message_role_serialization() {
        let json = serde_json::to_string(&MessageRole::Assistant).unwrap();
        assert_eq!(json, "\"Assistant\"");
        let back: MessageRole = serde_json::from_str(&json).unwrap();
        assert_eq!(back, MessageRole::Assistant);
    }
}
