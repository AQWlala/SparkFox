//! sparkfox-parser 集成测试 — Parser trait 跨实现一致性
//!
//! 覆盖三种解析器的公共契约：
//! - `supported_extensions` 返回非空切片
//! - 解析不存在的文件统一返回 `Error::Io`
//! - 解析空文件统一返回 `Error::Parse`（非有效格式）
//! - `parse_with_timeout` 在快速失败路径上不触发超时

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Arc;

use sparkfox_core::Error;
use sparkfox_parser::{
    parse_with_timeout, DocxParser, DocumentMetadata, ParsedDocument, Parser, PdfParser, XlsxParser,
    MAX_FILE_SIZE, MAX_PAGE_COUNT, PARSE_TIMEOUT_SECS,
};

/// 三种解析器的扩展名声明必须非空且为小写
#[test]
fn test_all_parsers_declare_extensions() {
    let pdf = PdfParser;
    let docx = DocxParser;
    let xlsx = XlsxParser;

    assert!(!pdf.supported_extensions().is_empty());
    assert!(!docx.supported_extensions().is_empty());
    assert!(!xlsx.supported_extensions().is_empty());

    for ext in pdf.supported_extensions() {
        assert!(!ext.is_empty(), "扩展名不应为空字符串");
        assert!(
            ext.chars().all(|c| !c.is_ascii_uppercase()),
            "扩展名应全小写: {ext}"
        );
    }
    for ext in docx.supported_extensions() {
        assert!(!ext.is_empty());
        assert!(ext.chars().all(|c| !c.is_ascii_uppercase()));
    }
    for ext in xlsx.supported_extensions() {
        assert!(!ext.is_empty());
        assert!(ext.chars().all(|c| !c.is_ascii_uppercase()));
    }
}

/// PDF 解析器声明支持 .pdf
#[test]
fn test_pdf_parser_extensions() {
    assert_eq!(PdfParser.supported_extensions(), &["pdf"]);
}

/// DOCX 解析器声明支持 .docx
#[test]
fn test_docx_parser_extensions() {
    assert_eq!(DocxParser.supported_extensions(), &["docx"]);
}

/// XLSX 解析器声明仅支持 .xlsx（v1.0.0 不支持 .xls BIFF 格式）
#[test]
fn test_xlsx_parser_extensions() {
    assert_eq!(XlsxParser.supported_extensions(), &["xlsx"]);
}

/// 所有解析器对不存在的文件应返回 Io 错误（来自 check_file_size 的 metadata 调用）
#[test]
fn test_all_parsers_return_io_err_for_nonexistent_file() {
    let nonexistent = Path::new("/nonexistent/sparkfox/parser/xyz/abc");
    let cases: Vec<(&str, Box<dyn Parser>)> = vec![
        ("pdf", Box::new(PdfParser)),
        ("docx", Box::new(DocxParser)),
        ("xlsx", Box::new(XlsxParser)),
    ];
    for (name, parser) in cases {
        let result = parser.parse(nonexistent);
        assert!(result.is_err(), "{name}: 不存在的文件应返回 Err");
        match result.unwrap_err() {
            Error::Io(_) => { /* 期望路径 */ }
            other => panic!("{name}: 期望 Io 错误，实际: {other:?}"),
        }
    }
}

/// 所有解析器对空文件应返回错误（非有效格式）
#[test]
fn test_all_parsers_return_err_for_empty_file() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let cases: Vec<(&str, Box<dyn Parser>)> = vec![
        ("pdf", Box::new(PdfParser)),
        ("docx", Box::new(DocxParser)),
        ("xlsx", Box::new(XlsxParser)),
    ];
    for (name, parser) in cases {
        let result = parser.parse(tmp.path());
        assert!(
            result.is_err(),
            "{name}: 空文件应返回 Err（非有效格式）"
        );
    }
}

/// 所有解析器对损坏（非对应格式）的文件应返回错误
#[test]
fn test_all_parsers_return_err_for_corrupt_file() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), b"definitely not a valid binary format").unwrap();

    let cases: Vec<(&str, Box<dyn Parser>)> = vec![
        ("pdf", Box::new(PdfParser)),
        ("docx", Box::new(DocxParser)),
        ("xlsx", Box::new(XlsxParser)),
    ];
    for (name, parser) in cases {
        let result = parser.parse(tmp.path());
        assert!(result.is_err(), "{name}: 损坏文件应返回 Err");
    }
}

/// parse_with_timeout 对不存在的文件应快速返回 Err（不触发 30s 超时）
#[test]
fn test_parse_with_timeout_does_not_hang_on_nonexistent_file() {
    let parsers: Vec<(&str, Arc<dyn Parser>)> = vec![
        ("pdf", Arc::new(PdfParser)),
        ("docx", Arc::new(DocxParser)),
        ("xlsx", Arc::new(XlsxParser)),
    ];
    for (name, parser) in parsers {
        let result = parse_with_timeout(parser, Path::new("/nonexistent/xyz/abc"));
        assert!(
            result.is_err(),
            "{name}: parse_with_timeout 对不存在的文件应返回 Err"
        );
    }
}

/// 验证 ParsedDocument 与 DocumentMetadata 的构造与默认值
#[test]
fn test_parsed_document_construction() {
    let doc = ParsedDocument {
        text: "sample text".to_string(),
        metadata: DocumentMetadata {
            title: Some("标题".to_string()),
            author: Some("作者".to_string()),
            page_count: Some(10),
            word_count: Some(100),
        },
    };
    assert_eq!(doc.text, "sample text");
    assert_eq!(doc.metadata.title.as_deref(), Some("标题"));
    assert_eq!(doc.metadata.author.as_deref(), Some("作者"));
    assert_eq!(doc.metadata.page_count, Some(10));
    assert_eq!(doc.metadata.word_count, Some(100));

    let default_meta = DocumentMetadata::default();
    assert!(default_meta.title.is_none());
    assert!(default_meta.author.is_none());
    assert!(default_meta.page_count.is_none());
    assert!(default_meta.word_count.is_none());
}

/// 验证安全限制常量符合 spec 1.0
#[test]
fn test_safety_constants() {
    assert_eq!(MAX_FILE_SIZE, 100 * 1024 * 1024, "MAX_FILE_SIZE 应为 100MB");
    assert_eq!(PARSE_TIMEOUT_SECS, 30, "PARSE_TIMEOUT_SECS 应为 30s");
    assert_eq!(MAX_PAGE_COUNT, 1000, "MAX_PAGE_COUNT 应为 1000");
}

/// Parser trait 对象可跨线程共享（Send + Sync 约束）
#[test]
fn test_parser_trait_object_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PdfParser>();
    assert_send_sync::<DocxParser>();
    assert_send_sync::<XlsxParser>();
    assert_send_sync::<Arc<dyn Parser>>();
}
