//! L5 元认知引擎 — 监控 L0-L4，提供策略日志/错误模式/自评
//!
//! 设计：基于 Pangu Nebula L5 蓝图（清洁室重写，无 Python 源码参考）

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyLog {
    pub task_id: String,
    pub strategy: String,
    pub outcome: String,    // "成功" | "失败"
    pub score: f32,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub task_id: String,
    pub stage: String,      // "json_parse" | "tool_call" | "llm_call" 等
    pub message: String,
    pub count: u32,
    pub first_ts: i64,
    pub last_ts: i64,
}

pub struct L5MetaEngine {
    strategies: HashMap<String, Vec<StrategyLog>>,
    errors: HashMap<String, Vec<ErrorPattern>>,
}

impl L5MetaEngine {
    pub fn new() -> Self {
        Self { strategies: HashMap::new(), errors: HashMap::new() }
    }

    pub fn log_strategy(&mut self, task_id: impl Into<String>, strategy: impl Into<String>, outcome: impl Into<String>, score: f32) {
        let task_id = task_id.into();
        self.strategies.entry(task_id.clone()).or_default().push(StrategyLog {
            task_id,
            strategy: strategy.into(),
            outcome: outcome.into(),
            score,
            ts: now_ts(),
        });
    }

    pub fn log_error(&mut self, task_id: impl Into<String>, stage: impl Into<String>, message: impl Into<String>) {
        let task_id = task_id.into();
        let stage = stage.into();
        let message = message.into();
        let ts = now_ts();
        let entries = self.errors.entry(task_id.clone()).or_default();
        if let Some(p) = entries.iter_mut().find(|p| p.stage == stage && p.message == message) {
            p.count += 1;
            p.last_ts = ts;
        } else {
            entries.push(ErrorPattern {
                task_id: task_id.clone(),
                stage,
                message,
                count: 1,
                first_ts: ts,
                last_ts: ts,
            });
        }
    }

    pub fn strategy_logs(&self, task_id: &str) -> &[StrategyLog] {
        self.strategies.get(task_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn error_patterns(&self, task_id: &str) -> &[ErrorPattern] {
        self.errors.get(task_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// 自评：基于历史成功率推荐策略
    pub fn recommend_strategy(&self, task_id: &str) -> Option<&str> {
        let logs = self.strategies.get(task_id)?;
        let mut best: Option<(&StrategyLog, f32)> = None;
        for log in logs {
            if log.outcome == "成功" {
                if best.is_none() || log.score > best.unwrap().1 {
                    best = Some((log, log.score));
                }
            }
        }
        best.map(|(l, _)| l.strategy.as_str())
    }
}

impl Default for L5MetaEngine {
    fn default() -> Self { Self::new() }
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
