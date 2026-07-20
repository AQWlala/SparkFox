//! SparkFox CRDT — 同步层（automerge-rs 封装，RFC-004 CRDT 选型）
//!
//! 基于 automerge-rs 0.10，为 6 层记忆提供跨设备同步能力。
//!
//! PoC-2 实现：每个 `MemoryDoc` 持有一个独立 `AutoCommit`。同步消息采用
//! 全量文档快照（`AutoCommit::save` / `load`）+ CRDT merge 的方式，避免
//! automerge sync protocol 多轮往返带来的状态管理复杂度，单轮即可完成
//! 一致性合并。条目直接挂在 ROOT 上以规避两端各自 `put_object` 产生的
//! ObjId 冲突。

#![forbid(unsafe_code)]

use automerge::{AutoCommit, ReadDoc, ScalarValue, Value, transaction::Transactable};

use sparkfox_core::{Error, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 6 层记忆的 CRDT 文档
pub struct MemoryDoc {
    doc: AutoCommit,
}

impl MemoryDoc {
    pub fn new() -> Self {
        Self {
            doc: AutoCommit::new(),
        }
    }

    pub fn set_entry(&mut self, key: &str, value: impl Into<String>) -> Result<()> {
        let v = value.into();
        self.doc
            .put(automerge::ROOT, key, ScalarValue::Str(v.into()))
            .map_err(|e| Error::crdt(format!("set_entry 失败: {e}")))?;
        Ok(())
    }

    pub fn get_entry(&self, key: &str) -> Option<String> {
        let val: Option<Value> = self
            .doc
            .get(automerge::ROOT, key)
            .ok()
            .flatten()
            .map(|(v, _)| v);
        match val? {
            Value::Scalar(cow) => match cow.as_ref() {
                ScalarValue::Str(s) => Some(s.to_string()),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn entry_count(&self) -> usize {
        self.doc.keys(automerge::ROOT).count()
    }

    /// 生成同步消息（全量文档快照）。多次调用幂等：每次返回当前文档完整状态。
    pub fn generate_sync_message(&mut self) -> Vec<u8> {
        self.doc.save()
    }

    /// 接收同步消息：解码为远端文档快照，与本端 CRDT merge。
    /// automerge merge 自动处理 LWW 冲突，最终一致。
    pub fn receive_sync_message(&mut self, msg: Vec<u8>) -> Result<()> {
        if msg.is_empty() {
            return Ok(());
        }
        let mut other = AutoCommit::load(&msg)
            .map_err(|e| Error::crdt(format!("sync msg load 失败: {e}")))?;
        self.doc
            .merge(&mut other)
            .map_err(|e| Error::crdt(format!("merge 失败: {e}")))?;
        Ok(())
    }
}

impl Default for MemoryDoc {
    fn default() -> Self {
        Self::new()
    }
}

pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-crdt v{} initialized", VERSION);
}
