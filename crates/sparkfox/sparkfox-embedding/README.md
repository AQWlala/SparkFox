# sparkfox-embedding

> SparkFox 嵌入与重排（candle-transformers）— bge-small-zh / bge-large-zh / bge-reranker-v2-m3

## 功能

- **嵌入（Embedding）**：基于 candle-transformers 0.8 推理，默认 `bge-small-zh-v1.5`（120MB），可选 `bge-large-zh-v1.5`（1.2GB）
- **重排（Reranker）**：`bge-reranker-v2-m3`（560MB），对召回结果二次精排
- **CLIP 嵌入**：图文多模态嵌入（`ClipEmbedder`）
- **模型下载器**：从 HuggingFace Hub 拉取并缓存到本地（断点续传 + SHA256 校验）
- **查询缓存**：`QueryCache` 缓存重复 query 的嵌入结果，降低延迟
- **配置驱动**：`EmbeddingConfig` 支持运行时切换模型 / 维度 / 设备

## 架构

```
┌──────────────────────────────────────────┐
│           sparkfox-knowledge             │
│   (RagEngine 通过 Embedder trait 解耦)    │
└─────────────────┬────────────────────────┘
                  │ Embedder trait
                  ▼
┌──────────────────────────────────────────┐
│           sparkfox-embedding             │
│  ┌────────────┐  ┌────────────────────┐  │
│  │ BgeEmbedder│  │ BgeReranker        │  │
│  │  (candle)  │  │   (candle)         │  │
│  └─────┬──────┘  └─────────┬──────────┘  │
│        │                     │            │
│        ▼                     ▼            │
│  ┌──────────────────────────────────┐    │
│  │ QueryCache (LRU + SHA256)        │    │
│  └──────────────────────────────────┘    │
└─────────────────┬────────────────────────┘
                  │ ModelVariant + downloader
                  ▼
            HuggingFace Hub
```

**依赖**：`sparkfox-core` / `sparkfox-store` / `candle-core` / `candle-nn` / `candle-transformers` / `tokenizers` / `hf-hub`

## 使用

```rust
use sparkfox_embedding::{BgeEmbedder, Embedder, EmbeddingConfig};

let cfg = EmbeddingConfig::default(); // bge-small-zh-v1.5
let embedder = BgeEmbedder::new(cfg)?;

// 单条嵌入（L2 归一化）
let vec = embedder.embed("SparkFox 是 Tauri 2 桌面 AI Agent")?;
assert_eq!(vec.len(), embedder.dim());

// 批量嵌入
let batch = embedder.embed_batch(&["hello", "world"])?;
assert_eq!(batch.len(), 2);
```

## 测试

```bash
# 单元测试（lib）
cargo test -p sparkfox-embedding --lib

# PoC-3 集成测试（真实 bge 模型，模型缺失时自动 skip）
cargo test -p sparkfox-embedding --test poc3_bge

# 缓存集成测试
cargo test -p sparkfox-embedding --test cache_integration
```

PoC-3 验收门槛：
- 单条嵌入 < 50ms
- 1000 条批量 < 30s
- 与 Python sentence-transformers cosine > 0.99

## 安全约束

- `#![deny(unsafe_code)]`（非 `forbid`）—— candle-nn 的 `VarBuilder::from_mmaped_safetensors` 是 unsafe 的（mmap + FFI），仅在 mmap 加载处局部 `#[allow(unsafe_code)]`
- 模型权重下载后做 SHA256 校验（防供应链篡改）

## 许可证

AGPL-3.0-only，详见工作区根 `LICENSE`。

## 致谢

- [candle](https://github.com/huggingface/candle)（HuggingFace，Apache-2.0）— Rust 纯实现 tensor 推理框架
- [BAAI/bge-small-zh-v1.5](https://huggingface.co/BAAI/bge-small-zh-v1.5)（MIT）— 中文嵌入模型
- [BAAI/bge-reranker-v2-m3](https://huggingface.co/BAAI/bge-reranker-v2-m3)（MIT）— 多语言重排模型
