//! 6 层记忆公共类型

use serde::{Deserialize, Serialize};

use sparkfox_core::{Id, MemoryId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryKind {
    Raw,            // L0
    Working,        // L1
    Fact,           // L2
    Preference,     // L2
    Skill,          // L2
    Rule,           // L2
    Semantic,       // L3
    Episodic,       // L3
    GraphNode,      // L3
    GraphEdge,      // L3
    Identity,       // L4
    Role,           // L4
    History,        // L4
    StrategyLog,    // L5
    ErrorPattern,   // L5
    SelfEval,       // L5
}

impl MemoryKind {
    pub fn layer(&self) -> u8 {
        match self {
            Self::Raw => 0,
            Self::Working => 1,
            Self::Fact | Self::Preference | Self::Skill | Self::Rule => 2,
            Self::Semantic | Self::Episodic | Self::GraphNode | Self::GraphEdge => 3,
            Self::Identity | Self::Role | Self::History => 4,
            Self::StrategyLog | Self::ErrorPattern | Self::SelfEval => 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Id<MemoryId>,
    pub kind: MemoryKind,
    pub key: String,
    pub value: String,
    pub confidence: f32,
    pub ts: i64,
}

impl MemoryEntry {
    pub fn new(kind: MemoryKind, key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: Id::new(),
            kind,
            key: key.into(),
            value: value.into(),
            confidence: 1.0,
            ts: now_ts(),
        }
    }
}

impl sparkfox_core::MemoryLayer for MemoryEntry {
    const LAYER: u8 = 2;
    fn name() -> &'static str { "L2_core" }
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
