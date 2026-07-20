//! Word 解析 — 基于 `docx-rs 0.4` 的段落文本提取
//!
//! 实现 [`Parser`] trait，支持 `.docx` 扩展名（不支持旧版 `.doc` 二进制格式）。
//! 安全限制：文件 < 100MB。
//!
//! docx-rs 0.4 API 说明：
//! - `read_docx(buf: &[u8])` 接受字节切片（非 File），需先 `std::fs::read` 整个文件
//! - 文档树结构：`Docx.document.children: Vec<DocumentChild>`
//!   - `DocumentChild::Paragraph(Box<Paragraph>)`
//!   - `Paragraph.children: Vec<ParagraphChild>`
//!     - `ParagraphChild::Run(Box<Run>)`
//!     - `Run.children: Vec<RunChild>`
//!       - `RunChild::Text(Text)` —— 文本节点
//!       - `Text.text: String`
//!
//! 当前实现遍历 `Paragraph → Run → Text`，段落间以 `\n` 分隔。
//! 生产级实现需进一步处理：
//! - 表格（`DocumentChild::Table` → TSV）
//! - 列表样式与编号
//! - 嵌入对象（图片、OLE）
//! - 修订标记（track changes，`Insert`/`Delete` 节点）
//! - 旧版 `.doc` 二进制格式（需 libole2 或纯 Rust 实现，超出本任务范围）

use std::path::Path;

use docx_rs::{read_docx, DocumentChild, ParagraphChild, RunChild};

use sparkfox_core::{Error, Result};

use crate::{check_file_size, DocumentMetadata, ParsedDocument, Parser};

/// Word 解析器（基于 `docx-rs 0.4`）
pub struct DocxParser;

impl Parser for DocxParser {
    fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        // 1. 文件大小检查
        check_file_size(path)?;

        // 2. 读取文件到字节切片（docx-rs 0.4 的 read_docx 接受 &[u8]）
        let bytes = std::fs::read(path).map_err(Error::from)?;
        let docx = read_docx(&bytes).map_err(|e| {
            Error::parse(format!("docx 加载失败: {e}"), "DocxParser::parse")
        })?;

        // 3. 遍历文档树：Document → Paragraph → Run → Text
        let mut text = String::new();
        for child in docx.document.children {
            if let DocumentChild::Paragraph(p) = child {
                // p 是 Box<Paragraph>，自动 deref
                for c in p.children {
                    if let ParagraphChild::Run(r) = c {
                        // r 是 Box<Run>，自动 deref
                        for rc in r.children {
                            if let RunChild::Text(t) = rc {
                                text.push_str(&t.text);
                            }
                            // Tab/Break 等其他 RunChild 当前跳过（生产级可扩展）
                        }
                    }
                }
                text.push('\n');
            }
            // Table / SectionProperties 等节点当前跳过
        }

        // 4. 计算字数（按空白分隔）
        let word_count = text.split_whitespace().count();

        Ok(ParsedDocument {
            text,
            metadata: DocumentMetadata {
                word_count: Some(word_count),
                ..Default::default()
            },
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        &["docx"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        let parser = DocxParser;
        assert_eq!(parser.supported_extensions(), &["docx"]);
    }

    #[test]
    fn test_parse_nonexistent_file_returns_err() {
        let parser = DocxParser;
        let result = parser.parse(Path::new("/nonexistent/xyz/abc.docx"));
        assert!(result.is_err(), "不存在的文件应返回 Err");
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::Io(_)),
            "期望 Io 错误，实际: {err:?}"
        );
    }

    #[test]
    fn test_parse_empty_file_returns_err() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = DocxParser.parse(tmp.path());
        assert!(result.is_err(), "空文件应返回 Err（非有效 docx）");
    }

    #[test]
    fn test_parse_corrupt_docx_returns_err() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"not a docx content").unwrap();
        let result = DocxParser.parse(tmp.path());
        assert!(result.is_err(), "非 docx 内容应返回 Err");
    }
}
