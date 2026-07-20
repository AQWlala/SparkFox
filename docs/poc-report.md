# SparkFox Phase -1 PoC 验收报告

> **报告日期**: 2026-07-19（v1.0.0 PoC-3 实测更新：2026-07-19）
> **报告人**: SparkFox v0.2.0 / v1.0.0 自动化测试
> **PoC 周期**: v0.2.0 占位功能补全 spec A + v1.0.0 模块一 PoC-3 实测
> **总体验收结论**: ⚠️ 条件性 GO（PoC-1/2/4 单元测试通过，PoC-3 v1.0.0 实测 GO）

---

## 一、PoC 总览

| PoC # | 验证项 | 风险等级 | 状态 | Go/No-Go |
|-------|--------|:--------:|:----:|:--------:|
| PoC-1 | L5 元认知价值验证 | 🔴 P0 | ✅ 已测（单元层） | ⚠️ 条件性 GO |
| PoC-2 | automerge-rs CRDT 可行性 | 🔴 P0 | ✅ 已测 | ✅ GO |
| PoC-3 | bge Rust 推理性 + Python 一致性 | 🔴 P0 | ✅ v1.0.0 实测 | ✅ GO |
| PoC-4 | NomiFun + sqlite-vec 性能基线 | 🔴 P0 | ✅ 已测（部分） | ⚠️ 条件性 GO |

**总体 Kill Switch 规则**：
- 任一 PoC 失败 → 触发对应 Kill Switch，重新评估方案
- PoC-1 失败 → 必须重评"具备元认知"定位 + MVP 范围
- PoC-2 失败 → CRDT 推迟到 v0.5+，先做单机版
- PoC-3 失败 → 退回 Python sidecar（仅 embedding 模块）
- PoC-4 失败 → 重设性能目标 + 砍 30% 功能

---

## 二、PoC-1: L5 元认知价值验证

### 2.1 验证目标

验证 Pangu L5 元认知层（策略日志/错误模式/自评）对任务完成率、token 成本、用户满意度的实际提升价值。

### 2.2 验证方法

**对照组设计**：
- **A 组（无 L5）**：仅使用 L0-L4 四层记忆 + 基础蜂群
- **B 组（有 L5）**：L0-L5 五层记忆 + L5 元认知（策略日志 + 错误模式 + 自评）
- 两组使用相同 LLM Provider（GPT-4o）、相同任务集、相同 Prompt 模板

**任务集**（50 轮对话，每轮一个独立任务）：
1. 代码生成任务 ×10
2. 文档总结任务 ×10
3. 多步推理任务 ×10
4. 长上下文记忆任务 ×10
5. 错误恢复任务（故意制造失败）×10

### 2.3 验收标准

| 指标 | A 组（无 L5） | B 组（有 L5） | 验收门槛 |
|------|:------------:|:------------:|---------|
| 任务完成率 | 基线 | 期望提升 | B 组 ≥ A 组 + 10% |
| 平均 token 成本 | 基线 | 期望下降 | B 组 ≤ A 组 × 85% |
| 错误恢复成功率 | 基线 | 期望提升 | B 组 ≥ A 组 + 15% |
| 用户满意度（5 分制） | 基线 | 期望提升 | B 组 ≥ A 组 + 0.5 |

### 2.4 实测数据

> **测试范围说明**：v0.2.0 仅完成 L5 元认知引擎的单元测试（`tests/poc1_l5.rs` 4/4 PASS），spec 中的 50 轮对话对照实验（A 组 vs B 组）超出本 spec 范围，待 v0.2.1+ 接入 LLM Provider 后补完整对照实验。

| 指标 | A 组（无 L5） | B 组（有 L5） | 差值 | 是否达标 |
|------|:------------:|:------------:|:----:|:--------:|
| 任务完成率 | 未实测（无 LLM） | 未实测（无 LLM） | — | ⏳ 待 v0.2.1 |
| 平均 token 成本 | 未实测 | 未实测 | — | ⏳ 待 v0.2.1 |
| 错误恢复成功率 | 未实测 | 未实测 | — | ⏳ 待 v0.2.1 |
| 用户满意度 | 未实测 | 未实测 | — | ⏳ 待 v0.2.1 |

**单元测试层面实测**（`cargo test -p sparkfox-memory`）：
- `l5_records_strategy_log`：✅ PASS（策略日志正确记录，可按 task_id 检索）
- `l5_detects_error_pattern`：✅ PASS（连续 3 次相同错误被聚合，count >= 3）
- `l5_self_eval_recommendation_improves_score`：✅ PASS（B 组 0.85 > A 组 0.65 + 0.10 门槛，推荐策略 "CoT 推理" 正确）
- `memory_entry_has_layer_trait`：✅ PASS（MemoryEntry 实现 MemoryLayer，LAYER=2）

### 2.5 结论与决策

- **GO**：L5 价值验证通过，保留 L5 进入 Phase 1
- **NO-GO**：L5 价值不显著，砍 L5，重评"具备元认知"定位 + MVP 范围
- **条件性 GO**：L5 在特定场景有价值，保留但降级为可选 feature flag

**实测结论**: 单元测试 4/4 PASS，L5 元认知引擎的「策略日志/错误模式/自评推荐」核心机制工作正常，但完整 50 轮对照实验未实施
**决策**: ⚠️ 条件性 GO
**理由**: L5 引擎的内部机制（策略记录、错误聚合、推荐算法）在单元测试层面验证通过，可进入 Phase 1；但「对任务完成率/token 成本/错误恢复/满意度的实际提升」需 v0.2.1+ 接入 LLM 后补充 50 轮对照实验。若 v0.2.1 对照实验显示提升不显著，则降级为 feature flag。

---

## 三、PoC-2: automerge-rs CRDT 可行性

### 3.1 验证目标

验证 automerge-rs 在 SparkFox 场景下的 CRDT 同步性能与正确性。

### 3.2 验证方法

**测试场景**：
1. 2 设备同步 1000 条记忆条目
2. 离线 1 小时后重新同步（冲突测试）
3. 3 设备并发编辑同一条目
4. 10000 条记忆的全量同步

**测试维度**：
- 正确性：0 冲突丢失
- 性能：同步延迟、CPU 占用、内存占用
- 稳定性：长时间运行（24 小时）无崩溃

### 3.3 验收标准

| 指标 | 验收门槛 |
|------|---------|
| 冲突丢失数 | 0 |
| 1000 条同步延迟 | < 2s |
| 10000 条同步延迟 | < 30s |
| CPU 占用（同步时） | < 5% |
| 内存占用（10000 条） | < 100MB |
| 24 小时稳定性 | 无崩溃 |

### 3.4 实测数据

> **测试范围说明**：v0.2.0 完成 `tests/poc2_sync.rs` 3/3 PASS。**实现偏差**：spec 中使用 automerge sync protocol，但单轮 sync 无法传输数据，实际改用 `AutoCommit::save() + load() + merge()` 全量快照 + CRDT merge 方式（spec 偏差，详见 commit 92bb94d）。10000 条/24 小时稳定性未在本 spec 测试范围。

| 指标 | 实测值 | 验收门槛 | 是否达标 |
|------|:------:|---------|:--------:|
| 冲突丢失数 | 0（CRDT merge 保证） | 0 | ✅ |
| 1000 条同步延迟 | < 2s（断言通过，具体数字未打印） | < 2s | ✅ |
| 10000 条同步延迟 | 未测（spec 测试仅 1000 条） | < 30s | ⏳ 待 v0.2.1 |
| CPU 占用 | 未测 | < 5% | ⏳ 待 v0.2.1 |
| 内存占用 | 未测 | < 100MB | ⏳ 待 v0.2.1 |
| 24 小时稳定性 | 未测 | 无崩溃 | ⏳ 待 v0.2.1 |

**单元测试层面实测**（`cargo test -p sparkfox-crdt`）：
- `poc2_1000_entries_sync_under_2s`：✅ PASS（1000 条同步 < 2s，doc_b.entry_count() == 1000）
- `poc2_offline_then_sync_no_conflict_loss`：✅ PASS（双向同步后 doc_a.get_entry("k") == doc_b.get_entry("k")，LWW 一致）
- `poc2_3way_concurrent_no_data_loss`：✅ PASS（三方全互连，100 条 entries 三方条目数一致）

### 3.5 结论与决策

- **GO**：automerge-rs 满足要求，作为 CRDT 实现进入 Phase 1
- **NO-GO**：automerge-rs 不达标，CRDT 推迟到 v0.5+，先做单机版
- **条件性 GO**：性能边界需优化，限制同步规模

**实测结论**: 单元测试 3/3 PASS，automerge-rs 0.10 在 1000 条同步、离线双向同步、三方并发场景下均达成验收门槛（无冲突丢失、< 2s、最终一致）
**决策**: ✅ GO
**理由**: automerge-rs 0.10 集成成功，核心 CRDT 语义（LWW、最终一致、无冲突丢失）在测试中验证通过。10000 条/24 小时稳定性/CPU/内存指标待 v0.2.1 用 profiling 工具补充。**注意**：实际实现采用 save+load+merge 全量同步，未来若需增量同步（带宽优化）需重新评估 sync protocol 多轮交互方案。

---

## 四、PoC-3: bge Rust 推理性 + Python 一致性

### 4.1 验证目标

验证 bge-small-zh-v1.5 模型在 Rust（candle-transformers 0.8.4）推理下的性能、内存占用，以及与 Python sentence-transformers 的向量一致性。

### 4.2 验证方法

**测试场景**：
1. 单条文本 embedding 延迟（预热后）
2. 批量 1000 条文本 embedding 吞吐
3. 与 Python sentence-transformers 5.6.0 一致性对比（cosine similarity > 0.99，严格门槛）
4. L2 归一化验证（每条嵌入 norm ≈ 1.0）

**对比基线**：Python 3.11.15 (Astral) + sentence-transformers 5.6.0 + bge-small-zh-v1.5
**Rust 实现**：candle-transformers 0.8.4 + candle-core 0.8.4 + candle-nn 0.8.4 + tokenizers 0.21.4 + hf-hub 0.4.3
**测试 profile**：`--release`（opt-level=3, lto=thin, codegen-units=1, strip=true）
**模型加载**：本地预下载 `.models/BAAI_bge-small-zh-v1.5/`（通过 `SPARKFOX_MODELS_DIR` 环境变量指定）
**baseline 样本**：101 条中文文本（50 条"测试文本 N" + 51 条技术/文学场景），512 维，1.04 MB JSON

### 4.3 验收标准（用户决策 B：严格门槛）

| 指标 | 验收门槛 |
|------|---------|
| 单条 embedding 延迟 | < 50ms |
| 1000 条批量 embedding | < 30s |
| 与 Python 一致性（每条） | cosine > 0.99 |
| L2 归一化 | norm ≈ 1.0（误差 < 1e-4） |

### 4.4 实测数据

> **测试范围说明**：v1.0.0 模块一 Task 1.4 完成 PoC-3 全部 4 项验收测试（commit 80757f8）。10 万向量检索延迟 + 内存占用留待 v0.3 知识库 RAG 引擎集成时补充（依赖 sqlite-vec 二进制就位）。

| 指标 | Rust 实测 | Python 基线 | 验收门槛 | 是否达标 |
|------|:---------:|:-----------:|---------|:--------:|
| 单条 embedding 延迟 | 11ms | — | < 50ms | ✅ |
| 1000 条批量 embedding | 7.78s | — | < 30s | ✅ |
| 与 Python 一致性（min cosine） | 0.999999 | 1.0 | > 0.99 | ✅ |
| 与 Python 一致性（max cosine） | 1.000001 | 1.0 | > 0.99 | ✅ |
| L2 归一化（4 样本） | norm ≈ 1.0 | — | 误差 < 1e-4 | ✅ |
| 10 万向量检索 | 待 v0.3 | — | < 500ms | ⏳ 待 sqlite-vec |
| 内存占用 | 待 v0.3 | — | < 300MB | ⏳ 待 profiling |

**测试命令**（release profile）：
```bash
$env:SPARKFOX_MODELS_DIR="d:\xin kaifa\SparkFox\.models"
cargo test -p sparkfox-embedding --release --test poc3_bge -- --ignored --nocapture
```

**测试输出摘要**：
```
✅ PoC-3.1 单条嵌入: 11ms
✅ PoC-3.2 1000 条批量: 7.78s
✅ PoC-3.3 一致性: 样本 100, min cosine = 0.999999, max cosine = 1.000001
✅ PoC-3.4 归一化: 所有样本 L2 norm ≈ 1.0
```

### 4.5 结论与决策

- **GO**：Rust 推理性满足要求，进入 Phase 1
- **NO-GO**：Rust 推理性不达标，退回 Python sidecar（仅 embedding 模块）
- **条件性 GO**：性能边界需优化，限制向量规模

**实测结论**: v1.0.0 模块一 PoC-3 全部 4 项验收测试通过（release profile）
- 单条 11ms（远低于 50ms 门槛，2.3 倍余量）
- 1000 条批量 7.78s（远低于 30s 门槛，3.9 倍余量）
- Python 一致性 min cosine 0.999999（远超 0.99 门槛，10^−6 级精度，浮点误差范围内）
- L2 归一化全部通过

**决策**: ✅ GO
**理由**: candle-transformers 0.8.4 + bge-small-zh-v1.5 在 release profile 下性能远超门槛，与 Python sentence-transformers 5.6.0 的向量一致性达 10^−6 级精度（min cosine 0.999999）。Kill Switch（退回 Python sidecar）不触发，sparkfox-embedding 作为 v1.0.0 嵌入模块进入 Phase 1。10 万向量检索 + 内存占用指标留待 v0.3 知识库 RAG 引擎集成时补充（依赖 sqlite-vec 二进制）。

**关键实现偏差**（已记录）：
1. spec 写 candle 0.7，实际升级到 0.8.4（candle-core 0.7.2 的 rand_distr 版本冲突）
2. spec 写 tokenizers 0.20 / hf-hub 0.3，实际升级到 0.21.4 / 0.4.3（同步升级）
3. spec 写 `forbid(unsafe_code)`，实际改为 `deny(unsafe_code)` + 局部 `allow`（candle-nn 的 mmap 是 unsafe）
4. spec 用 `ApiBuilder::new()`，实际改用 `from_env()` 才能读 HF_ENDPOINT；最终因 hf-mirror.com 与 hf-hub 0.4 不兼容，改用 PowerShell 预下载 + `find_local_model_dir` 本地查找
5. spec 写 bge-large-zh，实际用 bge-small-zh（512 维，120MB，桌面端默认；large 留作可选）

---

## 五、PoC-4: NomiFun + sqlite-vec 性能基线

### 5.1 验证目标

建立 SparkFox 基座（NomiFun 0.2.28 + sqlite-vec）的性能基线，验证关键性能指标可达成。

### 5.2 验证方法

**测试环境**：
- OS: Windows 11 / macOS 14 / Ubuntu 22.04
- CPU: TBD
- RAM: TBD
- Disk: TBD

**测试场景**：
1. 冷启动时间（首次启动）
2. 热启动时间（已缓存）
3. 10 万向量 RAG 检索延迟
4. 100 万向量 RAG 检索延迟（压力测试）
5. 安装包体积
6. 内存占用（空闲 / 满载）
7. 50 轮对话内存增长

### 5.3 验收标准（校准后）

| 指标 | 原方案目标 | 校准后目标 | 理由 |
|------|:---------:|:---------:|------|
| 冷启动 | < 2s | < 3s | NomiFun 实际 ~2.5s，加 Pangu crate 后放宽 |
| 热启动 | < 1s | < 1.5s | 同上 |
| RAG 10 万向量 | < 500ms | < 800ms | sqlite-vec 实际 ~600-800ms |
| RAG 100 万向量 | < 2s | < 5s | 百万向量压力测试 |
| 安装包体积 | < 150MB | < 200MB | NomiFun ~140MB + Pangu crate + bge 模型 |
| 内存占用（空闲） | - | < 300MB | - |
| 内存占用（满载） | - | < 800MB | - |
| 50 轮对话内存增长 | - | < 100MB | 检测内存泄漏 |

### 5.4 实测数据

> **测试范围说明**：v0.2.0 完成 `tests/poc4_perf.rs` 2/3 PASS。**关键阻塞**：sqlite-vec 二进制未就位（用户尚未下载放置），导致 `poc4_100k_vector_search_under_800ms` 无法执行（vector_insert 在 sqlite-vec 未加载时返回 Error）。其余性能指标（热启动/100 万向量/安装包/内存）超出本 spec 测试范围，待 v0.2.1+ 补充。

| 指标 | Windows | macOS | Linux | 验收门槛 | 是否达标 |
|------|:-------:|:-----:|:-----:|---------|:--------:|
| 冷启动 | 0.02s | 未测 | 未测 | < 3s | ✅ |
| 热启动 | 未测 | 未测 | 未测 | < 1.5s | ⏳ 待 v0.2.1 |
| RAG 10 万 | ❌ 未通过 | 未测 | 未测 | < 800ms | ⚠️ sqlite-vec 二进制未就位 |
| RAG 100 万 | 未测 | 未测 | 未测 | < 5s | ⏳ 待 v0.2.1 |
| 安装包 | 未测 | 未测 | 未测 | < 200MB | ⏳ 待 v0.2.1 |
| 内存空闲 | 未测 | 未测 | 未测 | < 300MB | ⏳ 待 v0.2.1 |
| 内存满载 | 未测 | 未测 | 未测 | < 800MB | ⏳ 待 v0.2.1 |
| 50 轮增长 | 未测 | 未测 | 未测 | < 100MB | ⏳ 待 v0.2.1 |

**单元测试层面实测**（`cargo test -p sparkfox-store`）：
- `poc4_cold_start_under_3s`：✅ PASS（冷启动 0.02s，远低于 3s 门槛）
- `poc4_schema_migrate_idempotent`：✅ PASS（6 层记忆 schema 迁移幂等性验证通过）
- `poc4_100k_vector_search_under_800ms`：❌ FAIL（`sqlite-vec 未加载`，二进制未就位）

**sqlite-vec 二进制放置路径**（按 `vec.rs` 中 `dirs_next::data_dir()` 实现）：
- 环境变量：`SPARKFOX_SQLITE_VEC_PATH` 指向 `sqlite_vec.dll` 完整路径
- exe 同目录：`<exe_dir>/sqlite-vec/sqlite_vec.dll`
- 用户数据目录（Windows）：`%APPDATA%\sparkfox\sqlite-vec\sqlite_vec.dll`

### 5.5 结论与决策

- **GO**：性能基线达标，进入 Phase 0
- **NO-GO**：性能基线不达标，重设性能目标 + 砍 30% 功能
- **条件性 GO**：部分指标需优化，限制使用场景

**实测结论**: 单元测试 2/3 PASS，冷启动 0.02s 远低于门槛，schema 迁移幂等性通过；10 万向量检索因 sqlite-vec 二进制未就位无法执行
**决策**: ⚠️ 条件性 GO
**理由**: SQLite 基座 + 6 层记忆 schema 工作正常，冷启动性能远超门槛。向量检索功能需用户放置 sqlite-vec 二进制后重测 `poc4_100k_vector_search_under_800ms`，若重测通过则升级为 GO。**缓解措施**：v0.2.1 在 `sparkfox-store/README.md` 中补充 sqlite-vec 二进制下载与放置指南，并考虑在 Store::open 时若 vec 加载失败则给出更详细的诊断信息（含下载链接）。

---

## 六、总体结论与下一步

### 6.1 PoC 总体结论

| PoC | 决策 | 影响 |
|-----|------|------|
| PoC-1 L5 | ⚠️ 条件性 GO（单元层通过，对照实验待 v0.2.1） | 保留 L5 进入 Phase 1，对照实验后定升降级 |
| PoC-2 CRDT | ✅ GO | automerge-rs 作为 CRDT 实现进入 Phase 1 |
| PoC-3 bge | ✅ GO（v1.0.0 实测 4/4 通过） | sparkfox-embedding 作为嵌入模块进入 v1.0.0，Kill Switch 不触发 |
| PoC-4 性能 | ⚠️ 条件性 GO（冷启动+schema 通过，向量检索待二进制就位） | 性能目标暂维持，向量检索重测后定升降级 |

### 6.2 进入 Phase 0 的条件

- [x] 4 项 PoC 全部 GO 或条件性 GO（PoC-1/4 条件性 GO，PoC-2/3 GO）— **已满足**
- [x] 条件性 GO 项的缓解措施已制定（见 6.3）
- [x] 任何 NO-GO 项的 Kill Switch 已执行（无 NO-GO 项）
- [ ] 7 专家评审通过 PoC 报告

**进入 Phase 0 的实际状态**：**基本满足**（仅剩 7 专家评审）。v1.0.0 模块一 PoC-3 实测通过后，4 项 PoC 全部 GO 或条件性 GO。可正式启动 v1.0.0 后续模块（模块二 embedding 完整实现 / 模块三知识库 RAG / 模块四 parser 等）。

### 6.3 风险遗留

| 风险项 | 严重度 | 缓解措施 | 责任人 |
|--------|:------:|---------|:------:|
| PoC-1 L5 完整对照实验未实施 | 🟡 中 | v0.2.1 接入 LLM 后补 50 轮对照实验，若提升不显著则降级为 feature flag | TBD |
| PoC-4 sqlite-vec 二进制未就位 | 🟡 中 | 用户放置 sqlite_vec.dll 后重测，README 补充下载指南 | TBD |
| PoC-2 实际用 save+load+merge 而非 sync protocol | 🟡 中 | 全量同步在小数据量下可用，大数据量需评估增量 sync protocol 方案 | TBD |
| PoC-3 10万向量检索 + 内存占用未测 | 🟡 中 | v0.3 知识库 RAG 引擎集成时补充（依赖 sqlite-vec 二进制） | TBD |
| PoC-3 实现偏差：candle 0.7→0.8.4 / tokenizers 0.20→0.21.4 / hf-hub 0.3→0.4.3 | 🟢 低 | 已记录在 PoC-3 章节 4.5，spec 同步更新 | TBD |
| ratchetx2 0.3 API 与 spec 不兼容，已改自实现 | 🟢 低 | 自实现 Double Ratchet 已通过 2/2 测试，需在 v0.3+ 补完整安全审计 | TBD |
| sparkfox-core 中 log 未在 workspace 声明 | 🟢 低 | 各 crate 直接用 `log = "0.4"`，可后续统一加入 workspace.dependencies | TBD |

---

## 附录：PoC 执行日志

| 日期 | PoC | 事件 | 负责人 |
|------|------|------|:------:|
| 2026-07-19 | PoC-1/2/4 | v0.2.0 spec A 完成，单元测试层面 PoC 验收 | SparkFox v0.2.0 |
| 2026-07-19 | PoC-1 | sparkfox-memory L5MetaEngine 落地，4/4 测试 PASS（commit 810f684） | SparkFox v0.2.0 |
| 2026-07-19 | PoC-2 | sparkfox-crdt automerge-rs 集成，3/3 测试 PASS（commit 92bb94d） | SparkFox v0.2.0 |
| 2026-07-19 | PoC-4 | sparkfox-store SQLite+sqlite-vec 加载，2/3 测试 PASS（commit 6c4aa4f） | SparkFox v0.2.0 |
| 2026-07-19 | PoC-3 | sparkfox-embedding crate 骨架（commit c587b95） | SparkFox v1.0.0 |
| 2026-07-19 | PoC-3 | 模型下载器 + SHA256 校验（commit 50fcfa3） | SparkFox v1.0.0 |
| 2026-07-19 | PoC-3 | BgeEmbedder 推理器 + mean pool + L2 normalize（commit 5cf4024） | SparkFox v1.0.0 |
| 2026-07-19 | PoC-3 | PoC-3 全部 4 项验收测试 PASS：单条 11ms / 批量 7.78s / cosine 0.999999 / L2 norm 1.0（commit 80757f8） | SparkFox v1.0.0 |
| 2026-07-19 | PoC-3 | 决策：✅ GO，Kill Switch 不触发，进入 v1.0.0 后续模块 | SparkFox v1.0.0 |
| TBD | PoC-1 | 50 轮对照实验（A 组 vs B 组）待 v0.2.1 接入 LLM 后实施 | TBD |
| TBD | PoC-3 | 10 万向量检索 + 内存占用待 v0.3 sqlite-vec 就位后补充 | TBD |
| TBD | PoC-4 | sqlite-vec 二进制就位后重测 10 万向量检索 | TBD |

---

**报告完成。**

> 本报告作为 Phase -1 PoC 验收的归档文档，GO/NO-GO 决策必须经 7 专家评审签字。
> 当前状态：v0.2.0 单元测试层面 + v1.0.0 模块一 PoC-3 实测验收完成，4 项 PoC 全部 GO 或条件性 GO，7 专家评审待启动。
