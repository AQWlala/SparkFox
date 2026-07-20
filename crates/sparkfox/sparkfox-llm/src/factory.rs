//! Provider 工厂 — 根据名称字符串创建对应 LlmProvider 实例（Sub-Step 10.1.6）
//!
//! 提供 [`ProviderFactory::create`] 静态方法，根据 provider 名称（如 `"openai"` /
//! `"anthropic"` / `"ollama"` / `"mock"`）返回 `Box<dyn LlmProvider>`，
//! 屏蔽各家 Provider 构造细节，便于业务层按配置动态选择 LLM 后端。
//!
//! # 用法
//! ```no_run
//! # use sparkfox_llm::factory::{ProviderConfig, ProviderFactory};
//! # use sparkfox_llm::LlmProvider;
//! # async fn demo() -> sparkfox_core::Result<()> {
//! let config = ProviderConfig {
//!     api_key: Some("sk-xxx".into()),
//!     model: "gpt-4".into(),
//!     base_url: None,
//! };
//! let provider = ProviderFactory::create("openai", config)?;
//! let text = provider.complete("你好").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # 设计要点
//! - 工厂方法返回 `Box<dyn LlmProvider>` trait 对象，屏蔽具体类型
//! - [`ProviderConfig`] 统一封装 api_key / model / base_url，覆盖所有 Provider 构造需求
//! - 内置 [`FactoryMockProvider`] 用于测试场景，`complete` 直接返回 Ok（不依赖真实 API）
//! - 未知 provider 名返回 [`Error::llm`](sparkfox_core::Error::llm) 错误

#![forbid(unsafe_code)]

use async_trait::async_trait;

use sparkfox_core::{Error, Result};

use crate::{AnthropicProvider, LlmProvider, LlmStream, OllamaProvider, OpenAIProvider};

// ============================================================================
// ProviderConfig — 统一配置结构
// ============================================================================

/// Provider 创建参数 — 统一封装各家 Provider 构造所需字段
///
/// # 字段
/// - `api_key`: API Key（OpenAI / Anthropic 必填；Ollama / Mock 可空）
/// - `model`: 模型名（如 `"gpt-4"` / `"claude-3-5-sonnet-20240620"` / `"llama3"`）
/// - `base_url`: 自定义 API 基础 URL（`None` 时使用各 Provider 默认值）
///
/// # 设计
/// 字段全部为 `Option<String>` 或 `String` 以兼容所有 Provider 构造签名：
/// - OpenAI / Anthropic 的 `new(api_key, model)` 接收两个参数
/// - Ollama 的 `new(model)` 仅接收 model（无 api_key）
/// - `base_url` 字段为后续扩展 `with_base_url` 构造器预留（当前 v1.1.0 工厂暂不使用）
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// API Key（OpenAI / Anthropic 必填；Ollama / Mock 可为 None）
    pub api_key: Option<String>,
    /// 模型名（如 "gpt-4" / "claude-3-5-sonnet-20240620" / "llama3"）
    pub model: String,
    /// 自定义 API 基础 URL（None 时使用各 Provider 默认值；当前工厂默认走 `new()` 不使用此字段）
    pub base_url: Option<String>,
}

impl ProviderConfig {
    /// 创建 ProviderConfig（便捷构造器，api_key 和 base_url 均为 None）
    ///
    /// 适用于 Ollama / Mock 等不需要 API Key 的 Provider。
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            api_key: None,
            model: model.into(),
            base_url: None,
        }
    }

    /// 设置 api_key（builder 风格）
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// 设置 base_url（builder 风格）
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }
}

// ============================================================================
// ProviderFactory — 工厂入口
// ============================================================================

/// Provider 工厂 — 根据 provider 名称创建对应 [`LlmProvider`] 实例
///
/// # 支持的 provider 名称
/// | 名称 | 创建类型 | 备注 |
/// |------|----------|------|
/// | `"openai"` | [`OpenAIProvider`] | 真实 HTTP，需 api_key |
/// | `"anthropic"` | [`AnthropicProvider`] | 真实 HTTP，需 api_key |
/// | `"ollama"` | [`OllamaProvider`] | 本地服务，无 api_key |
/// | `"mock"` | [`FactoryMockProvider`] | 测试用，complete 直接返回 Ok |
/// | 其他 | — | 返回 [`Error::llm`] |
pub struct ProviderFactory;

impl ProviderFactory {
    /// 根据 provider 名称创建对应 Provider 实例
    ///
    /// # 参数
    /// - `name`: provider 名称（"openai" / "anthropic" / "ollama" / "mock"）
    /// - `config`: 创建参数（含 api_key / model / base_url）
    ///
    /// # 返回
    /// - `Ok(Box<dyn LlmProvider>)`: 创建成功
    /// - `Err`: 未知 provider 名（错误信息含未知名称 + "未知" 字样）
    ///
    /// # 用法
    /// ```no_run
    /// # use sparkfox_llm::factory::{ProviderConfig, ProviderFactory};
    /// # use sparkfox_llm::LlmProvider;
    /// # async fn demo() -> sparkfox_core::Result<()> {
    /// let provider = ProviderFactory::create(
    ///     "openai",
    ///     ProviderConfig::new("gpt-4").with_api_key("sk-xxx"),
    /// )?;
    /// let _text = provider.complete("hello").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create(name: &str, config: ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        match name {
            "openai" => {
                let api_key = config
                    .api_key
                    .ok_or_else(|| Error::llm("OpenAI Provider 需要 api_key，但 ProviderConfig.api_key 为 None"))?;
                Ok(Box::new(OpenAIProvider::new(api_key, config.model)))
            }
            "anthropic" => {
                let api_key = config.api_key.ok_or_else(|| {
                    Error::llm("Anthropic Provider 需要 api_key，但 ProviderConfig.api_key 为 None")
                })?;
                Ok(Box::new(AnthropicProvider::new(api_key, config.model)))
            }
            "ollama" => {
                // Ollama 不需要 api_key（本地服务）
                Ok(Box::new(OllamaProvider::new(config.model)))
            }
            "mock" => Ok(Box::new(FactoryMockProvider::new())),
            _ => Err(Error::llm(format!(
                "未知 provider: {name}（支持的名称: openai / anthropic / ollama / mock）"
            ))),
        }
    }
}

// ============================================================================
// FactoryMockProvider — 工厂内置 Mock 实现（用于测试）
// ============================================================================

/// 工厂内置 Mock Provider — `complete` 直接返回固定响应，不调用真实 LLM
///
/// # 设计
/// - 独立于 `provider.rs` 内部 tests 模块的 `MockProvider`（不依赖私有类型）
/// - 响应内容含 "Mock" 字样，便于 E2E 测试通过响应特征区分 mock 与真实 provider
/// - `stream_complete` 返回 Err（mock 不模拟流式行为）
/// - `structured_complete` 走默认实现（基于 complete + jsonrepair）
pub struct FactoryMockProvider {
    /// mock 响应内容（complete 调用时返回）
    response: String,
}

impl FactoryMockProvider {
    /// 创建 FactoryMockProvider（使用默认 mock 响应）
    pub fn new() -> Self {
        Self {
            response: "Mock response from FactoryMockProvider".to_string(),
        }
    }

    /// 创建 FactoryMockProvider，指定自定义响应内容
    pub fn with_response(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }
}

impl Default for FactoryMockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for FactoryMockProvider {
    /// 直接返回预设响应，不调用真实 LLM
    async fn complete(&self, _prompt: &str) -> Result<String> {
        Ok(self.response.clone())
    }

    /// mock 不支持流式 — 返回明确错误
    async fn stream_complete(&self, _prompt: &str) -> Result<LlmStream> {
        Err(Error::llm(
            "FactoryMockProvider 不支持 stream_complete（mock 仅用于 complete E2E 测试）",
        ))
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 ProviderConfig::new 默认构造器
    #[test]
    fn test_provider_config_new() {
        let config = ProviderConfig::new("gpt-4");
        assert_eq!(config.model, "gpt-4");
        assert!(config.api_key.is_none());
        assert!(config.base_url.is_none());
    }

    /// 验证 ProviderConfig builder 风格
    #[test]
    fn test_provider_config_builder() {
        let config = ProviderConfig::new("gpt-4")
            .with_api_key("sk-test")
            .with_base_url("http://localhost:8080");
        assert_eq!(config.api_key.as_deref(), Some("sk-test"));
        assert_eq!(config.base_url.as_deref(), Some("http://localhost:8080"));
        assert_eq!(config.model, "gpt-4");
    }

    /// 验证 FactoryMockProvider::new 默认响应含 "Mock"
    #[test]
    fn test_factory_mock_provider_default_response() {
        let p = FactoryMockProvider::new();
        assert!(p.response.contains("Mock"));
    }

    /// 验证 FactoryMockProvider::default 等价于 new
    #[test]
    fn test_factory_mock_provider_default() {
        let p = FactoryMockProvider::default();
        assert!(p.response.contains("Mock"));
    }

    /// 验证 FactoryMockProvider::with_response 自定义响应
    #[test]
    fn test_factory_mock_provider_with_response() {
        let p = FactoryMockProvider::with_response("custom mock response");
        assert_eq!(p.response, "custom mock response");
    }

    /// 验证工厂 create 对未知 provider 返回 Err 且错误信息含未知名
    #[test]
    fn test_factory_unknown_provider() {
        let config = ProviderConfig::new("any");
        let result = ProviderFactory::create("nonexistent", config);
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("期望 Err，但返回 Ok"),
        };
        let err_msg = format!("{err}");
        assert!(err_msg.contains("nonexistent"));
        assert!(err_msg.contains("未知"));
    }
}
