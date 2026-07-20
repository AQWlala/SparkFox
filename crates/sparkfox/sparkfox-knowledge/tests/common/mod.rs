//! Sub-Step 10.3.2 — sparkfox-knowledge 测试共享辅助模块
//!
//! ## 用途
//! 提供 100 case 中文 NER 数据集的加载函数与类型定义，供多个测试文件复用：
//! - `tests/zh_ner_f1.rs`（10.3.2 数据集验证）
//! - 后续 10.3.3 LLM F1 验证（Sub-Step 10.3.3）
//! - 后续 10.6.2 jieba 降级 F1 验证（Sub-Step 10.6.2）
//!
//! ## 使用方式
//! 在测试文件顶部声明：
//! ```ignore
//! mod common;
//! use common::load_zh_ner_dataset;
//! ```
//!
//! ## 数据集说明
//! - **来源**：人工标注，覆盖 6 类实体（PERSON / LOCATION / ORGANIZATION / TIME / NUMBER / EVENT）
//! - **分布**（软约束，spec 目标 30/30/20/20）：
//!   - PERSON: 36 个实体（含真实人名 / 历史人物 / 虚构角色 / 名人）
//!   - LOCATION: 64 个实体（含城市 / 省份 / 国家 / 地标 / 山川）
//!   - ORGANIZATION: 27 个实体（含公司 / 学校 / 政府 / 国际组织）
//!   - TIME: 28 个实体（含绝对日期 / 相对时间词 / 时间段）
//!   - NUMBER: 20 个实体（含金额 / 百分比 / 计量值 / 纯数字）
//!   - EVENT: 39 个实体（含历史事件 / 会议 / 庆典 / 动作事件）
//! - **格式**：JSON 数组，每个 case 含 `id` / `text` / `expected_entities`
//! - **case 难度递进**：简单（单一实体）→ 中等（2-3 实体）→ 复杂（4+ 实体，如 case 26/88/100）
//! - **边界覆盖**：含短文本（case 45/47/85/87）、长文本（case 100）、
//!   中英混合（case 99）、含标点（case 88/100）、含空格（case 15/24/25/33 等）

#![forbid(unsafe_code)]

use serde::Deserialize;

/// 单个 NER 测试 case
///
/// 对应 `tests/data/zh_ner_100_cases.json` 中的一条记录。
///
/// ## 跨测试文件 dead_code 说明
/// 每个 `tests/*.rs` 是独立 crate，引用 `common` 时会重新编译本模块。
/// `NerCase` / `ExpectedEntity` / `load_zh_ner_dataset` 仅被 NER 相关测试使用
/// （如 `zh_ner_f1.rs`），其他测试（如 `bench_compare_4_strategies.rs`）仅使用
/// `bench_metrics` 模块，会导致这些 NER 类型在本测试 crate 内未使用而触发
/// dead_code warning。因此标注 `#[allow(dead_code)]` 抑制跨 crate 的假阳性。
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct NerCase {
    /// case 编号（1-100，连续递增）
    pub id: usize,
    /// 输入文本（自然中文句子）
    pub text: String,
    /// 期望识别的实体列表（至少 1 个）
    pub expected_entities: Vec<ExpectedEntity>,
}

/// 期望识别的实体
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ExpectedEntity {
    /// 实体类型（PERSON / LOCATION / ORGANIZATION / TIME / NUMBER / EVENT）
    pub entity_type: String,
    /// 实体文本（必须是 `NerCase.text` 的子串）
    pub text: String,
}

/// 加载 100 case 中文 NER 数据集
///
/// 使用 `include_str!` 在编译时嵌入 JSON 文件，运行时无 I/O 开销。
///
/// ## Panic
/// 若 JSON 解析失败会 panic（测试场景下可接受，说明数据集格式有误）。
#[allow(dead_code)]
pub fn load_zh_ner_dataset() -> Vec<NerCase> {
    let json = include_str!("../data/zh_ner_100_cases.json");
    serde_json::from_str(json).expect("zh_ner_100_cases.json 解析失败")
}

// Sub-Step 10.6.2 REFACTOR：F1 评估指标共享模块
// 供 jieba_fallback_f1_test.rs 及后续 10.3.3 LLM F1 验证复用。
pub mod metrics;

// Sub-Step 12.3.2 REFACTOR：Benchmark 指标共享模块
// 供 bench_compare_4_strategies.rs 及后续 benchmark 测试文件复用。
pub mod bench_metrics;
