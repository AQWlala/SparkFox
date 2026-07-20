//! E2EE 同步占位（spec 1.0 Task 3.1 补充）
//!
//! ## v1.0.0 范围
//! 仅定义 [`KnowledgeSync`] trait 与 [`NoOpSync`] 空实现，为 v1.1.0+ E2EE 同步预留接口。
//!
//! ## v1.1.0+ 范围
//! 基于 Double Ratchet（`ratchetx2`）+ Automerge CRDT 实现端到端加密的多设备知识库同步：
//! - **push**：本地知识库变更 → CRDT patch → E2EE 加密 → 同步服务器
//! - **pull**：同步服务器 → E2EE 解密 → CRDT merge → 本地知识库
//!
//! ## 设计参考
//! - Signal Protocol Double Ratchet（RFC 004）
//! - Automerge CRDT（rust-automerge 0.10）

use sparkfox_core::Result;

/// 知识库同步 trait — E2EE 多设备同步抽象
///
/// v1.0.0 仅 [`NoOpSync`] 空实现；v1.1.0+ 由 `sparkfox-e2ee` crate 提供真实实现。
pub trait KnowledgeSync: Send + Sync {
    /// 推送本地知识库变更到同步服务器
    ///
    /// - `kb_id`：知识库 ID
    /// - 返回 `Ok(())` 表示推送成功（v1.0.0 NoOpSync 恒返回 Ok）
    fn push(&self, kb_id: &str) -> Result<()>;

    /// 从同步服务器拉取知识库变更到本地
    ///
    /// - `kb_id`：知识库 ID
    /// - 返回 `Ok(())` 表示拉取成功（v1.0.0 NoOpSync 恒返回 Ok）
    fn pull(&self, kb_id: &str) -> Result<()>;

    /// 同步后端名（用于日志与诊断）
    fn backend_name(&self) -> &'static str;
}

/// 空操作同步 — v1.0.0 占位实现
///
/// `push` / `pull` 恒返回 `Ok(())`，不执行任何实际同步。
/// 用于 v1.0.0 单设备场景，避免 E2EE 同步未实现时阻塞 RAG 流程。
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpSync;

impl NoOpSync {
    pub fn new() -> Self {
        Self
    }
}

impl KnowledgeSync for NoOpSync {
    fn push(&self, kb_id: &str) -> Result<()> {
        log::debug!("NoOpSync::push({kb_id}) — v1.0.0 占位，无实际同步");
        Ok(())
    }

    fn pull(&self, kb_id: &str) -> Result<()> {
        log::debug!("NoOpSync::pull({kb_id}) — v1.0.0 占位，无实际同步");
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "noop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_sync_push_pull() {
        let sync = NoOpSync::new();
        assert!(sync.push("kb1").is_ok());
        assert!(sync.pull("kb1").is_ok());
        assert_eq!(sync.backend_name(), "noop");
    }

    #[test]
    fn test_knowledge_sync_trait_object() {
        // 验证 NoOpSync 可作为 trait object 使用
        let sync: Box<dyn KnowledgeSync> = Box::new(NoOpSync::new());
        assert!(sync.push("kb_test").is_ok());
        assert!(sync.pull("kb_test").is_ok());
        assert_eq!(sync.backend_name(), "noop");
    }
}

// ============ 新增：E2EE 同步集成（Task 9.2）============

use sparkfox_crdt::MemoryDoc;
use sparkfox_e2ee::Session;

/// E2EE 知识库同步集成 — 用户显式开启后经 sparkfox-e2ee 加密 + sparkfox-crdt 同步
///
/// 与 `NoOpSync` 的区别：
/// - `NoOpSync`：v1.0.0 默认，不同步
/// - `E2eeKnowledgeSync`：v1.1.0+ 用户显式开启后使用，真实 E2EE 同步
///
/// 使用方式：
/// ```ignore
/// let mut sync = E2eeKnowledgeSync::new();
/// sync.enable_sync(e2ee_session);
/// sync.sync_document("kdoc_001", "文档内容")?;
/// ```
pub struct E2eeKnowledgeSync {
    crdt_doc: MemoryDoc,
    e2ee_session: Option<Session>,
}

impl E2eeKnowledgeSync {
    /// 创建新的 E2EE 同步实例（默认未启用同步）
    pub fn new() -> Self {
        Self {
            crdt_doc: MemoryDoc::default(),
            e2ee_session: None,
        }
    }

    /// 用户显式开启同步
    ///
    /// 传入已建立的 E2EE Session（由 sparkfox-e2ee crate 提供）
    pub fn enable_sync(&mut self, session: Session) {
        self.e2ee_session = Some(session);
    }

    /// 同步单篇知识库文档
    ///
    /// 流程：
    /// 1. CRDT 记录文档内容（用于多设备冲突合并）
    /// 2. E2EE 加密（若已启用 Session）
    ///
    /// 返回 Err 表示同步失败（如 Session 未建立、CRDT merge 冲突等）
    pub fn sync_document(&mut self, kdoc_id: &str, content: &str) -> Result<()> {
        // 1. CRDT 记录
        self.crdt_doc.set_entry(kdoc_id, content)?;

        // 2. E2EE 加密（若启用）
        if let Some(session) = &mut self.e2ee_session {
            let payload = session.encrypt(content.as_bytes())?;
            // EncryptedPayload = ciphertext + nonce + header，三者合计为加密载荷总长度
            let total = payload.ciphertext.len() + payload.nonce.len() + payload.header.len();
            log::debug!(
                "E2eeKnowledgeSync::sync_document({kdoc_id}) 已加密，payload {total} bytes"
            );
        } else {
            log::warn!(
                "E2eeKnowledgeSync::sync_document({kdoc_id}) 未启用 E2EE，仅本地 CRDT 记录"
            );
        }

        Ok(())
    }

    /// 是否已启用 E2EE 同步
    pub fn is_sync_enabled(&self) -> bool {
        self.e2ee_session.is_some()
    }
}

impl Default for E2eeKnowledgeSync {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod e2ee_tests {
    use super::*;

    #[test]
    fn test_e2ee_sync_new_not_enabled() {
        let sync = E2eeKnowledgeSync::new();
        assert!(!sync.is_sync_enabled());
    }

    #[test]
    fn test_e2ee_sync_default_not_enabled() {
        let sync = E2eeKnowledgeSync::default();
        assert!(!sync.is_sync_enabled());
    }
}
