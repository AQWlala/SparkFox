//! tests/common — 跨测试文件共享的工具模块
//!
//! Sub-Step 10.14.2 REFACTOR 阶段：提取 nDCG 计算等通用工具，
//! 供 reranker_test / xlm_roberta_load_test 等多个测试文件复用。

pub mod metrics;
