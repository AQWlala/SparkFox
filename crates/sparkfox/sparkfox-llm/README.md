# sparkfox-llm

> SparkFox LLM Provider 抽象层 — OpenAI / Anthropic / Google / Bedrock / Local 统一接口

## 功能

- **LlmProvider trait**：统一抽象 5 个 LLM 厂商，支持三种调用模式
  - `complete`：普通 completion
  - `stream_complete`：流式 completion（基于 tokio mpsc channel）
  - `structured_complete`：结构化 JSON 输出（SAG 提取流程依赖，含 jsonrepair 修复）
- **LlmStream**：流式响应封装（`tokio::sync::mpsc::Receiver`），支持背压与取消
- **AuditedProvider**（Task 7.2.2）：装饰器模式，自动记录 LLM 调用审计日志（S-01 P0 修复）
- **JSON 修复**：`jsonrepair` 处理 LLM 输出的非严格 JSON（尾随逗号、缺引号等）

## 架构

```
                ┌──────────────────────────────────┐
                │       sparkfox-agent             │
                │   (Agent 调用 LlmProvider)       │
                └────────────────┬─────────────────┘
                                 │
                                 ▼
                ┌──────────────────────────────────┐
                │       sparkfox-llm               │
                │  ┌────────────────────────────┐  │
                │  │  LlmProvider trait         │  │
                │  └─────────┬──────────────────┘  │
                │            │                      │
                │  ┌─────────▼──────────────────┐  │
                │  │  AuditedProvider           │  │
                │  │  (装饰器，S-01 P0 修复)    │  │
                │  └─────────┬──────────────────┘  │
                │            │                      │
                │     ┌──────┼──────┬──────┬─────┐  │
                │     ▼      ▼      ▼      ▼     ▼  │
                │  OpenAI  Anthropic Google Bedrock Local
                └──────────────────────────────────┘
                                 │
                                 ▼
                ┌──────────────────────────────────┐
                │       sparkfox-security          │
                │  (LlmAuditLogger 审计日志)       │
                └──────────────────────────────────┘
```

**依赖**：`sparkfox-core` / `sparkfox-security` / `async-trait` / `tokio` / `futures` / `serde_json` / `jsonrepair`

## 使用

```rust
use sparkfox_llm::{LlmProvider, AuditedProvider};

// 假设有 OpenAI provider 实现
let provider = MyOpenAiProvider::new("sk-...");

// 包装审计日志（每次调用自动记录到 sparkfox-security::LlmAuditLogger）
let audited = AuditedProvider::new(provider);

// 普通 completion
let resp = audited.complete("你好").await?;

// 流式
use sparkfox_llm::LlmStream;
let mut stream = audited.stream_complete("讲个故事").await?;
while let Some(chunk) = stream.next().await {
    print!("{}", chunk?);
}

// 结构化 JSON（SAG 实体抽取依赖）
let json: serde_json::Value = audited.structured_complete("提取实体: ...").await?;
```

## 测试

```bash
# 单元测试（lib）
cargo test -p sparkfox-llm --lib

# 全部测试
cargo test -p sparkfox-llm
```

## 安全约束

- `#![forbid(unsafe_code)]` — 全 crate 禁用 unsafe
- AuditedProvider 装饰器记录所有 LLM 调用（输入 / 输出 / 耗时 / token 数），仅本地不同步
- structured_complete 输出经 jsonrepair 修复后必须可解析为有效 JSON，否则返回错误（防 prompt 注入通过结构化字段逃逸）

## 许可证

AGPL-3.0-only，详见工作区根 `LICENSE`。

## 致谢

- [jsonrepair](https://github.com/josdejong/jsonrepair)（MIT）— 修复 LLM 输出的非严格 JSON
- [OpenAI API](https://platform.openai.com/docs) / [Anthropic API](https://docs.anthropic.com) — Provider 接口参考
