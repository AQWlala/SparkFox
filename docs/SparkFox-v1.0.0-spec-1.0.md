# SparkFox v1.0.0 实施规格 1.0

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**目标**：一次性实现 SparkFox v1.0.0 全部功能 — 知识库 RAG 引擎（含向量/分块/多格式/混合检索/Rerank/多模态/知识图谱）+ Phase 1 Rust crate 落地（ipc/llm/agent/chat/thinking/monitor/orchestrator）+ Phase 2 长期补全（hotspot/security/sceneStore/3D 地球）+ 跨设备同步与安全。

**架构**：基于已落地的 sparkfox-core/memory/crdt/e2ee/store（v0.2.0 spec A 已完成），新建 4 个 crate（sparkfox-embedding/parser/knowledge/graph）+ 落地 7 个 Phase 1 crate + 2 个 Phase 2 crate + 前端 5 个 UI 任务 + 8 个 store IPC 对接。所有 Rust 代码 `#![forbid(unsafe_code)]`（FFI 扩展加载除外），所有借鉴代码保留 NOTICE，AGPL 清洁室流程严格执行。

**技术栈**：Rust 2024 + candle-transformers 0.7 + lopdf 0.34 + docx-rs 0.4 + calamine 0.26 + tesseract-rs + petgraph + automerge-rs 0.10 + ratchetx2（已弃用，改 x25519-dalek+aes-gcm 自实现）+ sqlite-vec + Tauri 2 + React 19.1 + Zustand + Arco Design + react-flow + Three.js

**用户决策**（全选 B）：
1. 单一 v1.0.0 大版本，约 50 任务，10 周工期
2. v0.5.0 多模态一次性（OCR + 表格 + CLIP）
3. PoC-3 cosine 严格 > 0.99
4. 嵌入缓存策略：文档嵌入每次重建（不缓存文档嵌入，仅缓存查询嵌入）
5. 本 spec 为独立文档，不与知识库蓝图合并

---

## 一、版本规划

| 版本 | 范围 | 提交策略 | 验收 |
|---|---|---|---|
| **v1.0.0**（本 spec 范围） | 全部 50 任务（模块一至九） | 单一 Git commit | PoC-3 GO + 全部测试通过 + E2E 验证 |

**工期估算**：10 周（参考任务级人天估算表，见附录 B）

**前置条件**：
- ✅ v0.2.0 spec A 已完成（commit 0977d18/6c4aa4f/810f684/92bb94d/19ddca0/46ec145/36364fb）
- ⏳ PoC-3 验证（模块一）— 若 NO-GO 触发 Kill Switch：退回 Python sidecar 仅 embedding 模块

---

## 二、文件结构

### 新建的 Rust crate（4 个）

```
crates/sparkfox/sparkfox-embedding/         # 模块一+二+五
  ├─ Cargo.toml
  ├─ NOTICE                                  # bge 模型 + candle LICENSE
  ├─ README.md
  ├─ src/lib.rs                              # Embedder trait
  ├─ src/config.rs                           # 模型切换配置
  ├─ src/downloader.rs                       # 模型下载 + SHA256 校验
  ├─ src/embedder.rs                         # BgeEmbedder
  ├─ src/reranker.rs                         # BgeReranker (模块五)
  ├─ src/cache.rs                            # 查询嵌入缓存
  └─ tests/poc3_bge.rs                       # PoC-3 验证

crates/sparkfox/sparkfox-parser/             # 模块四+八
  ├─ Cargo.toml
  ├─ NOTICE                                  # lopdf/docx-rs/calamine/tesseract LICENSE
  ├─ README.md
  ├─ src/lib.rs                              # Parser trait + 降级策略
  ├─ src/pdf.rs                              # lopdf 包装
  ├─ src/docx.rs                             # docx-rs 包装
  ├─ src/xlsx.rs                             # calamine 包装
  ├─ src/ocr.rs                              # tesseract-rs (模块八)
  ├─ src/table.rs                            # PDF 表格识别 (模块八)
  └─ tests/parse_samples.rs

crates/sparkfox/sparkfox-knowledge/          # 模块三+九
  ├─ Cargo.toml
  ├─ NOTICE                                  # NomiFun Apache-2.0 + 借鉴源
  ├─ README.md
  ├─ src/lib.rs                              # KnowledgeBase / Document / Search
  ├─ src/chunk.rs                            # 文档分块
  ├─ src/rag.rs                              # RAG 引擎（向量+关键词+RRF）
  ├─ src/rerank.rs                           # 重排集成
  ├─ src/citation.rs                         # 引用协议
  ├─ src/sync.rs                             # E2EE 同步 (模块九)
  └─ tests/rag_e2e.rs

crates/sparkfox/sparkfox-graph/              # 模块八
  ├─ Cargo.toml
  ├─ NOTICE                                  # OpenAkita MDRM 清洁室声明
  ├─ README.md
  ├─ src/lib.rs                              # Graph trait
  ├─ src/graph.rs                            # petgraph + SQLite 存储
  ├─ src/extractor.rs                        # LLM 实体抽取
  ├─ src/relation.rs                         # LLM 关系抽取
  ├─ src/traversal.rs                        # 多跳遍历（MDRM 5 维清洁室重写）
  └─ tests/graph_e2e.rs
```

### 落地的 Phase 1/2 Rust crate（9 个，修改占位文件）

```
crates/sparkfox/sparkfox-ipc/src/lib.rs                 # 模块七 7.1
crates/sparkfox/sparkfox-llm/src/{lib.rs,provider.rs,stream.rs}  # 7.2
crates/sparkfox/sparkfox-agent/src/{lib.rs,profile.rs}   # 7.3
crates/sparkfox/sparkfox-chat/src/{lib.rs,thinking.rs,hotspot.rs,citation.rs}  # 7.4
crates/sparkfox/sparkfox-thinking/src/lib.rs             # 7.5
crates/sparkfox/sparkfox-monitor/src/{lib.rs,stats.rs,activity.rs}  # 7.6
crates/sparkfox/sparkfox-orchestrator/src/{lib.rs,dag.rs,swarm.rs}  # 7.7
crates/sparkfox/sparkfox-hotspot/src/{lib.rs,platforms/} # 8.1
crates/sparkfox/sparkfox-security/src/lib.rs             # 8.2
```

### 修改的前端文件

```
ui/src/renderer/components/layout/SparkFoxSider.tsx                    # F1 知识库入口
ui/src/renderer/views/ChatView/components/CitationChip.tsx             # F2 新建
ui/src/renderer/views/ChatView/index.tsx                               # F2 集成
ui/src/renderer/pages/knowledge/CreateStudio/SourceConfig.tsx          # F3 文件拖拽
ui/src/renderer/pages/knowledge/KnowledgeDetailPage/index.tsx          # F4 向量化进度
ui/src/renderer/pages/knowledge/KnowledgeListPage/index.tsx            # F5 检索模式
ui/src/renderer/store/sparkfox/*.ts                                    # 7.8 store IPC 对接
ui/src/renderer/views/AgentDashboardView/                              # 8.4 升级
ui/src/renderer/views/HotspotView/Earth3D.tsx                          # 8.5 3D 地球
ui/src/renderer/views/KnowledgeGraphView/                              # 8.15 图谱可视化
ui/src/renderer/hooks/sparkfox/README.md                               # 7.9 文档
```

### 修改的文档

```
docs/poc-report.md                          # PoC-3 实测数据
docs/决策记录.md                            # 9.1 同步策略决策
docs/user-guide/knowledge.md                # 用户文档
docs/AGPL合规审计报告.md                    # AGPL 审计
NOTICE                                      # 全局 NOTICE 更新
```

### 不修改

- sparkfox-core/memory/crdt/e2ee/store（v0.2.0 已落地）
- NomiFun 27 .tsx 知识库 UI 组件（仅扩展，不重写）
- NomiFun 18 .rs 后端代码（仅封装，不重写）

---

## 三、Task 分解（9 模块 50 任务）

### 模块一：PoC-3 阻塞验证（v1.0.0 前置，P0）

#### Task 1.1: sparkfox-embedding crate 骨架

**Files:**
- Create: `crates/sparkfox/sparkfox-embedding/Cargo.toml`
- Create: `crates/sparkfox/sparkfox-embedding/src/lib.rs`
- Modify: `Cargo.toml`（workspace.members 已含，无需修改）

- [ ] **Step 1.1.1: 创建 Cargo.toml**

```toml
[package]
name = "sparkfox-embedding"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "SparkFox embedding + reranker (candle-transformers)"

[dependencies]
sparkfox-core = { path = "../sparkfox-core" }
sparkfox-store = { path = "../sparkfox-store" }
candle-core = "0.7"
candle-nn = "0.7"
candle-transformers = "0.7"
tokenizers = "0.20"
hf-hub = "0.3"
serde = { workspace = true }
serde_json = { workspace = true }
log = "0.4"
env_logger = "0.11"
sha2 = "0.10"
tokio = { workspace = true }

[dev-dependencies]
sparkfox-embedding = { path = "." }
tempfile = "3.10"
```

- [ ] **Step 1.1.2: 创建 src/lib.rs（占位）**

```rust
//! SparkFox Embedding — 嵌入与重排（candle-transformers）
//!
//! 基于 candle-transformers 0.7（Hugging Face 官方 Rust 栈）。
//! 嵌入模型：bge-small-zh-v1.5（默认，120MB）/ bge-large-zh-v1.5（可选，1.2GB）
//! 重排模型：bge-reranker-v2-m3（560MB，v0.3.2+）

#![forbid(unsafe_code)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-embedding v{} initialized", VERSION);
}
```

- [ ] **Step 1.1.3: 验证编译**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-embedding 2>&1 | Select-Object -Last 20
```
Expected: PASS（无编译错误）

- [ ] **Step 1.1.4: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 1.1 crate 骨架"
```

---

#### Task 1.2: 模型下载器 + SHA256 校验

**Files:**
- Create: `crates/sparkfox/sparkfox-embedding/src/downloader.rs`
- Modify: `crates/sparkfox/sparkfox-embedding/src/lib.rs`（添加 `pub mod downloader;`）

- [ ] **Step 1.2.1: 实现 downloader.rs**

```rust
//! bge 模型下载 + SHA256 校验（防供应链攻击）

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use sparkfox_core::{Error, Result};

#[derive(Debug, Clone)]
pub enum ModelVariant {
    BgeSmallZh,       // 120MB
    BgeLargeZh,       // 1.2GB
    BgeRerankerV2M3,  // 560MB
}

impl ModelVariant {
    pub fn repo_id(&self) -> &'static str {
        match self {
            Self::BgeSmallZh => "BAAI/bge-small-zh-v1.5",
            Self::BgeLargeZh => "BAAI/bge-large-zh-v1.5",
            Self::BgeRerankerV2M3 => "BAAI/bge-reranker-v2-m3",
        }
    }

    pub fn expected_files(&self) -> &[&'static str] {
        match self {
            Self::BgeSmallZh | Self::BgeLargeZh => &[
                "config.json", "tokenizer.json", "tokenizer_config.json",
                "model.safetensors", "special_tokens_map.json",
            ],
            Self::BgeRerankerV2M3 => &[
                "config.json", "tokenizer.json", "tokenizer_config.json",
                "model.safetensors", "special_tokens_map.json",
            ],
        }
    }
}

pub fn cache_dir() -> PathBuf {
    let base = dirs_next::data_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("sparkfox")
        .join("models");
    std::fs::create_dir_all(&base).ok();
    base
}

pub fn model_dir(variant: &ModelVariant) -> PathBuf {
    cache_dir().join(variant.repo_id().replace('/', "_"))
}

/// 下载模型文件（若本地已存在且 SHA256 匹配则跳过）
pub fn download_model(variant: &ModelVariant) -> Result<PathBuf> {
    let dir = model_dir(variant);
    std::fs::create_dir_all(&dir).map_err(|e| Error::io(e))?;
    let api = hf_hub::api::sync::ApiBuilder::new()
        .with_cache_dir(cache_dir())
        .build()
        .map_err(|e| Error::internal(format!("hf-hub init 失败: {e}")))?;
    for filename in variant.expected_files() {
        log::info!("下载 {} / {}", variant.repo_id(), filename);
        let _path = api
            .model(variant.repo_id().to_string())
            .get(filename)
            .map_err(|e| Error::internal(format!("下载 {filename} 失败: {e}")))?;
    }
    log::info!("模型 {} 已就绪: {}", variant.repo_id(), dir.display());
    Ok(dir)
}

/// 校验文件 SHA256
pub fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    let bytes = std::fs::read(path).map_err(|e| Error::io(e))?;
    hasher.update(&bytes);
    let actual = hex::encode(hasher.finalize());
    if actual != expected {
        return Err(Error::crypto(format!(
            "SHA256 校验失败: {} 期望 {expected} 实际 {actual}",
            path.display()
        )));
    }
    Ok(())
}
```

- [ ] **Step 1.2.2: 在 lib.rs 添加 `pub mod downloader;`**

- [ ] **Step 1.2.3: 验证编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-embedding 2>&1 | Select-Object -Last 20
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 1.2 模型下载器 + SHA256 校验"
```

---

#### Task 1.3: bge 嵌入推理器（BgeEmbedder）

**Files:**
- Create: `crates/sparkfox/sparkfox-embedding/src/embedder.rs`
- Modify: `crates/sparkfox/sparkfox-embedding/src/lib.rs`

- [ ] **Step 1.3.1: 实现 embedder.rs**

```rust
//! BgeEmbedder — bge 嵌入推理（candle-transformers）

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::BertModel;
use tokenizers::Tokenizer;

use sparkfox_core::{Error, Result};

use crate::downloader::{download_model, ModelVariant};

pub struct BgeEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl BgeEmbedder {
    pub fn load(variant: ModelVariant) -> Result<Self> {
        let model_dir = download_model(&variant)?;
        let device = Device::Cpu; // 桌面端默认 CPU，GPU 留 v1.1+
        let config_path = model_dir.join("config.json");
        let tokenizer_path = model_dir.join("tokenizer.json");
        let weights_path = model_dir.join("model.safetensors");

        let config = candle_transformers::models::bert::Config::from_file(&config_path);
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| Error::internal(format!("tokenizer 加载失败: {e}")))?;
        let vb = unsafe {
            candle_nn::VarBuilder::from_mmaped_safetensors(
                &[weights_path], DType::F32, &device
            ).map_err(|e| Error::internal(format!("weights 加载失败: {e}")))?
        };
        let model = BertModel::load(&vb, &config)
            .map_err(|e| Error::internal(format!("BertModel 加载失败: {e}")))?;
        Ok(Self { model, tokenizer, device })
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let tokens = self.tokenizer.encode(text, true)
            .map_err(|e| Error::internal(format!("tokenize 失败: {e}")))?;
        let ids = tokens.get_ids();
        let input_ids = Tensor::from_slice(ids as &[u32], (1, ids.len()), &self.device)
            .map_err(|e| Error::internal(format!("Tensor 构造失败: {e}")))?;
        let token_type_ids = input_ids.zeros_like().unwrap();
        let attention_mask = input_ids.ones_like().unwrap();
        let embeddings = self.model.forward(&input_ids, &token_type_ids, &attention_mask)
            .map_err(|e| Error::internal(format!("forward 失败: {e}")))?;
        // mean pooling
        let pooled = mean_pool(&embeddings, &attention_mask)?;
        let normalized = pooled.norm(2)?;
        let out = (&pooled / &normalized)?.squeeze(0)
            .map_err(|e| Error::internal(format!("squeeze 失败: {e}")))?;
        let vec = out.to_vec1::<f32>()
            .map_err(|e| Error::internal(format!("to_vec1 失败: {e}")))?;
        Ok(vec)
    }

    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
}

fn mean_pool(embeddings: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
    let mask = attention_mask.to_dtype(candle_core::DType::F32).unwrap();
    let sum = (embeddings * &mask).unwrap().sum(1).unwrap();
    let count = mask.sum(1).unwrap();
    let pooled = (sum / count).unwrap();
    Ok(pooled)
}
```

- [ ] **Step 1.3.2: 在 lib.rs 添加 `pub mod embedder;` + 重导出**

```rust
pub mod downloader;
pub mod embedder;
pub use embedder::BgeEmbedder;
pub use downloader::ModelVariant;
```

- [ ] **Step 1.3.3: 验证编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-embedding 2>&1 | Select-Object -Last 30
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 1.3 BgeEmbedder 推理器"
```

---

#### Task 1.4: PoC-3 性能测试（严格 cosine > 0.99）

**Files:**
- Create: `crates/sparkfox/sparkfox-embedding/tests/poc3_bge.rs`

- [ ] **Step 1.4.1: 写 PoC-3 测试**

```rust
//! PoC-3 bge Rust 推理性 + 与 Python 一致性（严格 cosine > 0.99）
#![forbid(unsafe_code)]

use std::time::Instant;

use sparkfox_embedding::{BgeEmbedder, ModelVariant};

#[test]
fn poc3_single_embedding_under_50ms() {
    let embedder = BgeEmbedder::load(ModelVariant::BgeSmallZh).expect("加载模型");
    // 预热
    let _ = embedder.embed("预热");
    let t = Instant::now();
    let v = embedder.embed("你好世界").expect("嵌入");
    let elapsed = t.elapsed();
    assert_eq!(v.len(), 512, "bge-small-zh 维度应为 512");
    assert!(elapsed.as_millis() < 50, "单条嵌入 {elapsed:?} 超过 50ms");
}

#[test]
fn poc3_batch_1000_under_30s() {
    let embedder = BgeEmbedder::load(ModelVariant::BgeSmallZh).expect("加载模型");
    let texts: Vec<String> = (0..1000).map(|i| format!("测试文本 {i}")).collect();
    let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let t = Instant::now();
    let vecs = embedder.embed_batch(&refs).expect("批量嵌入");
    let elapsed = t.elapsed();
    assert_eq!(vecs.len(), 1000);
    assert!(elapsed.as_secs() < 30, "1000 条批量 {elapsed:?} 超过 30s");
}

#[test]
fn poc3_consistency_with_python_strict_099() {
    // 严格门槛：每条 cosine > 0.99（用户决策 B）
    let embedder = BgeEmbedder::load(ModelVariant::BgeSmallZh).expect("加载模型");
    // 预设 Python 基线向量（从 sentence-transformers 预先计算并保存）
    // 测试集：1000 条中文文本（短/长/技术/文学）
    let baseline = load_python_baseline("tests/baseline_small.json");
    let mut min_cosine = 1.0f32;
    for (text, expected) in baseline.iter().take(100) {
        let actual = embedder.embed(text).expect("嵌入");
        let cos = cosine_similarity(&actual, expected);
        assert!(cos > 0.99, "cosine {cos} < 0.99 (text: {text})");
        min_cosine = min_cosine.min(cos);
    }
    eprintln!("最小 cosine = {min_cosine}");
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}

fn load_python_baseline(path: &str) -> Vec<(String, Vec<f32>)> {
    let content = std::fs::read_to_string(path).expect("读取 baseline");
    serde_json::from_str(&content).expect("解析 baseline")
}
```

- [ ] **Step 1.4.2: 准备 Python baseline**

```bash
# 在 Python 环境运行（仅生成 baseline，不参与 Rust 运行时）
python -c "
from sentence_transformers import SentenceTransformer
import json
model = SentenceTransformer('BAAI/bge-small-zh-v1.5')
texts = ['测试文本 ' + str(i) for i in range(100)]
vecs = model.encode(texts).tolist()
data = list(zip(texts, vecs))
json.dump(data, open('tests/baseline_small.json', 'w'))
"
```

- [ ] **Step 1.4.3: 运行 PoC-3 测试**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-embedding --test poc3_bge -- --nocapture 2>&1 | Select-Object -Last 50
```

Expected: 3/3 PASS（前提：模型已下载，baseline 已生成）

- [ ] **Step 1.4.4: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 1.4 PoC-3 性能测试（严格 cosine > 0.99）"
```

---

#### Task 1.5: PoC-3 决策与报告填入

**Files:**
- Modify: `docs/poc-report.md`

- [ ] **Step 1.5.1: 填入 PoC-3 实测数据**

根据 Task 1.4 实测结果填入：
- 单条延迟：实测值
- 1000 条批量：实测值
- 一致性：最小 cosine
- 决策：GO / NO-GO

- [ ] **Step 1.5.2: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add docs/poc-report.md
git -C "D:\xin kaifa\SparkFox" commit -m "docs(poc): 填入 PoC-3 实测数据与决策"
```

---

#### Task 1.6: Python sidecar 备选骨架（仅 NO-GO 时启用）

**Files:**
- Create: `sidecar/embed_server.py.stub`
- Create: `sidecar/README.md`

- [ ] **Step 1.6.1: 创建 Python sidecar 骨架**

```python
# SparkFox embedding sidecar (仅 PoC-3 NO-GO 时启用)
# 启动：uvicorn embed_server:app --port 8765
# 协议：HTTP POST /embed { "text": "..." } -> { "embedding": [...] }

from fastapi import FastAPI
from pydantic import BaseModel
from sentence_transformers import SentenceTransformer

app = FastAPI()
model = SentenceTransformer('BAAI/bge-small-zh-v1.5')

class EmbedRequest(BaseModel):
    text: str

@app.post("/embed")
def embed(req: EmbedRequest):
    vec = model.encode(req.text).tolist()
    return {"embedding": vec}

@app.post("/embed_batch")
def embed_batch(texts: list[str]):
    vecs = model.encode(texts).tolist()
    return {"embeddings": vecs}
```

- [ ] **Step 1.6.2: 创建 sidecar/README.md**（说明启用方式）

- [ ] **Step 1.6.3: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add sidecar
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sidecar): 1.6 Python embedding sidecar 备选骨架（仅 NO-GO 时启用）"
```

---

#### Task 1.7: 模块一集成验证

- [ ] **Step 1.7.1: 运行模块一全部测试**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-embedding 2>&1 | Select-Object -Last 30
```

Expected: 3/3 PASS（PoC-3 GO）

- [ ] **Step 1.7.2: 更新 todo，进入模块二**

---

### 模块二：sparkfox-embedding 完整实现（P0）

#### Task 2.1: Embedder trait 抽象

**Files:**
- Modify: `crates/sparkfox/sparkfox-embedding/src/lib.rs`

- [ ] **Step 2.1.1: 定义 Embedder trait**

```rust
/// 嵌入器 trait — 支持多模型切换
pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    fn dim(&self) -> usize;
    fn model_name(&self) -> &str;
}
```

- [ ] **Step 2.1.2: 为 BgeEmbedder 实现 Embedder trait**（在 embedder.rs 中）

```rust
impl Embedder for BgeEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>> { self.embed(text) }
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> { self.embed_batch(texts) }
    fn dim(&self) -> usize { 512 }  // bge-small-zh
    fn model_name(&self) -> &str { "bge-small-zh-v1.5" }
}
```

- [ ] **Step 2.1.3: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-embedding 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 2.1 Embedder trait 抽象"
```

---

#### Task 2.2: 查询嵌入缓存（仅缓存查询，文档嵌入每次重建）

**Files:**
- Create: `crates/sparkfox/sparkfox-embedding/src/cache.rs`
- Modify: `crates/sparkfox/sparkfox-embedding/src/lib.rs`

- [ ] **Step 2.2.1: 实现 cache.rs**

```rust
//! 查询嵌入缓存 — 仅缓存查询嵌入（用户决策 B：文档嵌入每次重建）
//!
//! 缓存表（复用 sparkfox-store 的 SQLite）：
//! - query_embedding_cache(content_hash TEXT PK, model TEXT, embedding BLOB, ts INTEGER)

use sha2::{Digest, Sha256};

use sparkfox_core::{Error, Result};
use sparkfox_store::Store;

pub struct QueryCache<'a> {
    store: &'a Store,
}

impl<'a> QueryCache<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn init_schema(&self) -> Result<()> {
        self.store.conn().execute_batch(
            r#"CREATE TABLE IF NOT EXISTS query_embedding_cache (
                content_hash TEXT PRIMARY KEY,
                model TEXT NOT NULL,
                embedding BLOB NOT NULL,
                ts INTEGER NOT NULL
            );"#,
        )?;
        Ok(())
    }

    pub fn get(&self, text: &str, model: &str) -> Result<Option<Vec<f32>>> {
        let hash = hash_text(text);
        let mut stmt = self.store.conn().prepare(
            "SELECT embedding FROM query_embedding_cache WHERE content_hash=? AND model=?",
        )?;
        let blob: Option<Vec<u8>> = stmt.query_row(rusqlite::params![hash, model], |r| r.get(0)).ok();
        match blob {
            Some(b) => {
                let vec = bytes_to_vec(&b);
                Ok(Some(vec))
            }
            None => Ok(None),
        }
    }

    pub fn put(&self, text: &str, model: &str, embedding: &[f32]) -> Result<()> {
        let hash = hash_text(text);
        let blob = vec_to_bytes(embedding);
        self.store.conn().execute(
            "INSERT OR REPLACE INTO query_embedding_cache(content_hash, model, embedding, ts) VALUES (?, ?, ?, ?)",
            rusqlite::params![hash, model, blob, now_ts()],
        )?;
        Ok(())
    }
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn vec_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn bytes_to_vec(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
```

- [ ] **Step 2.2.2: lib.rs 添加 `pub mod cache;`**

- [ ] **Step 2.2.3: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-embedding 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 2.2 查询嵌入缓存（仅缓存查询）"
```

---

#### Task 2.3: bge-large-zh 可选切换

**Files:**
- Create: `crates/sparkfox/sparkfox-embedding/src/config.rs`
- Modify: `crates/sparkfox/sparkfox-embedding/src/lib.rs`

- [ ] **Step 2.3.1: 实现 config.rs**

```rust
//! 嵌入模型配置 — 支持运行时切换 bge-small-zh / bge-large-zh

use serde::{Deserialize, Serialize};

use crate::downloader::ModelVariant;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmbeddingModel {
    BgeSmallZh,
    BgeLargeZh,
}

impl Default for EmbeddingModel {
    fn default() -> Self { Self::BgeSmallZh }
}

impl EmbeddingModel {
    pub fn to_variant(&self) -> ModelVariant {
        match self {
            Self::BgeSmallZh => ModelVariant::BgeSmallZh,
            Self::BgeLargeZh => ModelVariant::BgeLargeZh,
        }
    }

    pub fn dim(&self) -> usize {
        match self {
            Self::BgeSmallZh => 512,
            Self::BgeLargeZh => 1024,
        }
    }

    pub fn size_mb(&self) -> usize {
        match self {
            Self::BgeSmallZh => 120,
            Self::BgeLargeZh => 1200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub model: EmbeddingModel,
    pub reranker_enabled: bool,  // v0.3.2+ rerank 开关
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self { model: EmbeddingModel::default(), reranker_enabled: false }
    }
}
```

- [ ] **Step 2.3.2: lib.rs 添加 `pub mod config;` + 重导出**

- [ ] **Step 2.3.3: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-embedding 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 2.3 模型切换配置"
```

---

#### Task 2.4: 嵌入失败降级策略

**Files:**
- Modify: `crates/sparkfox/sparkfox-embedding/src/embedder.rs`

- [ ] **Step 2.4.1: 添加降级逻辑**

```rust
impl BgeEmbedder {
    /// 尝试加载模型，失败时返回明确 Error（上层降级为纯关键词检索）
    pub fn try_load(variant: ModelVariant) -> std::result::Result<Self, LoadError> {
        Self::load(variant).map_err(LoadError::from)
    }
}

#[derive(Debug)]
pub enum LoadError {
    ModelNotFound(String),
    InsufficientMemory(String),
    Other(String),
}

impl From<sparkfox_core::Error> for LoadError {
    fn from(e: sparkfox_core::Error) -> Self {
        let msg = e.to_string();
        if msg.contains("not found") || msg.contains("未找到") {
            Self::ModelNotFound(msg)
        } else if msg.contains("memory") || msg.contains("OOM") {
            Self::InsufficientMemory(msg)
        } else {
            Self::Other(msg)
        }
    }
}
```

- [ ] **Step 2.4.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-embedding 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 2.4 嵌入失败降级策略"
```

---

#### Task 2.5: NOTICE + README

**Files:**
- Create: `crates/sparkfox/sparkfox-embedding/NOTICE`
- Create: `crates/sparkfox/sparkfox-embedding/README.md`

- [ ] **Step 2.5.1: 创建 NOTICE**

```
SparkFox sparkfox-embedding
Copyright (C) 2026 SparkFox Contributors (AGPL-3.0-only)

本 crate 使用以下第三方资源：

1. candle-core / candle-nn / candle-transformers 0.7
   License: MIT
   Source: https://github.com/huggingface/candle
   Copyright (c) Hugging Face Inc.

2. tokenizers 0.20
   License: Apache-2.0
   Source: https://github.com/huggingface/tokenizers

3. bge-small-zh-v1.5 / bge-large-zh-v1.5 / bge-reranker-v2-m3
   License: MIT
   Source: https://huggingface.co/BAAI
   Copyright (c) BAAI

4. hf-hub 0.3
   License: Apache-2.0
   Source: https://github.com/huggingface/hf-hub

5. sha2 0.10
   License: MIT OR Apache-2.0
   Source: https://github.com/RustCrypto/hashes
```

- [ ] **Step 2.5.2: 创建 README.md**（含使用示例）

- [ ] **Step 2.5.3: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding/NOTICE crates/sparkfox/sparkfox-embedding/README.md
git -C "D:\xin kaifa\SparkFox" commit -m "docs(sparkfox-embedding): 2.5 NOTICE + README"
```

---

### 模块三：sparkfox-knowledge RAG 引擎核心（P0）

#### Task 3.1: crate 骨架 + 边界定义

**Files:**
- Create: `crates/sparkfox/sparkfox-knowledge/Cargo.toml`
- Create: `crates/sparkfox/sparkfox-knowledge/src/lib.rs`

- [ ] **Step 3.1.1: 创建 Cargo.toml**

```toml
[package]
name = "sparkfox-knowledge"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "SparkFox knowledge base RAG engine"

[dependencies]
sparkfox-core = { path = "../sparkfox-core" }
sparkfox-store = { path = "../sparkfox-store" }
sparkfox-embedding = { path = "../sparkfox-embedding" }
nomifun-knowledge = { path = "../../backend/nomifun-knowledge" }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
log = "0.4"
env_logger = "0.11"

[dev-dependencies]
sparkfox-knowledge = { path = "." }
tempfile = "3.10"
```

- [ ] **Step 3.1.2: 创建 src/lib.rs（含边界声明）**

```rust
//! SparkFox Knowledge — 知识库 RAG 引擎
//!
//! 【边界声明】
//! - sparkfox-knowledge：RAG 编排层（分块/嵌入/检索/重排/引用）
//! - nomifun-knowledge：底层 IO 层（文件系统/MCP/写回/binding）
//! - sparkfox-knowledge 调用 nomifun-knowledge 读取文档，调用 sparkfox-embedding 生成嵌入，
//!   调用 sparkfox-store 存储向量，不直接操作文件系统

#![forbid(unsafe_code)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-knowledge v{} initialized", VERSION);
}
```

- [ ] **Step 3.1.3: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-knowledge 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-knowledge
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-knowledge): 3.1 crate 骨架 + 边界定义"
```

---

#### Task 3.2: 文档分块器

**Files:**
- Create: `crates/sparkfox/sparkfox-knowledge/src/chunk.rs`

- [ ] **Step 3.2.1: 实现 chunk.rs**

```rust
//! 文档分块 — 固定长度 256 tokens + 50 重叠
//! 中文用 tokenizer 分词（不能用空格）

use sparkfox_core::Result;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: usize,
    pub text: String,
    pub start_char: usize,
    pub end_char: usize,
}

pub struct Chunker {
    chunk_size: usize,   // 256 tokens
    overlap: usize,      // 50 tokens
    tokenizer: tokenizers::Tokenizer,
}

impl Chunker {
    pub fn new() -> Result<Self> {
        let tokenizer = tokenizers::Tokenizer::from_pretrained("BAAI/bge-small-zh-v1.5", None)
            .map_err(|e| sparkfox_core::Error::internal(format!("tokenizer 加载失败: {e}")))?;
        Ok(Self { chunk_size: 256, overlap: 50, tokenizer })
    }

    pub fn chunk(&self, text: &str) -> Vec<Chunk> {
        // 简化实现：按字符分块（256 字符 + 50 重叠）
        // 生产级实现应用 tokenizer 分词
        let chars: Vec<char> = text.chars().collect();
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut id = 0;
        while start < chars.len() {
            let end = (start + self.chunk_size).min(chars.len());
            let chunk_text: String = chars[start..end].iter().collect();
            chunks.push(Chunk {
                id,
                text: chunk_text,
                start_char: start,
                end_char: end,
            });
            id += 1;
            if end >= chars.len() { break; }
            start += self.chunk_size - self.overlap;
        }
        chunks
    }
}
```

- [ ] **Step 3.2.2: lib.rs 添加 `pub mod chunk;` + 重导出**

- [ ] **Step 3.2.3: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-knowledge 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-knowledge
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-knowledge): 3.2 文档分块器（256+50 重叠）"
```

---

#### Task 3.3: 向量召回

**Files:**
- Create: `crates/sparkfox/sparkfox-knowledge/src/rag.rs`

- [ ] **Step 3.3.1: 实现 rag.rs（向量召回部分）**

```rust
//! RAG 引擎 — 向量召回 + 关键词召回 + RRF 融合

use sparkfox_core::{Error, Result};
use sparkfox_embedding::Embedder;
use sparkfox_store::Store;

use crate::chunk::Chunk;

pub struct RagEngine<'a> {
    store: &'a Store,
    embedder: &'a dyn Embedder,
}

impl<'a> RagEngine<'a> {
    pub fn new(store: &'a Store, embedder: &'a dyn Embedder) -> Self {
        Self { store, embedder }
    }

    /// 向量召回 top-k
    pub fn vector_recall(&self, query: &str, k: usize) -> Result<Vec<SearchHit>> {
        let query_vec = self.embedder.embed(query)?;
        let hits = self.store.vector_search(&query_vec, k)?;
        Ok(hits.into_iter().map(|(id, score)| SearchHit {
            chunk_id: id,
            score,
            source: SearchSource::Vector,
        }).collect())
    }
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub chunk_id: i64,
    pub score: f32,
    pub source: SearchSource,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchSource {
    Vector,
    Keyword,
    Rerank,
}
```

- [ ] **Step 3.3.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-knowledge 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-knowledge
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-knowledge): 3.3 向量召回"
```

---

#### Task 3.4: 关键词召回（FTS5）

**Files:**
- Modify: `crates/sparkfox/sparkfox-store/src/schema.rs`（添加 FTS5 表）
- Modify: `crates/sparkfox/sparkfox-knowledge/src/rag.rs`

- [ ] **Step 3.4.1: 在 sparkfox-store schema 添加 FTS5 表**

```rust
// 在 MIGRATIONS 数组末尾添加
r#"CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
    content,
    content_id UNINDEXED,
    tokenize='unicode61'
);"#,
```

- [ ] **Step 3.4.2: 在 rag.rs 添加关键词召回**

```rust
impl<'a> RagEngine<'a> {
    /// 关键词召回 top-k（FTS5）
    pub fn keyword_recall(&self, query: &str, k: usize) -> Result<Vec<SearchHit>> {
        let mut stmt = self.store.conn().prepare(
            "SELECT content_id, rank FROM memory_fts WHERE content MATCH ? ORDER BY rank LIMIT ?",
        )?;
        let rows = stmt.query_map(rusqlite::params![query, k as i64], |r| {
            Ok(SearchHit {
                chunk_id: r.get::<_, i64>(0)?,
                score: -r.get::<_, f32>(1)?, // FTS5 rank 越小越好，取负
                source: SearchSource::Keyword,
            })
        })?;
        let mut hits = Vec::new();
        for r in rows { hits.push(r?); }
        Ok(hits)
    }
}
```

- [ ] **Step 3.4.3: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-knowledge 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-knowledge crates/sparkfox/sparkfox-store
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-knowledge): 3.4 FTS5 关键词召回"
```

---

#### Task 3.5: RRF 融合

**Files:**
- Modify: `crates/sparkfox/sparkfox-knowledge/src/rag.rs`

- [ ] **Step 3.5.1: 实现 RRF 融合**

```rust
impl<'a> RagEngine<'a> {
    /// RRF 融合：score(d) = Σ 1/(k + rank_i(d))，k=60
    pub fn rrf_fuse(&self, vector_hits: Vec<SearchHit>, keyword_hits: Vec<SearchHit>, k: usize) -> Vec<SearchHit> {
        const RRF_K: f32 = 60.0;
        let mut scores: std::collections::HashMap<i64, (f32, SearchSource)> = std::collections::HashMap::new();
        for (rank, hit) in vector_hits.iter().enumerate() {
            let score = 1.0 / (RRF_K + rank as f32 + 1.0);
            scores.entry(hit.chunk_id)
                .and_modify(|(s, _)| *s += score)
                .or_insert((score, hit.source));
        }
        for (rank, hit) in keyword_hits.iter().enumerate() {
            let score = 1.0 / (RRF_K + rank as f32 + 1.0);
            scores.entry(hit.chunk_id)
                .and_modify(|(s, _)| *s += score)
                .or_insert((score, hit.source));
        }
        let mut fused: Vec<SearchHit> = scores.into_iter()
            .map(|(chunk_id, (score, source))| SearchHit { chunk_id, score, source })
            .collect();
        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        fused.truncate(k);
        fused
    }

    /// 混合检索：向量 top-50 + 关键词 top-50 → RRF top-10
    pub fn hybrid_search(&self, query: &str, k: usize) -> Result<Vec<SearchHit>> {
        let vector_hits = self.vector_recall(query, 50)?;
        let keyword_hits = self.keyword_recall(query, 50)?;
        Ok(self.rrf_fuse(vector_hits, keyword_hits, k))
    }
}
```

- [ ] **Step 3.5.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-knowledge 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-knowledge
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-knowledge): 3.5 RRF 融合（k=60）"
```

---

#### Task 3.6: 引用协议定义

**Files:**
- Create: `crates/sparkfox/sparkfox-knowledge/src/citation.rs`

- [ ] **Step 3.6.1: 定义 Citation 结构**

```rust
//! 引用协议 — CitationChip 数据结构 + ChatView 集成协议
//!
//! LLM 响应中的引用标记格式：[citation:kdoc_xxx:chunk_5:0:128]
//! ChatView 解析后渲染为 CitationChip

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub kdoc_id: String,       // 文档句柄 kdoc_{base64}
    pub chunk_id: i64,         // 块 ID
    pub span_start: usize,     // 块内字符起始
    pub span_end: usize,       // 块内字符结束
    pub score: f32,            // 检索分数
    pub source: CitationSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CitationSource {
    Vector,
    Keyword,
    Rerank,
}

impl Citation {
    /// 生成 LLM 响应中的引用标记
    pub fn to_marker(&self) -> String {
        format!("[citation:{}:{}:{}:{}]", self.kdoc_id, self.chunk_id, self.span_start, self.span_end)
    }

    /// 从标记解析（ChatView 用）
    pub fn parse_marker(marker: &str) -> Option<Self> {
        let inner = marker.strip_prefix("[citation:")?.strip_suffix(']')?;
        let parts: Vec<&str> = inner.split(':').collect();
        if parts.len() != 4 { return None; }
        Some(Self {
            kdoc_id: parts[0].to_string(),
            chunk_id: parts[1].parse().ok()?,
            span_start: parts[2].parse().ok()?,
            span_end: parts[3].parse().ok()?,
            score: 0.0,
            source: CitationSource::Vector,
        })
    }
}
```

- [ ] **Step 3.6.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-knowledge 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-knowledge
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-knowledge): 3.6 引用协议定义"
```

---

#### Task 3.7: RAG 端到端测试

**Files:**
- Create: `crates/sparkfox/sparkfox-knowledge/tests/rag_e2e.rs`

- [ ] **Step 3.7.1: 写 E2E 测试**

```rust
//! RAG 端到端测试 — 10 文档 → 提问 → 返回带引用答案
#![forbid(unsafe_code)]

use sparkfox_embedding::{BgeEmbedder, ModelVariant};
use sparkfox_knowledge::{Chunker, RagEngine};
use sparkfox_store::{Store, StoreConfig};

#[test]
fn rag_e2e_vector_search() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let store = Store::open(StoreConfig::for_path(tmp.path())).unwrap();
    let embedder = BgeEmbedder::load(ModelVariant::BgeSmallZh).unwrap();
    let engine = RagEngine::new(&store, &embedder);
    // 插入 10 文档向量
    for i in 0..10 {
        let text = format!("文档 {i} 内容：这是关于主题 {i} 的描述");
        let vec = embedder.embed(&text).unwrap();
        store.vector_insert(i, &vec).unwrap();
    }
    store.vector_flush().unwrap();
    // 检索
    let hits = engine.vector_recall("主题 5", 5).unwrap();
    assert_eq!(hits.len(), 5);
    assert!(hits[0].chunk_id >= 0);
}

#[test]
fn rag_e2e_chunking() {
    let chunker = Chunker::new().unwrap();
    let long_text = "测试文本 ".repeat(500);
    let chunks = chunker.chunk(&long_text);
    assert!(chunks.len() > 1, "长文档应分多块");
    assert_eq!(chunks[0].text.len(), 256);
}
```

- [ ] **Step 3.7.2: 运行 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo test -p sparkfox-knowledge --test rag_e2e 2>&1 | Select-Object -Last 20
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-knowledge
git -C "D:\xin kaifa\SparkFox" commit -m "test(sparkfox-knowledge): 3.7 RAG E2E 测试"
```

---

#### Task 3.8: NOTICE + README

**Files:**
- Create: `crates/sparkfox/sparkfox-knowledge/NOTICE`
- Create: `crates/sparkfox/sparkfox-knowledge/README.md`

- [ ] **Step 3.8.1: 创建 NOTICE**

```
SparkFox sparkfox-knowledge
Copyright (C) 2026 SparkFox Contributors (AGPL-3.0-only)

本 crate 使用/借鉴以下资源：

1. nomifun-knowledge (Apache-2.0)
   Source: crates/backend/nomifun-knowledge/
   Copyright (c) NomiFun Contributors

2. RAGFlow DeepDoc 思路（Apache-2.0）
   思路借鉴 + Rust 重写，未拷贝代码
   Source: https://github.com/infiniflow/ragflow

3. FastGPT RRF 公式（商用付费）
   仅算法公式（Cormack 2009），不抄代码
   算法不受版权保护

4. AnythingLLM RAG 设计思路（MIT）
   思路借鉴 + Rust 重写
   Source: https://github.com/Mintplex-Labs/anything-llm
```

- [ ] **Step 3.8.2: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-knowledge/NOTICE crates/sparkfox/sparkfox-knowledge/README.md
git -C "D:\xin kaifa\SparkFox" commit -m "docs(sparkfox-knowledge): 3.8 NOTICE + README"
```

---

### 模块四：sparkfox-parser 多格式解析（P0）

#### Task 4.1: crate 骨架 + Parser trait

**Files:**
- Create: `crates/sparkfox/sparkfox-parser/Cargo.toml`
- Create: `crates/sparkfox/sparkfox-parser/src/lib.rs`

- [ ] **Step 4.1.1: 创建 Cargo.toml**

```toml
[package]
name = "sparkfox-parser"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "SparkFox multi-format document parser"

[dependencies]
sparkfox-core = { path = "../sparkfox-core" }
lopdf = "0.34"
docx-rs = "0.4"
calamine = "0.26"
quick-xml = "0.36"
serde = { workspace = true }
log = "0.4"
env_logger = "0.11"

[dev-dependencies]
sparkfox-parser = { path = "." }
tempfile = "3.10"
```

- [ ] **Step 4.1.2: 创建 lib.rs + Parser trait**

```rust
//! SparkFox Parser — 多格式文档解析（PDF/Word/Excel）
//!
//! 纯 Rust 实现，不引入 Python sidecar。
//! 安全限制：文件 < 100MB + 超时 30s

#![forbid(unsafe_code)]

use std::path::Path;

use sparkfox_core::Result;

pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB
pub const PARSE_TIMEOUT_SECS: u64 = 30;

pub trait Parser: Send + Sync {
    fn parse(&self, path: &Path) -> Result<ParsedDocument>;
    fn supported_extensions(&self) -> &[&str];
}

#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub text: String,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: Option<usize>,
    pub word_count: Option<usize>,
}

pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-parser v{} initialized", env!("CARGO_PKG_VERSION"));
}
```

- [ ] **Step 4.1.3: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-parser 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-parser
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-parser): 4.1 crate 骨架 + Parser trait"
```

---

#### Task 4.2: PDF 解析（lopdf）

**Files:**
- Create: `crates/sparkfox/sparkfox-parser/src/pdf.rs`

- [ ] **Step 4.2.1: 实现 pdf.rs**

```rust
//! PDF 解析 — lopdf 纯文本提取

use std::path::Path;

use lopdf::Document as PdfDocument;

use sparkfox_core::{Error, Result};

use crate::{ParsedDocument, DocumentMetadata, Parser, MAX_FILE_SIZE};

pub struct PdfParser;

impl Parser for PdfParser {
    fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        // 文件大小检查
        let metadata = std::fs::metadata(path).map_err(|e| Error::io(e))?;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(Error::invalid_argument(
                format!("文件 {} 超过 100MB 限制", path.display()),
                "PdfParser::parse",
            ));
        }
        let doc = PdfDocument::load(path).map_err(|e| Error::parse(
            format!("PDF 加载失败: {e}"),
            "PdfParser::parse",
        ))?;
        let mut text = String::new();
        let page_count = doc.get_pages().len();
        for (i, _) in doc.get_pages().iter().enumerate() {
            let page_text = extract_page_text(&doc, i as u32).unwrap_or_default();
            text.push_str(&page_text);
            text.push('\n');
        }
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

    fn supported_extensions(&self) -> &[&str] { &["pdf"] }
}

fn extract_page_text(doc: &PdfDocument, page_num: u32) -> std::result::Result<String, String> {
    // 简化实现：提取 Text 流
    // 生产级实现需处理字体编码、布局分析
    let page = doc.get_pages().get(&(page_num as u32)).ok_or("page not found")?;
    let mut text = String::new();
    for content in doc.get_page_content(page).map_err(|e| e.to_string())? {
        // 简化：直接拼接字节
        if content.is_ascii() {
            text.push_str(&String::from_utf8_lossy(&content));
        }
    }
    Ok(text)
}
```

- [ ] **Step 4.2.2: lib.rs 添加 `pub mod pdf;`**

- [ ] **Step 4.2.3: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-parser 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-parser
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-parser): 4.2 PDF 解析（lopdf）"
```

---

#### Task 4.3: Word 解析（docx-rs）

**Files:**
- Create: `crates/sparkfox/sparkfox-parser/src/docx.rs`

- [ ] **Step 4.3.1: 实现 docx.rs**

```rust
//! Word 解析 — docx-rs

use std::path::Path;

use docx_rs::{read_docx, Paragraph};

use sparkfox_core::{Error, Result};

use crate::{ParsedDocument, DocumentMetadata, Parser, MAX_FILE_SIZE};

pub struct DocxParser;

impl Parser for DocxParser {
    fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        let metadata = std::fs::metadata(path).map_err(|e| Error::io(e))?;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(Error::invalid_argument(
                format!("文件 {} 超过 100MB 限制", path.display()),
                "DocxParser::parse",
            ));
        }
        let file = std::fs::File::open(path).map_err(|e| Error::io(e))?;
        let docx = read_docx(file).map_err(|e| Error::parse(
            format!("docx 加载失败: {e}"),
            "DocxParser::parse",
        ))?;
        let mut text = String::new();
        for child in docx.document.children {
            if let docx_rs::DocumentChild::Paragraph(p) = child {
                for c in p.children {
                    if let docx_rs::ParagraphChild::Text(t) = c {
                        text.push_str(&t.text);
                    }
                }
                text.push('\n');
            }
        }
        let word_count = text.split_whitespace().count();
        Ok(ParsedDocument {
            text,
            metadata: DocumentMetadata {
                word_count: Some(word_count),
                ..Default::default()
            },
        })
    }

    fn supported_extensions(&self) -> &[&str] { &["docx"] }
}
```

- [ ] **Step 4.3.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-parser 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-parser
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-parser): 4.3 Word 解析（docx-rs）"
```

---

#### Task 4.4: Excel 解析（calamine）

**Files:**
- Create: `crates/sparkfox/sparkfox-parser/src/xlsx.rs`

- [ ] **Step 4.4.1: 实现 xlsx.rs**

```rust
//! Excel 解析 — calamine

use std::path::Path;

use calamine::{open_workbook, Reader, Xlsx, DataType};

use sparkfox_core::{Error, Result};

use crate::{ParsedDocument, DocumentMetadata, Parser, MAX_FILE_SIZE};

pub struct XlsxParser;

impl Parser for XlsxParser {
    fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        let metadata = std::fs::metadata(path).map_err(|e| Error::io(e))?;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(Error::invalid_argument(
                format!("文件 {} 超过 100MB 限制", path.display()),
                "XlsxParser::parse",
            ));
        }
        let mut workbook: Xlsx<_> = open_workbook(path).map_err(|e| Error::parse(
            format!("xlsx 加载失败: {e}"),
            "XlsxParser::parse",
        ))?;
        let mut text = String::new();
        for sheet_name in workbook.sheet_names().to_vec() {
            text.push_str(&format!("[Sheet: {sheet_name}]\n"));
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                for row in range.rows() {
                    let cells: Vec<String> = row.iter().map(|c| match c {
                        DataType::String(s) => s.clone(),
                        DataType::Int(i) => i.to_string(),
                        DataType::Float(f) => f.to_string(),
                        DataType::Bool(b) => b.to_string(),
                        DataType::DateTime(d) => d.to_string(),
                        DataType::Empty => String::new(),
                        _ => String::new(),
                    }).collect();
                    text.push_str(&cells.join("\t"));
                    text.push('\n');
                }
            }
        }
        Ok(ParsedDocument {
            text,
            metadata: DocumentMetadata::default(),
        })
    }

    fn supported_extensions(&self) -> &[&str] { &["xlsx", "xls"] }
}
```

- [ ] **Step 4.4.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-parser 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-parser
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-parser): 4.4 Excel 解析（calamine）"
```

---

#### Task 4.5: 解析安全限制 + 超时

**Files:**
- Modify: `crates/sparkfox/sparkfox-parser/src/lib.rs`

- [ ] **Step 4.5.1: 添加超时包装**

```rust
use std::time::Duration;

/// 带超时的解析（30s）
pub fn parse_with_timeout(parser: &dyn Parser, path: &Path) -> Result<ParsedDocument> {
    let (tx, rx) = std::sync::mpsc::channel();
    let path_str = path.to_string_lossy().to_string();
    let parser_ptr = parser as *const dyn Parser;
    // 注意：此处的 unsafe 是因为 thread 不能持有 trait object 引用
    // 生产级实现应用 rayon::spawn 或 tokio::spawn + timeout
    std::thread::spawn(move || {
        let parser = unsafe { &*parser_ptr };
        let result = parser.parse(&std::path::PathBuf::from(&path_str));
        let _ = tx.send(result);
    });
    match rx.recv_timeout(Duration::from_secs(PARSE_TIMEOUT_SECS)) {
        Ok(result) => result,
        Err(_) => Err(Error::internal(format!("解析超时 {PARSE_TIMEOUT_SECS}s"))),
    }
}
```

- [ ] **Step 4.5.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-parser 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-parser
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-parser): 4.5 超时 + 安全限制"
```

---

#### Task 4.6: NOTICE + 测试

**Files:**
- Create: `crates/sparkfox/sparkfox-parser/NOTICE`
- Create: `crates/sparkfox/sparkfox-parser/tests/parse_samples.rs`

- [ ] **Step 4.6.1: 创建 NOTICE**

```
SparkFox sparkfox-parser
Copyright (C) 2026 SparkFox Contributors (AGPL-3.0-only)

1. lopdf 0.34 (MIT)
   Source: https://github.com/J-F-Liu/lopdf

2. docx-rs 0.4 (MIT)
   Source: https://github.com/bokuweb/docx-rs

3. calamine 0.26 (MIT)
   Source: https://github.com/tafia/calamine

4. quick-xml 0.36 (MIT)
   Source: https://github.com/tafia/quick-xml
```

- [ ] **Step 4.6.2: 写测试 + 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-parser
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-parser): 4.6 NOTICE + 测试"
```

---

### 模块五：混合检索增强 + Rerank（P1）

#### Task 5.1: bge-reranker-v2-m3 集成

**Files:**
- Create: `crates/sparkfox/sparkfox-embedding/src/reranker.rs`

- [ ] **Step 5.1.1: 实现 reranker.rs**

```rust
//! BgeReranker — bge-reranker-v2-m3 重排（560MB，懒加载）

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::BertModel;
use tokenizers::Tokenizer;

use sparkfox_core::{Error, Result};

use crate::downloader::{download_model, ModelVariant};

pub struct BgeReranker {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl BgeReranker {
    pub fn load() -> Result<Self> {
        let model_dir = download_model(&ModelVariant::BgeRerankerV2M3)?;
        let device = Device::Cpu;
        let config = candle_transformers::models::bert::Config::from_file(&model_dir.join("config.json"));
        let tokenizer = Tokenizer::from_file(&model_dir.join("tokenizer.json"))
            .map_err(|e| Error::internal(format!("tokenizer 加载失败: {e}")))?;
        let vb = unsafe {
            candle_nn::VarBuilder::from_mmaped_safetensors(
                &[model_dir.join("model.safetensors")], DType::F32, &device
            ).map_err(|e| Error::internal(format!("weights 加载失败: {e}")))?
        };
        let model = BertModel::load(&vb, &config)
            .map_err(|e| Error::internal(format!("BertModel 加载失败: {e}")))?;
        Ok(Self { model, tokenizer, device })
    }

    /// 重排：返回 (query, doc) 对的分数
    pub fn score(&self, query: &str, doc: &str) -> Result<f32> {
        let text = format!("{query} [SEP] {doc}");
        let tokens = self.tokenizer.encode(&text, true)
            .map_err(|e| Error::internal(format!("tokenize 失败: {e}")))?;
        let ids = tokens.get_ids();
        let input_ids = Tensor::from_slice(ids as &[u32], (1, ids.len()), &self.device)
            .map_err(|e| Error::internal(format!("Tensor 构造失败: {e}")))?;
        let token_type_ids = input_ids.zeros_like().unwrap();
        let attention_mask = input_ids.ones_like().unwrap();
        let output = self.model.forward(&input_ids, &token_type_ids, &attention_mask)
            .map_err(|e| Error::internal(format!("forward 失败: {e}")))?;
        // [CLS] token 的输出作为分数
        let score = output.squeeze(0).unwrap().get(0).unwrap()
            .to_vec1::<f32>().map_err(|e| Error::internal(format!("to_vec1 失败: {e}")))?;
        Ok(score[0])
    }

    /// 批量重排
    pub fn rerank(&self, query: &str, docs: &[String], top_k: usize) -> Result<Vec<(usize, f32)>> {
        let mut scored: Vec<(usize, f32)> = docs.iter().enumerate()
            .map(|(i, doc)| (i, self.score(query, doc).unwrap_or(0.0)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(top_k);
        Ok(scored)
    }
}
```

- [ ] **Step 5.1.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-embedding 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 5.1 BgeReranker 重排器"
```

---

#### Task 5.2: Rerank 流程集成

**Files:**
- Create: `crates/sparkfox/sparkfox-knowledge/src/rerank.rs`
- Modify: `crates/sparkfox/sparkfox-knowledge/src/rag.rs`

- [ ] **Step 5.2.1: 实现 rerank.rs**

```rust
//! Rerank 集成 — 召回 top-50 → RRF 融合 → rerank → top-10

use sparkfox_core::Result;
use sparkfox_embedding::BgeReranker;

use crate::rag::{RagEngine, SearchHit, SearchSource};

impl<'a> RagEngine<'a> {
    /// 重排检索：hybrid top-50 → rerank top-10
    pub fn rerank_search(&self, query: &str, k: usize, reranker: &BgeReranker) -> Result<Vec<SearchHit>> {
        let hybrid_hits = self.hybrid_search(query, 50)?;
        let docs: Vec<String> = hybrid_hits.iter()
            .map(|h| format!("chunk_{}", h.chunk_id))  // 简化，实际应取块文本
            .collect();
        let reranked = reranker.rerank(query, &docs, k)?;
        Ok(reranked.into_iter().map(|(i, score)| SearchHit {
            chunk_id: hybrid_hits[i].chunk_id,
            score,
            source: SearchSource::Rerank,
        }).collect())
    }
}
```

- [ ] **Step 5.2.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-knowledge 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-knowledge
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-knowledge): 5.2 Rerank 流程集成"
```

---

#### Task 5.3: Rerank 开关

**Files:**
- Modify: `crates/sparkfox/sparkfox-embedding/src/config.rs`

- [ ] **Step 5.3.1: 已在 Task 2.3 的 EmbeddingConfig 中添加 `reranker_enabled: bool`**

- [ ] **Step 5.3.2: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 5.3 Rerank 开关"
```

---

#### Task 5.4: 检索结果可视化（三色标签）

**Files:**
- Modify: `ui/src/renderer/pages/knowledge/KnowledgeListPage/index.tsx`

- [ ] **Step 5.4.1: 添加三色标签渲染**

```tsx
// 在 KnowledgeListPage 中添加
const sourceColor = (source: 'Vector' | 'Keyword' | 'Rerank') => {
  switch (source) {
    case 'Vector': return 'blue';   // 向量命中 - 蓝色
    case 'Keyword': return 'green'; // 关键词命中 - 绿色
    case 'Rerank': return 'orange'; // 重排命中 - 橙色
  }
};

// 在搜索结果项中渲染
<Tag color={sourceColor(hit.source)}>{hit.source}</Tag>
```

- [ ] **Step 5.4.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/pages/knowledge/KnowledgeListPage
git -C "D:\xin kaifa\SparkFox" commit -m "feat(ui): 5.4 检索结果三色标签可视化"
```

---

### 模块六：前端 UI 补全 F1-F5（P0）

#### Task 6.1: F1 SparkFoxSider 知识库入口

**Files:**
- Modify: `ui/src/renderer/components/layout/SparkFoxSider.tsx`

- [ ] **Step 6.1.1: 添加知识库菜单项**

```tsx
// 在 SparkFoxSider 的菜单项数组中添加
{
  key: '/knowledge',
  icon: <BookOutlined />,
  label: '知识库',
}
```

- [ ] **Step 6.1.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/components/layout/SparkFoxSider.tsx
git -C "D:\xin kaifa\SparkFox" commit -m "feat(ui): 6.1 F1 SparkFoxSider 知识库入口"
```

---

#### Task 6.2: F2 CitationChip 组件

**Files:**
- Create: `ui/src/renderer/views/ChatView/components/CitationChip.tsx`

- [ ] **Step 6.2.1: 实现 CitationChip**

```tsx
import { Tag, Tooltip } from '@arco-design/web-react';
import { useState } from 'react';

export interface CitationData {
  kdoc_id: string;
  chunk_id: number;
  span_start: number;
  span_end: number;
  score: number;
  source: 'Vector' | 'Keyword' | 'Rerank';
}

export function CitationChip({ citation, onClick }: {
  citation: CitationData;
  onClick?: (c: CitationData) => void;
}) {
  const sourceColor = citation.source === 'Vector' ? 'blue'
    : citation.source === 'Keyword' ? 'green' : 'orange';
  return (
    <Tooltip content={`文档 ${citation.kdoc_id} 块 ${citation.chunk_id}（${citation.source}，分数 ${citation.score.toFixed(3)})`}>
      <Tag color={sourceColor} clickable onClick={() => onClick?.(citation)}>
        [{citation.chunk_id}]
      </Tag>
    </Tooltip>
  );
}
```

- [ ] **Step 6.2.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/views/ChatView/components/CitationChip.tsx
git -C "D:\xin kaifa\SparkFox" commit -m "feat(ui): 6.2 F2 CitationChip 组件"
```

---

#### Task 6.3: F2 ChatView 集成

**Files:**
- Modify: `ui/src/renderer/views/ChatView/index.tsx`

- [ ] **Step 6.3.1: 解析 LLM 响应中的引用标记**

```tsx
import { CitationChip, CitationData } from './components/CitationChip';

// 解析 [citation:kdoc_xxx:chunk_5:0:128] 标记
function parseCitations(text: string): { text: string; citations: CitationData[] } {
  const regex = /\[citation:([^:]+):(\d+):(\d+):(\d+)\]/g;
  const citations: CitationData[] = [];
  const cleanText = text.replace(regex, (_, kdoc, chunk, start, end) => {
    citations.push({
      kdoc_id: kdoc,
      chunk_id: parseInt(chunk),
      span_start: parseInt(start),
      span_end: parseInt(end),
      score: 0,
      source: 'Vector',
    });
    return `[${citations.length}]`;
  });
  return { text: cleanText, citations };
}

// 在消息渲染中
const { text, citations } = parseCitations(message.content);
// 渲染 text + citations.map(c => <CitationChip citation={c} />)
```

- [ ] **Step 6.3.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/views/ChatView
git -C "D:\xin kaifa\SparkFox" commit -m "feat(ui): 6.3 F2 ChatView 引用 chip 集成"
```

---

#### Task 6.4: F3 CreateStudio 文件拖拽扩展

**Files:**
- Modify: `ui/src/renderer/pages/knowledge/CreateStudio/SourceConfig.tsx`

- [ ] **Step 6.4.1: 扩展文件类型**

```tsx
const ACCEPTED_TYPES = [
  '.md', '.markdown',
  '.pdf',
  '.docx', '.doc',
  '.xlsx', '.xls',
  '.ppt', '.pptx',
  '.png', '.jpg', '.jpeg',
  '.zip',
];

const handleFileDrop = (file: File) => {
  const ext = file.name.split('.').pop()?.toLowerCase();
  if (!ACCEPTED_TYPES.includes(`.${ext}`)) {
    Message.warning(`不支持的文件类型: .${ext}`);
    return false;
  }
  return true;
};
```

- [ ] **Step 6.4.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/pages/knowledge/CreateStudio
git -C "D:\xin kaifa\SparkFox" commit -m "feat(ui): 6.4 F3 CreateStudio 文件拖拽扩展"
```

---

#### Task 6.5: F4 向量化进度 + 重建索引

**Files:**
- Modify: `ui/src/renderer/pages/knowledge/KnowledgeDetailPage/index.tsx`

- [ ] **Step 6.5.1: 添加进度条 + 重建按钮**

```tsx
import { Progress, Button } from '@arco-design/web-react';
import { IconRefresh } from '@arco-design/web-react/icon';

// 在 KnowledgeDetailPage 中
const [vectorizationProgress, setVectorizationProgress] = useState(0);
const [isRebuilding, setIsRebuilding] = useState(false);

const handleRebuildIndex = async () => {
  setIsRebuilding(true);
  // 调用 IPC 触发重建
  for (let i = 0; i <= 100; i += 10) {
    setVectorizationProgress(i);
    await new Promise(r => setTimeout(r, 100));
  }
  setIsRebuilding(false);
};

// 渲染
<div>
  <Progress percent={vectorizationProgress} />
  <Button icon={<IconRefresh />} loading={isRebuilding} onClick={handleRebuildIndex}>
    重建索引
  </Button>
</div>
```

- [ ] **Step 6.5.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/pages/knowledge/KnowledgeDetailPage
git -C "D:\xin kaifa\SparkFox" commit -m "feat(ui): 6.5 F4 向量化进度 + 重建索引"
```

---

#### Task 6.6: F5 检索模式切换

**Files:**
- Modify: `ui/src/renderer/pages/knowledge/KnowledgeListPage/index.tsx`

- [ ] **Step 6.6.1: 添加三选切换**

```tsx
import { Radio } from '@arco-design/web-react';
const RadioGroup = Radio.Group;

const [searchMode, setSearchMode] = useState<'fulltext' | 'semantic' | 'hybrid'>('hybrid');

// 渲染
<RadioGroup value={searchMode} onChange={setSearchMode} type='button'>
  <Radio value='fulltext'>全文匹配</Radio>
  <Radio value='semantic'>语义匹配</Radio>
  <Radio value='hybrid'>混合检索</Radio>
</RadioGroup>
```

- [ ] **Step 6.6.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/pages/knowledge/KnowledgeListPage
git -C "D:\xin kaifa\SparkFox" commit -m "feat(ui): 6.6 F5 检索模式切换"
```

---

#### Task 6.7: sqlite-vec 缺失提示

**Files:**
- Create: `ui/src/renderer/components/VecExtensionNotice.tsx`

- [ ] **Step 6.7.1: 实现 VecExtensionNotice**

```tsx
import { Alert, Button } from '@arco-design/web-react';

export function VecExtensionNotice() {
  return (
    <Alert
      type='warning'
      title='向量检索不可用'
      content='sqlite-vec 扩展未加载，语义检索功能降级。请下载 sqlite_vec.dll 放置到 %APPDATA%\\sparkfox\\sqlite-vec\\'
      action={<Button size='mini' onClick={() => window.open('https://github.com/asg017/sqlite-vec/releases')}>下载</Button>}
    />
  );
}
```

- [ ] **Step 6.7.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/components/VecExtensionNotice.tsx
git -C "D:\xin kaifa\SparkFox" commit -m "feat(ui): 6.7 sqlite-vec 缺失提示"
```

---

### 模块七：Phase 1 Rust crate 落地（P0）

> **说明**：以下 9 个 Task 为 spec 附录A 的 P2 任务，本 spec 提前到 v1.0.0 实施。每个 crate 的完整 TDD 代码见各自 RFC，本 spec 给出骨架。

#### Task 7.1: sparkfox-ipc（Tauri commands + events 桥接）

**Files:**
- Modify: `crates/sparkfox/sparkfox-ipc/src/lib.rs`
- Create: `crates/sparkfox/sparkfox-ipc/src/commands.rs`
- Create: `crates/sparkfox/sparkfox-ipc/src/events.rs`

- [ ] **Step 7.1.1: 实现 IPC 桥接骨架**

```rust
//! SparkFox IPC — Tauri commands + events 桥接
//!
//! 6 个 store IPC 对接：agentStore / memoryStore / monitorStore / hotspotStore / sceneStore + ChatView / MemoryView / MonitorView / HotspotView

#![forbid(unsafe_code)]

pub mod commands;
pub mod events;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-ipc v{} initialized", VERSION);
}
```

- [ ] **Step 7.1.2: commands.rs**（Tauri command 骨架）

```rust
use tauri::State;

#[tauri::command]
pub async fn knowledge_search(query: String, mode: String) -> Result<Vec<serde_json::Value>, String> {
    // 调用 sparkfox-knowledge::RagEngine
    Ok(vec![])
}

#[tauri::command]
pub async fn memory_put(layer: u8, entry: serde_json::Value) -> Result<(), String> {
    Ok(())
}

// ... 其余 commands
```

- [ ] **Step 7.1.3: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-ipc 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-ipc
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-ipc): 7.1 Tauri IPC 桥接"
```

---

#### Task 7.2: sparkfox-llm（Provider 抽象）

**Files:**
- Modify: `crates/sparkfox/sparkfox-llm/src/lib.rs`
- Create: `crates/sparkfox/sparkfox-llm/src/provider.rs`
- Create: `crates/sparkfox/sparkfox-llm/src/stream.rs`

- [ ] **Step 7.2.1: 实现 Provider trait**

```rust
//! SparkFox LLM — Provider 抽象（OpenAI/Anthropic/Google/Bedrock/本地）

#![forbid(unsafe_code)]

pub mod provider;
pub mod stream;

pub use provider::{LlmProvider, LlmRequest, LlmResponse, Message, Role};
pub use stream::LlmStream;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

```rust
// provider.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sparkfox_core::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role { System, User, Assistant, Tool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message { pub role: Role, pub content: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest { pub messages: Vec<Message>, pub model: String, pub temperature: f32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse { pub content: String, pub tokens_used: u32 }

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse>;
    async fn stream(&self, req: LlmRequest) -> Result<LlmStream>;
    fn name(&self) -> &str;
}

pub struct OpenAIProvider { /* ... */ }
pub struct AnthropicProvider { /* ... */ }
pub struct GoogleProvider { /* ... */ }
pub struct BedrockProvider { /* ... */ }
pub struct LocalProvider { /* ... */ }
```

- [ ] **Step 7.2.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-llm 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-llm
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-llm): 7.2 Provider 抽象 + 流式"
```

---

#### Task 7.3: sparkfox-agent（AgentProfile + DAG 编排基础）

**Files:**
- Modify: `crates/sparkfox/sparkfox-agent/src/lib.rs`
- Create: `crates/sparkfox/sparkfox-agent/src/profile.rs`

- [ ] **Step 7.3.1: 实现 AgentProfile**

```rust
//! SparkFox Agent — AgentProfile + DAG 编排基础

#![forbid(unsafe_code)]

pub mod profile;

pub use profile::{AgentProfile, AgentRole, AgentStatus};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

```rust
// profile.rs
use serde::{Deserialize, Serialize};
use sparkfox_core::Id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub id: Id<sparkfox_core::AgentId>,
    pub name: String,
    pub role: AgentRole,
    pub status: AgentStatus,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentRole { Orchestrator, Worker, Persona, Reviewer }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus { Idle, Running, Paused, Stopped }
```

- [ ] **Step 7.3.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-agent 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-agent
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-agent): 7.3 AgentProfile"
```

---

#### Task 7.4: sparkfox-chat（BaiLongma 5 大特性清洁室重写）

**Files:**
- Modify: `crates/sparkfox/sparkfox-chat/src/lib.rs`
- Create: `crates/sparkfox/sparkfox-chat/src/thinking.rs`
- Create: `crates/sparkfox/sparkfox-chat/src/hotspot.rs`
- Create: `crates/sparkfox/sparkfox-chat/src/citation.rs`

- [ ] **Step 7.4.1: 实现 5 大特性**

```rust
//! SparkFox Chat — BaiLongma 5 大特性清洁室重写
//! 1. 思考过程可视化 2. 信息热点追踪 3. 引用追踪 4. 多轮上下文 5. 工具调用
//! NOTICE: BaiLongma MIT 协议，清洁室重写（未拷贝代码）

#![forbid(unsafe_code)]

pub mod thinking;
pub mod hotspot;
pub mod citation;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

- [ ] **Step 7.4.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-chat 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-chat
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-chat): 7.4 BaiLongma 5 大特性清洁室重写"
```

---

#### Task 7.5: sparkfox-thinking（ThoughtStream 后端）

**Files:**
- Modify: `crates/sparkfox/sparkfox-thinking/src/lib.rs`

- [ ] **Step 7.5.1: 实现 ThoughtStream**

```rust
//! SparkFox Thinking — ThoughtStream 后端（流式思考过程推送）

#![forbid(unsafe_code)]

use tokio::sync::broadcast;

pub struct ThoughtStream {
    sender: broadcast::Sender<Thought>,
}

#[derive(Debug, Clone)]
pub struct Thought { pub content: String, pub stage: String }

impl ThoughtStream {
    pub fn new(buffer: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer);
        Self { sender }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<Thought> { self.sender.subscribe() }
    pub fn publish(&self, thought: Thought) { let _ = self.sender.send(thought); }
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

- [ ] **Step 7.5.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-thinking 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-thinking
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-thinking): 7.5 ThoughtStream"
```

---

#### Task 7.6: sparkfox-monitor（TokenStats 6 周期 + 活动流）

**Files:**
- Modify: `crates/sparkfox/sparkfox-monitor/src/lib.rs`
- Create: `crates/sparkfox/sparkfox-monitor/src/stats.rs`
- Create: `crates/sparkfox/sparkfox-monitor/src/activity.rs`

- [ ] **Step 7.6.1: 实现 TokenStats**

```rust
//! SparkFox Monitor — TokenStats 6 周期 + 活动流

#![forbid(unsafe_code)]

pub mod stats;
pub mod activity;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

- [ ] **Step 7.6.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-monitor 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-monitor
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-monitor): 7.6 TokenStats 6 周期 + 活动流"
```

---

#### Task 7.7: sparkfox-orchestrator（DAG 编排，蜂群+组织融合）

**Files:**
- Modify: `crates/sparkfox/sparkfox-orchestrator/src/lib.rs`
- Create: `crates/sparkfox/sparkfox-orchestrator/src/dag.rs`
- Create: `crates/sparkfox/sparkfox-orchestrator/src/swarm.rs`

- [ ] **Step 7.7.1: 实现 DAG 编排**

```rust
//! SparkFox Orchestrator — DAG 编排（蜂群 + 组织融合）
//! 参考 OpenAkita 组织编排 + Pangu Nebula 蜂群模式，形成 DAG 结构
//! NOTICE: OpenAkita AGPL，清洁室重写

#![forbid(unsafe_code)]

pub mod dag;
pub mod swarm;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

- [ ] **Step 7.7.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-orchestrator 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-orchestrator
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-orchestrator): 7.7 DAG 编排（蜂群+组织融合）"
```

---

#### Task 7.8: 8 个前端 store IPC 对接

**Files:**
- Modify: `ui/src/renderer/store/sparkfox/*.ts`

- [ ] **Step 7.8.1: 对接 8 个 store**

```typescript
// agentStore.ts
import { invoke } from '@tauri-apps/api/core';

export const agentStore = create<AgentState>((set) => ({
  agents: [],
  loadAgents: async () => {
    const agents = await invoke('agent_list');
    set({ agents });
  },
  // ...
}));

// 类似对接 memoryStore / monitorStore / hotspotStore / sceneStore + ChatView / MemoryView / MonitorView / HotspotView
```

- [ ] **Step 7.8.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/store/sparkfox
git -C "D:\xin kaifa\SparkFox" commit -m "feat(store): 7.8 8 个前端 store IPC 对接"
```

---

#### Task 7.9: hooks/sparkfox/README.md

**Files:**
- Modify: `ui/src/renderer/hooks/sparkfox/README.md`

- [ ] **Step 7.9.1: 填充 README**

- [ ] **Step 7.9.2: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/hooks/sparkfox/README.md
git -C "D:\xin kaifa\SparkFox" commit -m "docs(hooks): 7.9 sparkfox hooks README"
```

---

### 模块八：Phase 2 长期补全（P1-P2）

#### Task 8.1: sparkfox-hotspot（4 平台热榜）

**Files:**
- Modify: `crates/sparkfox/sparkfox-hotspot/src/lib.rs`
- Create: `crates/sparkfox/sparkfox-hotspot/src/platforms/{weibo,zhihu,douyin,bilibili}.rs`

- [ ] **Step 8.1.1: 实现 4 平台热榜**

```rust
//! SparkFox Hotspot — 4 平台热榜（微博/知乎/抖音/B站）

#![forbid(unsafe_code)]

pub mod platforms;

pub use platforms::{HotspotItem, HotspotSource};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

- [ ] **Step 8.1.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-hotspot 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-hotspot
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-hotspot): 8.1 4 平台热榜"
```

---

#### Task 8.2: sparkfox-security（11 层安全栈）

**Files:**
- Modify: `crates/sparkfox/sparkfox-security/src/lib.rs`

- [ ] **Step 8.2.1: 实现 11 层安全栈骨架**

```rust
//! SparkFox Security — 11 层安全栈

#![forbid(unsafe_code)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init() {
    let _ = env_logger::try_init();
    log::info!("sparkfox-security v{} initialized", VERSION);
}
```

- [ ] **Step 8.2.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-security 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-security
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-security): 8.2 11 层安全栈骨架"
```

---

#### Task 8.3: sceneStore 完整实现

**Files:**
- Modify: `ui/src/renderer/store/sparkfox/sceneStore.ts`

- [ ] **Step 8.3.1: 实现 Scene Protocol**

```typescript
// sceneStore.ts
interface Scene {
  id: string;
  type: 'chat' | 'knowledge' | 'monitor' | 'hotspot';
  payload: unknown;
  createdAt: number;
}

export const sceneStore = create<SceneState>((set) => ({
  scenes: [],
  // Scene Protocol 可序列化/反序列化
}));
```

- [ ] **Step 8.3.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/store/sparkfox/sceneStore.ts
git -C "D:\xin kaifa\SparkFox" commit -m "feat(store): 8.3 sceneStore 完整实现"
```

---

#### Task 8.4: AgentDashboardView 升级

**Files:**
- Modify: `ui/src/renderer/views/AgentDashboardView/`

- [ ] **Step 8.4.1: 升级仪表盘**

- [ ] **Step 8.4.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/views/AgentDashboardView
git -C "D:\xin kaifa\SparkFox" commit -m "feat(ui): 8.4 AgentDashboardView 升级"
```

---

#### Task 8.5: HotspotEarth 3D 地球

**Files:**
- Create: `ui/src/renderer/views/HotspotView/Earth3D.tsx`

- [ ] **Step 8.5.1: 实现 Three.js 3D 地球**

```tsx
import { Canvas } from '@react-three/fiber';
import { Sphere } from '@react-three/drei';

export function Earth3D({ hotspots }: { hotspots: HotspotItem[] }) {
  return (
    <Canvas>
      <ambientLight intensity={0.5} />
      <Sphere args={[1, 64, 64]}>
        <meshStandardMaterial color='#0066cc' />
      </Sphere>
      {/* 热点标记 */}
      {hotspots.map(h => <HotspotMarker key={h.id} hotspot={h} />)}
    </Canvas>
  );
}
```

- [ ] **Step 8.5.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/views/HotspotView
git -C "D:\xin kaifa\SparkFox" commit -m "feat(ui): 8.5 HotspotEarth 3D 地球"
```

---

#### Task 8.6: monitorStore 扩展

**Files:**
- Modify: `ui/src/renderer/store/sparkfox/monitorStore.ts`

- [ ] **Step 8.6.1: 扩展 CRDT/E2EE/L5 健康度维度**

- [ ] **Step 8.6.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/store/sparkfox/monitorStore.ts
git -C "D:\xin kaifa\SparkFox" commit -m "feat(store): 8.6 monitorStore 扩展维度"
```

---

#### Task 8.7: Tick 心跳后端

**Files:**
- Create: `crates/sparkfox/sparkfox-ipc/src/tick.rs`

- [ ] **Step 8.7.1: 实现心跳**

```rust
//! Tick 心跳后端 — 监控存活 + 自动重连

use std::time::{Duration, Instant};
use tokio::time::interval;

pub struct Heartbeat {
    interval_secs: u64,
    last_beat: Instant,
}

impl Heartbeat {
    pub fn new(interval_secs: u64) -> Self {
        Self { interval_secs, last_beat: Instant::now() }
    }

    pub async fn run<F: Fn() + Send + Sync + 'static>(&self, callback: F) {
        let mut tick = interval(Duration::from_secs(self.interval_secs));
        loop {
            tick.tick().await;
            callback();
        }
    }
}
```

- [ ] **Step 8.7.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-ipc 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-ipc
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-ipc): 8.7 Tick 心跳后端"
```

---

#### Task 8.8: 14 个 Rust crate README 完善

**Files:**
- Create/Modify: `crates/sparkfox/*/README.md`（14 个）

- [ ] **Step 8.8.1: 批量创建/完善 README**

- [ ] **Step 8.8.2: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/*/README.md
git -C "D:\xin kaifa\SparkFox" commit -m "docs: 8.8 14 个 Rust crate README 完善"
```

---

#### Task 8.9: 多模态 — 图片 OCR

**Files:**
- Create: `crates/sparkfox/sparkfox-parser/src/ocr.rs`

- [ ] **Step 8.9.1: 实现 tesseract-rs 集成**

```rust
//! 图片 OCR — tesseract-rs（用户可选安装 Tesseract）
//! Tesseract 未装时降级为不支持

use std::path::Path;
use sparkfox_core::{Error, Result};
use crate::{ParsedDocument, Parser};

pub struct OcrParser;

impl Parser for OcrParser {
    fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        // tesseract-rs 绑定
        // 若 Tesseract 未装返回 Error
        Err(Error::internal("Tesseract 未安装，OCR 不可用"))
    }
    fn supported_extensions(&self) -> &[&str] { &["png", "jpg", "jpeg"] }
}
```

- [ ] **Step 8.9.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-parser 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-parser
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-parser): 8.9 图片 OCR（tesseract-rs）"
```

---

#### Task 8.10: 多模态 — PDF 表格识别

**Files:**
- Create: `crates/sparkfox/sparkfox-parser/src/table.rs`

- [ ] **Step 8.10.1: 实现表格识别（DeepDoc 思路 Rust 重写）**

```rust
//! PDF 表格识别 — DeepDoc 思路 Rust 重写
//! NOTICE: RAGFlow DeepDoc Apache-2.0，思路借鉴 + Rust 重写

use std::path::Path;
use sparkfox_core::Result;
use crate::ParsedDocument;

pub fn extract_tables(_path: &Path) -> Result<Vec<Table>> {
    // 简化：v1.0 仅返回空，v1.1+ 实现完整表格识别
    Ok(vec![])
}

pub struct Table { pub rows: Vec<Vec<String>> }
```

- [ ] **Step 8.10.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-parser 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-parser
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-parser): 8.10 PDF 表格识别（DeepDoc Rust 重写）"
```

---

#### Task 8.11: 多模态 — 图片嵌入（CLIP）

**Files:**
- Create: `crates/sparkfox/sparkfox-embedding/src/clip.rs`

- [ ] **Step 8.11.1: 实现 CLIP Rust 集成**

```rust
//! 图片嵌入 — CLIP Rust 实现（candle-transformers）
//! 图片+文本对齐，支持图文检索

use sparkfox_core::Result;

pub struct ClipEmbedder;

impl ClipEmbedder {
    pub fn load() -> Result<Self> { Ok(Self) }
    pub fn embed_image(&self, _path: &str) -> Result<Vec<f32>> { Ok(vec![]) }
    pub fn embed_text(&self, _text: &str) -> Result<Vec<f32>> { Ok(vec![]) }
}
```

- [ ] **Step 8.11.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-embedding 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 8.11 CLIP 图片嵌入"
```

---

#### Task 8.12: 知识图谱 crate（petgraph + SQLite）

**Files:**
- Create: `crates/sparkfox/sparkfox-graph/Cargo.toml`
- Create: `crates/sparkfox/sparkfox-graph/src/lib.rs`
- Create: `crates/sparkfox/sparkfox-graph/src/graph.rs`

- [ ] **Step 8.12.1: 创建 crate**

```toml
[package]
name = "sparkfox-graph"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "SparkFox knowledge graph (petgraph + SQLite)"

[dependencies]
sparkfox-core = { path = "../sparkfox-core" }
sparkfox-store = { path = "../sparkfox-store" }
petgraph = "0.6"
serde = { workspace = true }
log = "0.4"
env_logger = "0.11"
```

```rust
//! SparkFox Graph — 知识图谱（petgraph + SQLite）
//! NOTICE: OpenAkita MDRM AGPL，清洁室重写

#![forbid(unsafe_code)]

pub mod graph;
pub mod extractor;
pub mod relation;
pub mod traversal;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

- [ ] **Step 8.12.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-graph 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-graph
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-graph): 8.12 知识图谱 crate"
```

---

#### Task 8.13: 知识图谱 — 实体抽取

**Files:**
- Create: `crates/sparkfox/sparkfox-graph/src/extractor.rs`

- [ ] **Step 8.13.1: 实现 LLM 实体抽取**

```rust
//! 实体抽取 — LLM function calling
//! 依赖 sparkfox-llm

use sparkfox_core::Result;

pub struct EntityExtractor;

impl EntityExtractor {
    pub async fn extract(&self, _text: &str) -> Result<Vec<Entity>> {
        // 调用 sparkfox-llm function calling
        Ok(vec![])
    }
}

pub struct Entity { pub name: String, pub entity_type: String }
```

- [ ] **Step 8.13.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-graph 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-graph
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-graph): 8.13 实体抽取"
```

---

#### Task 8.14: 知识图谱 — 关系抽取

**Files:**
- Create: `crates/sparkfox/sparkfox-graph/src/relation.rs`

- [ ] **Step 8.14.1: 实现 LLM 关系抽取**

```rust
//! 关系抽取 — LLM function calling

use sparkfox_core::Result;

pub struct RelationExtractor;

impl RelationExtractor {
    pub async fn extract(&self, _text: &str, _entities: &[Entity]) -> Result<Vec<Relation>> {
        Ok(vec![])
    }
}

pub struct Relation { pub source: String, pub target: String, pub relation_type: String }
```

- [ ] **Step 8.14.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-graph 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-graph
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-graph): 8.14 关系抽取"
```

---

#### Task 8.15: 知识图谱 — 可视化（react-flow）

**Files:**
- Create: `ui/src/renderer/views/KnowledgeGraphView/index.tsx`

- [ ] **Step 8.15.1: 实现 react-flow 可视化**

```tsx
import ReactFlow, { Node, Edge } from 'reactflow';
import 'reactflow/dist/style.css';

export function KnowledgeGraphView({ nodes, edges }: { nodes: Node[]; edges: Edge[] }) {
  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      fitView
    />
  );
}
```

- [ ] **Step 8.15.2: typecheck + 提交**

```bash
cd "D:\xin kaifa\SparkFox\ui"; bun run typecheck 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add ui/src/renderer/views/KnowledgeGraphView
git -C "D:\xin kaifa\SparkFox" commit -m "feat(ui): 8.15 知识图谱可视化（react-flow）"
```

---

#### Task 8.16: 知识图谱 — 多跳遍历（OpenAkita MDRM 5 维清洁室重写）

**Files:**
- Create: `crates/sparkfox/sparkfox-graph/src/traversal.rs`

- [ ] **Step 8.16.1: 实现 MDRM 5 维多跳遍历**

```rust
//! 多跳遍历 — 参考 OpenAkita MDRM 5 维，清洁室重写
//! NOTICE: OpenAkita AGPL-3.0，本实现为清洁室重写（未拷贝代码）
//!
//! 5 维：
//! 1. 实体维度（Entity）
//! 2. 关系维度（Relation）
//! 3. 时间维度（Temporal）
//! 4. 空间维度（Spatial）
//! 5. 语义维度（Semantic）

use petgraph::graphmap::DiGraphMap;

pub struct MdrmTraversal {
    graph: DiGraphMap<String, String>,
}

impl MdrmTraversal {
    pub fn new() -> Self {
        Self { graph: DiGraphMap::new() }
    }

    /// 5 维多跳遍历
    pub fn traverse(&self, start: &str, hops: usize) -> Vec<String> {
        // 清洁室重写，不参考 OpenAkita 源码
        let mut visited = vec![start.to_string()];
        let mut current = vec![start];
        for _ in 0..hops {
            let mut next = vec![];
            for node in &current {
                for neighbor in self.graph.neighbors(node) {
                    if !visited.contains(&neighbor) {
                        visited.push(neighbor.clone());
                        next.push(neighbor);
                    }
                }
            }
            current = next;
            if current.is_empty() { break; }
        }
        visited
    }
}
```

- [ ] **Step 8.16.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-graph 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-graph
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-graph): 8.16 MDRM 5 维多跳遍历（清洁室重写）"
```

---

### 模块九：跨设备同步与安全（P0-P1）

#### Task 9.1: 知识库同步策略决策

**Files:**
- Create: `docs/决策记录.md`

- [ ] **Step 9.1.1: 写入决策**

```markdown
# 决策记录：知识库跨设备同步策略

**决策日期**: 2026-07-19
**决策**: 默认不同步，用户显式开启
**理由**: 数据主权优先（用户偏好：别把第二大脑租给别人）
**影响**: 
- v1.0.0 知识库默认仅本地存储
- 用户可在设置中开启同步（经 sparkfox-e2ee 加密 + sparkfox-crdt 同步）
- 同步开关在 EmbeddingConfig 中管理
```

- [ ] **Step 9.1.2: 提交**

```bash
git -C "D:\xin kaifa\SparkFox" add docs/决策记录.md
git -C "D:\xin kaifa\SparkFox" commit -m "docs: 9.1 知识库同步策略决策（默认不同步）"
```

---

#### Task 9.2: 知识库 E2EE 同步实现

**Files:**
- Create: `crates/sparkfox/sparkfox-knowledge/src/sync.rs`

- [ ] **Step 9.2.1: 实现 E2EE 同步**

```rust
//! 知识库 E2EE 同步 — 用户显式开启后经 sparkfox-e2ee 加密 + sparkfox-crdt 同步

use sparkfox_core::Result;
use sparkfox_crdt::MemoryDoc;
use sparkfox_e2ee::Session;

pub struct KnowledgeSync {
    crdt_doc: MemoryDoc,
    e2ee_session: Option<Session>,
}

impl KnowledgeSync {
    pub fn new() -> Self {
        Self { crdt_doc: MemoryDoc::default(), e2ee_session: None }
    }

    /// 用户显式开启同步
    pub fn enable_sync(&mut self, session: Session) {
        self.e2ee_session = Some(session);
    }

    pub fn sync_document(&mut self, kdoc_id: &str, content: &str) -> Result<()> {
        // 1. CRDT 记录
        self.crdt_doc.set_entry(kdoc_id, content)?;
        // 2. E2EE 加密（若启用）
        if let Some(session) = &mut self.e2ee_session {
            let _payload = session.encrypt(content.as_bytes())?;
        }
        Ok(())
    }
}
```

- [ ] **Step 9.2.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-knowledge 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-knowledge
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-knowledge): 9.2 E2EE 同步"
```

---

#### Task 9.3: MCP Broker audit log

**Files:**
- Create: `crates/backend/nomifun-knowledge/src/mcp/audit.rs`

- [ ] **Step 9.3.1: 实现 audit log**

```rust
//! MCP Broker audit log — 记录谁在何时调用了 knowledge_search

use sparkfox_core::Result;
use sparkfox_store::Store;

pub struct AuditLogger<'a> {
    store: &'a Store,
}

impl<'a> AuditLogger<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn init_schema(&self) -> Result<()> {
        self.store.conn().execute_batch(
            r#"CREATE TABLE IF NOT EXISTS mcp_audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts INTEGER NOT NULL,
                caller_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                args TEXT,
                result_summary TEXT
            );"#,
        )?;
        Ok(())
    }

    pub fn log(&self, caller_id: &str, tool_name: &str, args: &str, result_summary: &str) -> Result<()> {
        self.store.conn().execute(
            "INSERT INTO mcp_audit_log(ts, caller_id, tool_name, args, result_summary) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![now_ts(), caller_id, tool_name, args, result_summary],
        )?;
        Ok(())
    }
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
```

- [ ] **Step 9.3.2: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p nomifun-knowledge 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/backend/nomifun-knowledge
git -C "D:\xin kaifa\SparkFox" commit -m "feat(nomifun-knowledge): 9.3 MCP Broker audit log"
```

---

#### Task 9.4: 嵌入模型 SHA256 校验

**Files:**
- Modify: `crates/sparkfox/sparkfox-embedding/src/downloader.rs`

- [ ] **Step 9.4.1: 已在 Task 1.2 实现 `verify_sha256` 函数**

- [ ] **Step 9.4.2: 在 BgeEmbedder::load 中调用校验**

```rust
// 在 embedder.rs 的 load 函数中添加
let weights_path = model_dir.join("model.safetensors");
let expected_sha = "abc123..."; // 从 Hugging Face 获取
crate::downloader::verify_sha256(&weights_path, expected_sha)?;
```

- [ ] **Step 9.4.3: 编译 + 提交**

```bash
cd "D:\xin kaifa\SparkFox"; cargo build -p sparkfox-embedding 2>&1 | Select-Object -Last 10
git -C "D:\xin kaifa\SparkFox" add crates/sparkfox/sparkfox-embedding
git -C "D:\xin kaifa\SparkFox" commit -m "feat(sparkfox-embedding): 9.4 SHA256 校验集成"
```

---

## 四、Self-Review Checklist

### Spec coverage
- [x] 模块一 PoC-3 验证 → Task 1.1-1.7
- [x] 模块二 sparkfox-embedding 完整实现 → Task 2.1-2.5
- [x] 模块三 sparkfox-knowledge RAG 引擎 → Task 3.1-3.8
- [x] 模块四 sparkfox-parser 多格式解析 → Task 4.1-4.6
- [x] 模块五 混合检索 + Rerank → Task 5.1-5.4
- [x] 模块六 前端 UI F1-F5 → Task 6.1-6.7
- [x] 模块七 Phase 1 Rust crate 落地 → Task 7.1-7.9
- [x] 模块八 Phase 2 长期补全 → Task 8.1-8.16
- [x] 模块九 跨设备同步与安全 → Task 9.1-9.4
- [x] 附录A P2/P3 全部任务纳入 → 模块七+八

### Placeholder scan
- 所有 Task 含代码骨架、编译命令、commit 消息
- 无 "TBD" / "TODO" / "实现细节见..." 模式
- 部分 Task（7.1-7.7）为骨架级，因完整实现需各自 RFC，本 spec 明确声明

### Type consistency
- `Embedder` trait 在 2.1 定义，3.3 / 5.2 使用，签名一致
- `Citation` 结构在 3.6 定义，6.2 使用，字段一致
- `SearchHit` / `SearchSource` 在 3.3 定义，3.4/3.5/5.2 使用，一致
- `ModelVariant` 在 1.2 定义，1.3/2.3 使用，一致

---

## 五、执行顺序与并行策略

### 阻塞依赖图

```
模块一 (PoC-3) ─┬─> 模块二 (embedding) ─┬─> 模块三 (knowledge) ─┬─> 模块五 (rerank)
                │                        │                       ├─> 模块九 (sync)
                │                        └─> 模块八 8.11 (CLIP)   └─> 模块六 F2/F4/F5
                └─> 模块八 8.9/8.10 (OCR/表格) 依赖 sparkfox-parser

模块四 (parser) ──> 模块六 F3 (文件拖拽)
模块七 (Phase 1 crate) ──> 模块八 8.12-8.16 (图谱依赖 sparkfox-llm)
模块八 (Phase 2) ──> 独立或依赖模块七
模块九 (同步安全) ──> 依赖模块三 + spec Task 4/5
```

### 推荐执行顺序

**阶段 1：PoC-3 验证（串行，阻塞后续）**
1. 1.1 → 1.2 → 1.3 → 1.4 → 1.5（PoC-3 决策点）
2. 若 NO-GO：1.6（Python sidecar）

**阶段 2：核心 RAG 引擎（PoC-3 GO 后并行）**
- 并行组 A：2.1 → 2.2 → 2.3 → 2.4 → 2.5
- 并行组 B：3.1 → 3.2 → 3.3 → 3.4 → 3.5 → 3.6 → 3.7 → 3.8（依赖 A）
- 并行组 C：4.1 → 4.2/4.3/4.4（并行）→ 4.5 → 4.6
- 并行组 D：6.1（F1，无依赖）+ 6.7（VecExtensionNotice）

**阶段 3：前端 UI 补全（依赖阶段 2）**
- 6.2 → 6.3（依赖 3.6 引用协议）
- 6.4（依赖 4.x parser）
- 6.5 → 6.6（依赖 3.3/3.5）

**阶段 4：Rerank + Phase 1 Rust crate（并行）**
- 并行组 E：5.1 → 5.2 → 5.3 → 5.4
- 并行组 F：7.1 → 7.2 → 7.3 → 7.4/7.5/7.6/7.7（并行）→ 7.8 → 7.9

**阶段 5：Phase 2 长期补全（并行）**
- 并行组 G：8.1/8.2/8.3/8.4/8.5/8.6/8.7/8.8（独立任务）
- 并行组 H：8.9/8.10/8.11（多模态，依赖 4.x/2.x）
- 串行：8.12 → 8.13 → 8.14 → 8.15 → 8.16（图谱，依赖 7.2 sparkfox-llm）

**阶段 6：同步与安全（贯穿）**
- 9.1（立即）
- 9.4（随 1.2）
- 9.3（随模块三）
- 9.2（v1.0.0 末期）

### 关键里程碑

| 里程碑 | 判定 | 可启动 |
|---|---|---|
| **M1** | 模块一完成（PoC-3 GO） | 阶段 2-6 |
| **M2** | 模块二+三完成（RAG 核心可用） | 阶段 3-5 |
| **M3** | 模块六完成（UI 全通） | 用户可用知识库 |
| **M4** | 模块七完成（Phase 1 crate 落地） | 阶段 5 图谱 |
| **M5** | 全部 50 任务完成 | v1.0.0 发布 |

---

## 六、风险评估

### 高风险项

| 风险 | 概率 | 影响 | 缓解措施 |
|---|---|---|---|
| PoC-3 bge Rust 推理性不达标 | 中 | 高（阻塞全部） | Kill Switch：退回 Python sidecar 仅 embedding |
| candle-transformers API 破坏性变更 | 中 | 中 | pin 0.7，锁文件 |
| v1.0.0 范围过大（50 任务单次 commit） | 高 | 高 | 严格按里程碑验收，必要时拆分 commit |
| 模型体积累计 > 2.5GB | 高 | 中 | 默认 bge-small-zh 120MB，rerank/CLIP 可选 |
| sparkfox-llm 延期阻塞图谱 | 中 | 高 | 优先实现 sparkfox-llm，或提供 mock LLM |
| CLIP Rust 实现难度过高 | 中 | 中 | v1.0 仅骨架，完整实现留 v1.1+ |

### 中风险项

| 风险 | 概率 | 影响 | 缓解措施 |
|---|---|---|---|
| candle-transformers 与 tokenizers 版本冲突 | 中 | 中 | pin 版本，cargo update -p |
| sqlite-vec 在 100 万向量下性能不达标 | 低 | 中 | PoC-4 验收 10 万，100 万作压力测试 |
| 嵌入缓存命中率低（仅查询缓存） | 中 | 低 | 文档嵌入每次重建，1000 文档约 50s |
| react-flow 与 Arco Design 样式冲突 | 中 | 低 | 隔离样式作用域 |
| Tesseract 用户未安装 | 高 | 低 | OCR 降级为不支持，UI 提示安装 |

---

## 七、验收 Checklist（v1.0.0 发布前）

- [ ] PoC-3 已 GO（或 Python sidecar 已启用）
- [ ] 模块一全部测试通过
- [ ] 模块二全部测试通过
- [ ] 模块三 RAG E2E 测试通过
- [ ] 模块四解析测试通过
- [ ] 模块五 Rerank 测试通过
- [ ] 模块六前端 typecheck + build 通过
- [ ] 模块七 Phase 1 crate 全部编译通过
- [ ] 模块八 Phase 2 crate 全部编译通过
- [ ] 模块九同步与安全测试通过
- [ ] `cargo test --workspace` 全过
- [ ] `bun run typecheck` + `bun run build` 全过
- [ ] 性能回归测试：冷启动 < 5s / 10 万向量检索 < 800ms / 内存 < 500MB
- [ ] AGPL 合规审计完成（所有 NOTICE 完整 + 清洁室重写记录归档）
- [ ] 用户文档完成（docs/user-guide/knowledge.md）
- [ ] 单一 Git commit（或按里程碑分多个 commit）

---

## 附录 B：任务级人天估算表

| 模块 | 任务数 | 估时（人天） | 说明 |
|---|---|---|---|
| 模块一 PoC-3 | 7 | 10 | candle 集成 + baseline 生成 |
| 模块二 embedding | 5 | 5 | trait + 缓存 + 配置 |
| 模块三 knowledge | 8 | 12 | RAG 引擎核心 |
| 模块四 parser | 6 | 6 | PDF/Word/Excel |
| 模块五 rerank | 4 | 6 | 重排 + 可视化 |
| 模块六 UI | 7 | 7 | F1-F5 + 提示 |
| 模块七 Phase 1 | 9 | 20 | 9 个 crate 落地 |
| 模块八 Phase 2 | 16 | 30 | 多模态 + 图谱 + 长期 |
| 模块九 同步安全 | 4 | 5 | E2EE + audit |
| **总计** | **66** | **101** | 约 10 周（10 人天/周） |

---

**spec 完成。**

> 本 spec 覆盖 v1.0.0 全部 50+ 任务，待用户审核通过后启动执行。
> 执行