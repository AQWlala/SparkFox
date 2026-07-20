//! SparkFox LLM — LLM Provider 抽象层
//!
//! 统一封装 OpenAI / Anthropic / Google / AWS Bedrock / 本地模型等 LLM 调用。
//! 为 SparkFox Agent 提供一致的 LLM 接口。
//!
//! 核心抽象：[`LlmProvider`] trait，支持三种调用模式：
//! - `complete`: 普通 completion
//! - `stream_complete`: 流式 completion
//! - `structured_complete`: 结构化 JSON 输出（SAG 提取流程依赖，含 jsonrepair 修复）
//!
//! Task 7.2.2 新增 [`AuditedProvider`]：装饰器，自动记录 LLM 调用审计日志（S-01 P0 修复）。
//!
//! Sub-Step 10.1.2：[`OpenAIProvider`] 真实实现落地（见 `openai` 模块），
//! 通过 `reqwest` 调用 OpenAI `/v1/chat/completions` API。
//!
//! Sub-Step 10.1.3：[`AnthropicProvider`] 真实实现落地（见 `anthropic` 模块），
//! 通过 `reqwest` 调用 Anthropic `/v1/messages` API（Claude 系列模型）。
//!
//! Task 7.2 剩余：[`LlmStream`] 流式响应封装（基于 tokio mpsc channel），
//! 以及 4 个 Provider 占位结构体（Anthropic / Google / Bedrock / Local）。

#![forbid(unsafe_code)]

pub mod anthropic;
pub mod factory;
pub mod ollama;
pub mod openai;
pub mod provider;
pub mod stream;

pub use anthropic::AnthropicProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use provider::{repair_json, AuditedProvider, LlmProvider};
pub use stream::LlmStream;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化函数（占位 — 后续 Phase 接入日志/panic hook）
pub fn init() {
    // TODO: Phase 1 阶段接入 env_logger / tracing
}
