//! LLM 流式响应封装 — 基于 tokio mpsc channel
//!
//! Task 7.2：为 [`LlmProvider::stream_complete`](crate::LlmProvider::stream_complete)
//! 提供基于 `tokio::sync::mpsc` 的流式响应封装。
//!
//! # 设计
//! - Provider 持有 `Sender<Result<String>>`，逐 token 推送
//! - 消费方持有 [`LlmStream`]，通过 [`LlmStream::next`] 拉取
//! - `Sender` drop 时，`next()` 返回 `None`，表示流结束
//!
//! # 用法
//! ```no_run
//! # use sparkfox_core::Result;
//! # use sparkfox_llm::LlmStream;
//! # async fn demo() -> Result<()> {
//! let (tx, mut stream) = LlmStream::channel(16);
//! tx.send(Ok("hello".into())).await.ok();
//! tx.send(Ok(" world".into())).await.ok();
//! drop(tx); // 显式关闭，表示流结束
//!
//! while let Some(chunk) = stream.next().await {
//!     let chunk = chunk?;
//!     println!("{chunk}");
//! }
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

use tokio::sync::mpsc;

use sparkfox_core::Result;

/// LLM 流式响应封装
///
/// 包装 `tokio::sync::mpsc::Receiver`，提供简化的 `next()` 接口。
/// 通过 [`LlmStream::channel`] 创建 (sender, stream) 对。
pub struct LlmStream {
    rx: mpsc::Receiver<Result<String>>,
}

impl LlmStream {
    /// 创建 (sender, stream) 对
    ///
    /// # 参数
    /// - `buffer`: channel 缓冲区大小（建议 8-32，过小可能导致 Provider 背压）
    ///
    /// # 返回
    /// `(Sender, LlmStream)` — Provider 持有 sender，消费方持有 stream
    pub fn channel(buffer: usize) -> (mpsc::Sender<Result<String>>, Self) {
        let (tx, rx) = mpsc::channel(buffer);
        (tx, Self { rx })
    }

    /// 拉取下一段流式输出
    ///
    /// # 返回
    /// - `Some(Ok(chunk))`: 成功拉取一段文本
    /// - `Some(Err(e))`: Provider 端发生错误（透传）
    /// - `None`: 流结束（sender 已 drop）
    pub async fn next(&mut self) -> Option<Result<String>> {
        self.rx.recv().await
    }
}

// ============================================================================
// 测试 — 验证 LlmStream 基本行为
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sparkfox_core::Error;

    /// 基本收发 — 推送 2 段文本，按顺序拉取
    #[tokio::test]
    async fn test_stream_send_recv() {
        let (tx, mut stream) = LlmStream::channel(8);
        tx.send(Ok("hello".into())).await.expect("send 1 失败");
        tx.send(Ok(" world".into())).await.expect("send 2 失败");
        drop(tx); // 关闭 sender

        let chunk1 = stream.next().await.expect("应收到第 1 段");
        assert_eq!(chunk1.expect("第 1 段应为 Ok"), "hello");

        let chunk2 = stream.next().await.expect("应收到第 2 段");
        assert_eq!(chunk2.expect("第 2 段应为 Ok"), " world");

        assert!(stream.next().await.is_none(), "sender drop 后应返回 None");
    }

    /// sender drop 后 next() 返回 None — 流结束信号
    #[tokio::test]
    async fn test_stream_ends_on_sender_drop() {
        let (tx, mut stream) = LlmStream::channel(8);
        drop(tx);
        assert!(
            stream.next().await.is_none(),
            "sender drop 后 next() 应立即返回 None"
        );
    }

    /// 错误透传 — Provider 端发送 Err，消费方应收到
    #[tokio::test]
    async fn test_stream_error_propagation() {
        let (tx, mut stream) = LlmStream::channel(8);
        tx.send(Err(Error::llm("模拟 LLM 错误")))
            .await
            .expect("send Err 失败");
        drop(tx);

        let result = stream.next().await.expect("应收到错误");
        assert!(result.is_err(), "应透传 Err");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("模拟 LLM 错误"));
    }

    /// 空 stream（立即 drop sender）— next() 立即返回 None
    #[tokio::test]
    async fn test_empty_stream() {
        let (_tx, mut stream) = LlmStream::channel(8);
        // 不发送任何数据，直接 drop sender（_tx 在作用域结束时 drop）
        drop(_tx);
        assert!(stream.next().await.is_none(), "空 stream 应立即结束");
    }
}
