//! Sub-Step 10.1.6 — Provider 工厂 + E2E 集成测试
//!
//! 验证 [`sparkfox_llm::factory::ProviderFactory`] 能根据 provider 名称字符串
//! 创建对应 Provider 实例，并通过 mock 场景验证 E2E 调用链路完整。
//!
//! # 测试用例
//! 1. `test_factory_creates_openai_by_name` — 工厂按名 "openai" 创建出 OpenAIProvider
//! 2. `test_factory_creates_anthropic_by_name` — 工厂按名 "anthropic" 创建出 AnthropicProvider
//! 3. `test_factory_creates_ollama_by_name` — 工厂按名 "ollama" 创建出 OllamaProvider
//! 4. `test_factory_creates_mock_by_name` — 工厂按名 "mock" 创建出 FactoryMockProvider
//! 5. `test_factory_unknown_provider_returns_error` — 未知 provider 名返回明确错误
//! 6. `test_e2e_three_providers_complete_via_factory` — 工厂创建 3 个 mock provider，complete 全部 Ok
//!
//! # 类型识别策略
//! 由于 `LlmProvider` trait 不暴露 `type_name` 方法（且本任务禁止修改 `provider.rs`），
//! 测试通过 `complete()` 的错误信息（含 "OpenAI" / "Anthropic" / "Ollama"）
//! 或 mock 响应内容（含 "Mock"）来间接验证具体 Provider 类型。
//! 真 provider 在无 API key / 无服务时返回的错误信息天然包含 provider 名，
//! 此特征被用作类型指纹。

#![forbid(unsafe_code)]

use sparkfox_llm::factory::{ProviderConfig, ProviderFactory};
// 注意：Box<dyn LlmProvider> 通过 trait object 调用方法，无需 use LlmProvider trait

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 构造测试用 ProviderConfig（带 API key）
fn make_config_with_key(model: &str, api_key: &str) -> ProviderConfig {
    ProviderConfig {
        api_key: Some(api_key.to_string()),
        model: model.to_string(),
        base_url: None,
    }
}

/// 构造测试用 ProviderConfig（无 API key，用于 Ollama / Mock）
fn make_config_no_key(model: &str) -> ProviderConfig {
    ProviderConfig {
        api_key: None,
        model: model.to_string(),
        base_url: None,
    }
}

// ---------------------------------------------------------------------------
// 测试用例 1：工厂按名 "openai" 创建出 OpenAIProvider
// ---------------------------------------------------------------------------

/// 验证 `ProviderFactory::create("openai", config)` 返回 Ok，
/// 且返回的 Provider 在 complete 时返回包含 "OpenAI" 的错误（无真实 API key 时）。
#[tokio::test]
async fn test_factory_creates_openai_by_name() {
    let config = make_config_with_key("gpt-4", "sk-fake-key-for-test-only");
    let provider = ProviderFactory::create("openai", config)
        .expect("工厂应成功创建 OpenAIProvider");

    // 无真实 API key 时 complete 会失败，但错误信息应含 "OpenAI" 字样
    let result = provider.complete("ping").await;
    let err = match result {
        Err(e) => e,
        Ok(value) => panic!("期望 complete 失败，但返回 Ok: {value}"),
    };
    let err_msg = format!("{err}");
    assert!(
        err_msg.contains("OpenAI"),
        "错误信息应含 'OpenAI' 以确认类型，实际: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// 测试用例 2：工厂按名 "anthropic" 创建出 AnthropicProvider
// ---------------------------------------------------------------------------

/// 验证 `ProviderFactory::create("anthropic", config)` 返回 Ok，
/// 且返回的 Provider 在 complete 时返回包含 "Anthropic" 的错误。
#[tokio::test]
async fn test_factory_creates_anthropic_by_name() {
    let config = make_config_with_key(
        "claude-3-5-sonnet-20240620",
        "sk-ant-fake-key-for-test-only",
    );
    let provider = ProviderFactory::create("anthropic", config)
        .expect("工厂应成功创建 AnthropicProvider");

    let result = provider.complete("ping").await;
    let err = match result {
        Err(e) => e,
        Ok(value) => panic!("期望 complete 失败，但返回 Ok: {value}"),
    };
    let err_msg = format!("{err}");
    assert!(
        err_msg.contains("Anthropic"),
        "错误信息应含 'Anthropic' 以确认类型，实际: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// 测试用例 3：工厂按名 "ollama" 创建出 OllamaProvider
// ---------------------------------------------------------------------------

/// 验证 `ProviderFactory::create("ollama", config)` 返回 Ok，
/// 且返回的 Provider 在 complete 时返回包含 "Ollama" 的错误（无本地 Ollama 服务时）。
#[tokio::test]
async fn test_factory_creates_ollama_by_name() {
    let config = make_config_no_key("llama3");
    let provider = ProviderFactory::create("ollama", config)
        .expect("工厂应成功创建 OllamaProvider");

    let result = provider.complete("ping").await;
    let err = match result {
        Err(e) => e,
        Ok(value) => panic!("期望 complete 失败（无本地 Ollama 服务），但返回 Ok: {value}"),
    };
    let err_msg = format!("{err}");
    assert!(
        err_msg.contains("Ollama"),
        "错误信息应含 'Ollama' 以确认类型，实际: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// 测试用例 4：工厂按名 "mock" 创建出 FactoryMockProvider
// ---------------------------------------------------------------------------

/// 验证 `ProviderFactory::create("mock", config)` 返回 Ok，
/// 且返回的 Provider 调用 complete 返回 Ok，响应中含 "Mock" 字样
/// （区分于真实 provider，确认是 FactoryMockProvider 类型）。
#[tokio::test]
async fn test_factory_creates_mock_by_name() {
    let config = make_config_no_key("mock-model");
    let provider = ProviderFactory::create("mock", config)
        .expect("工厂应成功创建 FactoryMockProvider");

    let response = provider
        .complete("ping")
        .await
        .expect("FactoryMockProvider.complete 应返回 Ok");

    // mock 响应需含 "Mock" 标识以区分类型
    assert!(
        response.contains("Mock") || response.contains("mock"),
        "mock 响应应含 'Mock' 标识以确认类型，实际: {response}"
    );
}

// ---------------------------------------------------------------------------
// 测试用例 5：未知 provider 名返回明确错误
// ---------------------------------------------------------------------------

/// 验证 `ProviderFactory::create("unknown_provider", config)` 返回 Err，
/// 错误信息应含未知 provider 名或 "未知" 字样。
#[tokio::test]
async fn test_factory_unknown_provider_returns_error() {
    let config = make_config_no_key("any-model");
    let result = ProviderFactory::create("unknown_provider", config);
    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("未知 provider 名应返回 Err，但返回 Ok"),
    };
    let err_msg = format!("{err}");
    assert!(
        err_msg.contains("unknown_provider") || err_msg.contains("未知"),
        "错误信息应含未知 provider 名或 '未知' 字样，实际: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// 测试用例 6：E2E — 通过工厂创建 3 个 mock provider，complete 全部 Ok
// ---------------------------------------------------------------------------

/// 验证通过 ProviderFactory 连续创建 3 个 mock provider，
/// 每个 complete 调用都返回 Ok 且响应非空，模拟多 provider 协同 E2E 场景。
#[tokio::test]
async fn test_e2e_three_providers_complete_via_factory() {
    let prompts = [
        "你好，请介绍一下你自己",
        "请帮我生成一段 SparkFox 项目介绍",
        "今天的天气如何？",
    ];

    for (i, prompt) in prompts.iter().enumerate() {
        let config = ProviderConfig {
            api_key: None,
            model: format!("mock-model-{i}"),
            base_url: None,
        };
        let provider = ProviderFactory::create("mock", config)
            .expect("工厂应成功创建第 {i} 个 mock provider");
        let response = provider
            .complete(prompt)
            .await
            .expect("第 {i} 个 mock provider complete 应返回 Ok");
        assert!(
            !response.is_empty(),
            "第 {i} 个 mock provider 响应不应为空"
        );
    }
}
