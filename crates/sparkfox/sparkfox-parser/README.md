# sparkfox-parser

SparkFox 多格式文档解析（PDF / Word / Excel），纯 Rust 实现，无 Python sidecar。

## Features

- **PDF** — `lopdf 0.34` 纯文本提取
- **Word (.docx)** — `docx-rs 0.4` 段落文本提取
- **Excel (.xlsx/.xls)** — `calamine 0.26` 多 sheet 表格转 TSV
- 安全限制：文件 < 100MB + 解析超时 30s + 页数 < 1000
- `#![forbid(unsafe_code)]` — 全 crate 禁止 unsafe

## Usage

```rust
use std::path::Path;
use sparkfox_parser::{Parser, PdfParser};

let parser = PdfParser;
let doc = parser.parse(Path::new("example.pdf"))?;
println!("text = {}", doc.text);
println!("pages = {:?}", doc.metadata.page_count);
```

带超时的解析：

```rust
use std::path::Path;
use std::sync::Arc;
use sparkfox_parser::{parse_with_timeout, PdfParser, Parser};

let parser: Arc<dyn Parser> = Arc::new(PdfParser);
let doc = parse_with_timeout(parser, Path::new("example.pdf"))?;
```

## License

AGPL-3.0-only（SparkFox original）

## Source

依赖：lopdf (MIT) / docx-rs (MIT) / calamine (MIT) / quick-xml (MIT)。
Parser trait、ParsedDocument 类型与超时包装均为 SparkFox 原创实现。
