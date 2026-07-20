//! SparkFox Parser — 多格式文档解析（PDF/Word/Excel）
//!
//! 纯 Rust 实现，不引入 Python sidecar。
//! 安全限制：
//! - 文件大小 < 100MB（[`MAX_FILE_SIZE`]）
//! - 解析超时 30s（[`PARSE_TIMEOUT_SECS`]，见 [`parse_with_timeout`]）
//! - 页数 < 1000（[`MAX_PAGE_COUNT`]）
//!
//! 全 crate 严格遵守 `#![forbid(unsafe_code)]`，无任何 unsafe 代码。

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use sparkfox_core::{Error, Result};

pub mod docx;
pub mod ocr;
pub mod pdf;
pub mod table;
pub mod xlsx;

pub use docx::DocxParser;
pub use ocr::OcrParser;
pub use pdf::PdfParser;
pub use table::{extract_tables, Table};
pub use xlsx::XlsxParser;

/// 最大文件大小：100MB
pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;
/// 解析超时：30 秒（用于 [`parse_with_timeout`]
pub const PARSE_TIMEOUT_SECS: u64 = 30;
/// 最大页数限制：1000 页（PDF 等多页文档）
pub const MAX_PAGE_COUNT: usize = 1000;

/// 文档解析 trait
///
/// 所有具体解析器（PDF/Word/Excel）实现此 trait。
/// `Send + Sync` 约束允许解析器在多线程环境中共享，并支持
/// [`parse_with_timeout`] 在子线程中执行解析。
pub trait Parser: Send + Sync {
    /// 解析指定路径的文档，返回提取的文本与元数据
    fn parse(&self, path: &Path) -> Result<ParsedDocument>;
    /// 返回此解析器支持的文件扩展名列表（小写，不含 `.`）
    fn supported_extensions(&self) -> &[&str];
}

/// 解析后的文档
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    /// 提取的纯文本内容（按页/段落顺序拼接）
    pub text: String,
    /// 文档元数据
    pub metadata: DocumentMetadata,
}

/// 文档元数据
#[derive(Debug, Clone, Default)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: Option<usize>,
    pub word_count: Option<usize>,
}

/// 初始化（日志）
pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-parser v{} initialized", env!("CARGO_PKG_VERSION"));
}

/// 校验文件大小是否在 [`MAX_FILE_SIZE`] 限制内
///
/// 各 parser 在 `parse` 入口处调用此函数以统一执行大小检查。
/// 文件不存在或不可访问时返回 `Error::Io`，超限时返回
/// `Error::InvalidArgument`。
pub fn check_file_size(path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(path).map_err(Error::from)?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(Error::invalid_argument(
            format!(
                "文件 {} 超过 100MB 限制（{} bytes）",
                path.display(),
                metadata.len()
            ),
            "check_file_size",
        ));
    }
    Ok(())
}

/// 带超时的解析（[`PARSE_TIMEOUT_SECS`] 秒）
///
/// 使用 `Arc<dyn Parser>` 在子线程中执行解析，主线程通过 mpsc channel
/// 接收结果。若超时则返回 `Error::Internal`，子线程被分离（detached），
/// 它会继续运行到解析完成或 panic（panic 不会影响主线程，主线程会通过
/// `RecvTimeoutError::Disconnected` 检测到）。
///
/// 注意：本函数避免使用 `unsafe`，符合 `#![forbid(unsafe_code)]` 约束。
/// spec 1.0 原版使用裸指针 + `unsafe` 在线程间传递 `&dyn Parser`，本实现
/// 改用 `Arc<dyn Parser>` 在线程间共享所有权，语义等价且无需 unsafe。
/// 生产级实现可进一步改用 `tokio::spawn` + `tokio::time::timeout`。
///
/// # 示例
///
/// ```no_run
/// use std::path::Path;
/// use std::sync::Arc;
/// use sparkfox_parser::{parse_with_timeout, PdfParser, Parser};
///
/// let parser: Arc<dyn Parser> = Arc::new(PdfParser);
/// let doc = parse_with_timeout(parser, Path::new("example.pdf"))?;
/// # Ok::<(), sparkfox_core::Error>(())
/// ```
pub fn parse_with_timeout(parser: Arc<dyn Parser>, path: &Path) -> Result<ParsedDocument> {
    let (tx, rx) = std::sync::mpsc::channel::<Result<ParsedDocument>>();
    let path_buf = path.to_path_buf();
    std::thread::spawn(move || {
        // parser 是 Arc，移动到子线程；send 失败表示主线程已超时返回，忽略错误
        let result = parser.parse(&path_buf);
        let _ = tx.send(result);
    });
    match rx.recv_timeout(Duration::from_secs(PARSE_TIMEOUT_SECS)) {
        Ok(result) => result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(Error::internal(format!(
            "解析超时 {PARSE_TIMEOUT_SECS}s: {}",
            path.display()
        ))),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(Error::internal(format!(
            "解析线程异常终止（panic 或 channel 关闭）: {}",
            path.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(MAX_FILE_SIZE, 100 * 1024 * 1024);
        assert_eq!(PARSE_TIMEOUT_SECS, 30);
        assert_eq!(MAX_PAGE_COUNT, 1000);
    }

    #[test]
    fn test_check_file_size_nonexistent_returns_err() {
        // 不存在的路径应返回 Io 错误（不是 InvalidArgument）
        let result = check_file_size(Path::new("/nonexistent/path/xyz/abc.txt"));
        assert!(result.is_err(), "不存在的文件应返回 Err");
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::Io(_)),
            "期望 Io 错误，实际: {err:?}"
        );
    }

    #[test]
    fn test_parsed_document_default_metadata() {
        let doc = ParsedDocument {
            text: String::new(),
            metadata: DocumentMetadata::default(),
        };
        assert!(doc.text.is_empty());
        assert!(doc.metadata.title.is_none());
        assert!(doc.metadata.author.is_none());
        assert!(doc.metadata.page_count.is_none());
        assert!(doc.metadata.word_count.is_none());
    }

    #[test]
    fn test_parsed_document_clone() {
        let doc = ParsedDocument {
            text: "hello".to_string(),
            metadata: DocumentMetadata {
                title: Some("t".to_string()),
                author: None,
                page_count: Some(3),
                word_count: Some(5),
            },
        };
        let cloned = doc.clone();
        assert_eq!(cloned.text, "hello");
        assert_eq!(cloned.metadata.title.as_deref(), Some("t"));
        assert_eq!(cloned.metadata.page_count, Some(3));
    }

    /// 验证 parse_with_timeout 对不存在的文件返回 Err（不超时）
    #[test]
    fn test_parse_with_timeout_nonexistent_file() {
        let parser: Arc<dyn Parser> = Arc::new(PdfParser);
        let result = parse_with_timeout(parser, Path::new("/nonexistent/xyz.pdf"));
        assert!(result.is_err(), "不存在的文件应返回 Err 而非超时");
    }

    /// 验证 parse_with_timeout 对超时路径的处理（用一个会快速失败的路径）
    #[test]
    fn test_parse_with_timeout_fast_fail() {
        // 用 DocxParser 解析一个不存在的文件，应快速返回 Err
        let parser: Arc<dyn Parser> = Arc::new(DocxParser);
        let result = parse_with_timeout(parser, Path::new("/nonexistent/xyz.docx"));
        assert!(result.is_err());
    }
}
