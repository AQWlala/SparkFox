//! Excel 解析 — 基于 `calamine 0.26` 的多 sheet 表格转 TSV
//!
//! 实现 [`Parser`] trait，仅支持 `.xlsx` 扩展名（Office Open XML 格式）。
//! v1.0.0 不支持旧版 `.xls`（BIFF 二进制格式，需 calamine `Xls` reader 或其他库）。
//! 安全限制：文件 < 100MB。
//!
//! calamine 0.26 API 说明：
//! - `Data` 是单元格值的枚举类型（`Int`/`Float`/`String`/`Bool`/
//!   `DateTime(ExcelDateTime)`/`DateTimeIso(String)`/`DurationIso(String)`/
//!   `Error`/`Empty`）
//! - `DataType` 是 trait（提供 `as_string`/`as_i64`/`as_f64` 等方法），
//!   `Data` 实现了 `DataType`
//!
//! 输出格式：每个 sheet 以 `[Sheet: <name>]\n` 起始，行内单元格以 `\t` 分隔，
//! 行末 `\n` 结束。空单元格输出为空字符串。

use std::path::Path;

use calamine::{open_workbook, Data, Reader, Xlsx};

use sparkfox_core::{Error, Result};

use crate::{check_file_size, DocumentMetadata, ParsedDocument, Parser};

/// Excel 解析器（基于 `calamine 0.26`）
pub struct XlsxParser;

impl Parser for XlsxParser {
    fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        // 1. 文件大小检查
        check_file_size(path)?;

        // 2. 打开工作簿（Xlsx reader 仅支持 .xlsx）
        let mut workbook: Xlsx<_> = open_workbook(path).map_err(|e| {
            Error::parse(format!("xlsx 加载失败: {e}"), "XlsxParser::parse")
        })?;

        // 3. 遍历所有 sheet，按 TSV 格式拼接
        let mut text = String::new();
        let sheet_names = workbook.sheet_names().to_vec();
        for sheet_name in sheet_names {
            text.push_str(&format!("[Sheet: {sheet_name}]\n"));
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                for row in range.rows() {
                    let cells: Vec<String> = row.iter().map(format_cell).collect();
                    text.push_str(&cells.join("\t"));
                    text.push('\n');
                }
            }
            // worksheet_range 失败时跳过该 sheet（生产级可记录 warn）
        }

        Ok(ParsedDocument {
            text,
            metadata: DocumentMetadata::default(),
        })
    }

    fn supported_extensions(&self) -> &[&str] {
        // v1.0.0 仅支持 .xlsx（Office Open XML）。
        // 旧版 .xls（BIFF 格式）需 calamine Xls reader，v1.0.0 不支持。
        &["xlsx"]
    }
}

/// 将 calamine `Data` 单元格格式化为字符串
///
/// 覆盖 calamine 0.26 的所有 `Data` 变体：
/// - `Int(i64)` / `Float(f64)` / `Bool(bool)` —— 用 `to_string()`
/// - `String(String)` —— 直接 clone
/// - `DateTime(ExcelDateTime)` —— 用 `Display` trait（输出 float 值）
/// - `DateTimeIso(String)` / `DurationIso(String)` —— 直接 clone
/// - `Error(CellErrorType)` / `Empty` —— 输出空字符串
fn format_cell(c: &Data) -> String {
    match c {
        Data::String(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => f.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(d) => d.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Empty => String::new(),
        Data::Error(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        let parser = XlsxParser;
        assert_eq!(parser.supported_extensions(), &["xlsx"]);
    }

    #[test]
    fn test_parse_nonexistent_file_returns_err() {
        let parser = XlsxParser;
        let result = parser.parse(Path::new("/nonexistent/xyz/abc.xlsx"));
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
        let result = XlsxParser.parse(tmp.path());
        assert!(result.is_err(), "空文件应返回 Err（非有效 xlsx）");
    }

    #[test]
    fn test_parse_corrupt_xlsx_returns_err() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"not an xlsx content").unwrap();
        let result = XlsxParser.parse(tmp.path());
        assert!(result.is_err(), "非 xlsx 内容应返回 Err");
    }
}
