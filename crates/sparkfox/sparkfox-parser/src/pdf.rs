//! PDF 解析 — 基于 `lopdf 0.34` 的纯文本提取
//!
//! 实现 [`Parser`] trait，支持 `.pdf` 扩展名。
//! 安全限制：文件 < 100MB + 页数 < 1000。
//!
//! 当前实现：调用 `lopdf::Document::extract_text` 提取每页文本流，
//! 该 API 内部处理内容流解码 + 字体编码 + Tj/TJ 文本算子。
//! 生产级实现需进一步处理：
//! - 布局分析（多栏、表格、注释）
//! - 加密 PDF 的密码回调（lopdf 支持 `decrypt`，本任务未暴露）
//! - 嵌入图像的 OCR（属于 v0.5.0 多模态范围，本任务不实现）

use std::path::Path;

use lopdf::Document as PdfDocument;

use sparkfox_core::{Error, Result};

use crate::{check_file_size, DocumentMetadata, ParsedDocument, Parser, MAX_PAGE_COUNT};

/// PDF 解析器（基于 `lopdf 0.34`）
pub struct PdfParser;

impl Parser for PdfParser {
    fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        // 1. 文件大小检查（统一入口）
        check_file_size(path)?;

        // 2. 加载 PDF
        let doc = PdfDocument::load(path).map_err(|e| {
            Error::parse(format!("PDF 加载失败: {e}"), "PdfParser::parse")
        })?;

        // 3. 页数检查（lopdf 0.34 的 get_pages 返回 owned BTreeMap<u32, ObjectId>）
        let pages = doc.get_pages();
        let page_count = pages.len();
        if page_count > MAX_PAGE_COUNT {
            return Err(Error::invalid_argument(
                format!(
                    "PDF 页数 {page_count} 超过 {MAX_PAGE_COUNT} 限制: {}",
                    path.display()
                ),
                "PdfParser::parse",
            ));
        }

        // 4. 提取文本（按页号升序，extract_text 接受 &[u32]）
        let mut page_numbers: Vec<u32> = pages.keys().copied().collect();
        page_numbers.sort_unstable();
        let text = doc
            .extract_text(&page_numbers)
            .map_err(|e| Error::parse(format!("PDF 文本提取失败: {e}"), "PdfParser::parse"))?;

        Ok(ParsedDocument {
            text,
            metadata: DocumentMetadata {
                page_count: Some(page_count),
                word_count: None,
                title: None,
                author: None,
            },
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["pdf"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        let parser = PdfParser;
        assert_eq!(parser.supported_extensions(), &["pdf"]);
    }

    #[test]
    fn test_parse_nonexistent_file_returns_err() {
        let parser = PdfParser;
        let result = parser.parse(Path::new("/nonexistent/xyz/abc.pdf"));
        assert!(result.is_err(), "不存在的文件应返回 Err");
        // 应是 Io 错误（来自 check_file_size 的 metadata 调用）
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::Io(_)),
            "期望 Io 错误，实际: {err:?}"
        );
    }

    #[test]
    fn test_parse_empty_file_returns_err() {
        // 空文件不是有效 PDF，lopdf 应返回 Parse 错误
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = PdfParser.parse(tmp.path());
        assert!(result.is_err(), "空文件应返回 Err（非有效 PDF）");
    }

    #[test]
    fn test_parse_corrupt_pdf_returns_err() {
        // 写入一些非 PDF 字节
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"not a pdf content").unwrap();
        let result = PdfParser.parse(tmp.path());
        assert!(result.is_err(), "非 PDF 内容应返回 Err");
    }
}
