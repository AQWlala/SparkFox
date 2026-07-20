#![forbid(unsafe_code)]
//! 图片 OCR — tesseract-rs 集成占位
//!
//! v1.0.0：占位实现，返回 Err 说明 OCR 不可用
//! v1.1.0+：集成 tesseract-rs，用户可选安装 Tesseract
//!
//! NOTICE: Tesseract 本身是 Apache-2.0，tesseract-rs 是 MIT

use std::path::Path;

use sparkfox_core::{Error, Result};

use crate::{ParsedDocument, Parser};

/// 图片 OCR 解析器（v1.0.0 占位）
///
/// v1.0.0 不依赖 tesseract-rs（避免引入大依赖），所有调用返回
/// `Error::internal` 说明 OCR 不可用；v1.1.0+ 计划集成 tesseract-rs，
/// 在用户已安装 Tesseract 时启用真实 OCR。
pub struct OcrParser;

impl OcrParser {
    /// 创建一个新的 OCR 解析器
    pub fn new() -> Self {
        Self
    }

    /// 检测 Tesseract 是否可用
    ///
    /// v1.0.0 永远返回 `false`（未集成 tesseract-rs）
    /// v1.1.0+ 检测 tesseract-rs 是否可加载 + Tesseract 二进制是否在 PATH 中
    pub fn is_available(&self) -> bool {
        false
    }
}

impl Default for OcrParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser for OcrParser {
    fn parse(&self, _path: &Path) -> Result<ParsedDocument> {
        Err(Error::internal(
            "Tesseract 未安装，OCR 不可用（v1.0.0 占位，v1.1.0+ 实现）",
        ))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["png", "jpg", "jpeg"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_ocr_parser_new_default_eq() {
        let a = OcrParser::new();
        let b = OcrParser::default();
        // 二者都是单元结构体，is_available 应一致
        assert_eq!(a.is_available(), b.is_available());
    }

    #[test]
    fn test_is_available_v1_returns_false() {
        let parser = OcrParser::new();
        assert!(!parser.is_available(), "v1.0.0 占位实现 is_available 必须为 false");
    }

    #[test]
    fn test_supported_extensions() {
        let parser = OcrParser::new();
        let exts = parser.supported_extensions();
        assert!(exts.contains(&"png"), "应支持 png");
        assert!(exts.contains(&"jpg"), "应支持 jpg");
        assert!(exts.contains(&"jpeg"), "应支持 jpeg");
    }

    #[test]
    fn test_parse_returns_err_when_unavailable() {
        let parser = OcrParser::new();
        let result = parser.parse(Path::new("dummy.png"));
        assert!(result.is_err(), "v1.0.0 占位 parse 必须返回 Err");
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("Tesseract") || msg.contains("OCR"),
            "错误消息应说明 OCR 不可用，实际: {msg}"
        );
        assert!(matches!(err, Error::Internal(_)), "应返回 Internal 错误，实际: {err:?}");
    }
}
