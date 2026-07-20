# RFC-005: 并行度与 target 隔离

> **RFC 编号**: RFC-005
> **主题**: SparkFox 开发的并行 agent 数量与 target 隔离方案
> **状态**: 📝 草案（待评审）
> **创建日期**: 2026-07-18
> **评审专家**: 架构 / 工程化 / 风险评估
> **影响 Phase**: Phase 0-6（影响所有 Phase）

---

## 一、背景与问题

### 1.1 原方案问题（架构 + 工程 + 风险 3 专家点名）

原方案允许 14 并行 agent，但：

1. **14 并行 ≠ 14 倍生产力**：14 个 crate 共享 sqlx 数据层 + memory crate，强耦合无法真并行
2. **实际可并行度**：架构专家建议 6-8，工程化专家建议 3-4
3. **"每阶段单次 Git commit"与 14 并行冲突**：14 个 agent 同时修改无法单 commit
4. **无 target 隔离**：14 agent 同时操作同一仓库导致锁竞争

### 1.2 用户决策（2026-07-18）

用户已确认：**3-4 并行**（最稳但最慢，接受 18-30 个月工期）

---

## 二、设计目标

1. **3-4 并行**：用户确认的并行度
2. **target 隔离**：每个 agent 独立 crate + 独立测试
3. **单 commit 兼容**：3-4 并行可通过 git stage 合并为单 commit
4. **锁竞争避免**：3-4 agent 不操作同一文件

---

## 三、target 隔离方案

### 3.1 隔离原则

- 每个 agent 分配**独立 crate 目录**（如 `crates/sparkfox-memory/`）
- agent 只能修改自己分配的 crate 目录
- 跨 crate 依赖通过 **trait 契约**定义（在 Phase 0 提前定义）
- agent 不得修改 `Cargo.toml` workspace 根（由 Phase 0 统一管理）

### 3.2 target 分配示例（Phase 1，3-4 并行）

**批次 1（3 并行，5-7 周）**：

| Agent | crate | 目录 | 依赖 |
|-------|-------|------|------|
| Agent-1 | sparkfox-memory | `crates/sparkfox-memory/` | nomifun-db (trait) |
| Agent-2 | sparkfox-sponge | `crates/sparkfox-sponge/` | sparkfox-memory (trait) |
| Agent-3 | sparkfox-blackhole | `crates/sparkfox-blackhole/` | sparkfox-memory (trait) |

**批次 2（3 并行，5-7 周）**：

| Agent | crate | 目录 | 依赖 |
|-------|-------|------|------|
| Agent-1 | sparkfox-crdt | `crates/sparkfox-crdt/` | sparkfox-memory (trait) |
| Agent-2 | sparkfox-crypto | `crates/sparkfox-crypto/` | 无（独立） |
| Agent-3 | sparkfox-budget | `crates/sparkfox-budget/` | nomifun-db (trait) |

**批次 3（4 并行，5-7 周）**：

| Agent | crate | 目录 | 依赖 |
|-------|-------|------|------|
| Agent-1 | sparkfox-security | `crates/sparkfox-security/` | 无（独立） |
| Agent-2 | sparkfox-cu-safety | `crates/sparkfox-cu-safety/` | 无（独立） |
| Agent-3 | sparkfox-evolution | `crates/sparkfox-evolution/` | sparkfox-memory + loop (trait) |
| Agent-4 | sparkfox-loop | `crates/sparkfox-loop/` | sparkfox-memory + sponge + blackhole (trait) |

**批次 4（3 并行，5-9 周）**：

| Agent | crate | 目录 | 依赖 |
|-------|-------|------|------|
| Agent-1 | sparkfox-swarm | `crates/sparkfox-swarm/` | sparkfox-budget (trait) |
| Agent-2 | sparkfox-distiller | `crates/sparkfox-distiller/` | sparkfox-memory (trait) |
| Agent-3 | sparkfox-dag + sparkfox-mcp | `crates/sparkfox-dag/` + `crates/sparkfox-mcp/` | swarm (trait) / 独立 |

### 3.3 单 commit 合并流程

```
批次完成 → 各 agent 提交 PR 到 feature 分支
  ↓
Reviewer 审查（3-Agent 协作：Writer → Reviewer → Reviser）
  ↓
合并到主干（git merge --no-ff，保留批次历史）
  ↓
单次 git commit 标记批次完成（tag: phase1-batch1-done）
```

---

## 四、agent 工作流（3-Agent 协作）

### 4.1 用户 memory 决策

用户 memory 明确要求：
- **3-Agent 协作工作流**：Writer → Reviewer → Reviser
- **2-3 parallel task推进智能体**
- **2-3 parallel GitHub CLI 错误监控智能体**（每 2-5 分钟检查）

### 4.2 优化后工作流

```
Phase 1 批次执行:
  ├─ Writer Agent ×3-4（并行，按 target 分配）
  ├─ Reviewer Agent ×1（串行，审查 PR）
  ├─ Reviser Agent ×1（串行，修复审查问题）
  ├─ Monitor Agent ×2-3（并行，每 2-5 分钟检查 GitHub CLI 错误）
  └─ Orchestrator Agent ×1（串行，批次调度）
```

**总 agent 数**：8-10（Writer 3-4 + Reviewer 1 + Reviser 1 + Monitor 2-3 + Orchestrator 1）

---

## 五、锁竞争避免

### 5.1 文件级锁

- `Cargo.toml` workspace 根：**Phase 0 一次性配置**，Phase 1 不允许 agent 修改
- `crates/sparkfox-memory/Cargo.toml`：**Agent-1 独占**
- `ui/package.json`：**前端 agent 独占**（Phase 2-3）

### 5.2 数据库级锁

- sqlx 连接池由 nomifun-db 独占管理
- 各 crate 通过 trait 访问，不直接持有连接
- 测试时使用独立 SQLite 文件（每 agent 一个 `test-<crate>.db`）

### 5.3 Git 锁

- 每 agent 在独立 feature 分支工作（`feat/phase1-batch1-memory`）
- 合并时由 Orchestrator 串行合并，避免并发 push 冲突

---

## 六、时间线校准（3-4 并行）

| Phase | 批次数 | 每批 3-4 并行 | 单批周期 | 总周期 |
|-------|:------:|:------------:|:--------:|:------:|
| Phase 1 | 4 批 | 3-4 | 5-7 周 | 20-28 周 |
| Phase 2 | 3 批 | 3-4 | 3-4 周 | 9-12 周 |
| Phase 3 | 4 批 | 3-4 | 3-4 周 | 12-16 周 |
| Phase 4 | 2 批 | 3-4 | 3-4 周 | 6-8 周 |

**总工期**：75-130 周（约 18-30 个月），符合用户接受范围。

---

## 七、待评审决策点

- [ ] 3-4 并行是否接受？（用户已确认）
- [ ] target 隔离方案是否可行？
- [ ] 3-Agent 协作工作流（Writer → Reviewer → Reviser）是否纳入 RFC？
- [ ] Monitor Agent 每 2-5 分钟检查 GitHub CLI 错误，是否纳入 RFC？
- [ ] 单 commit 合并流程是否可行？还是改为多 commit？

---

## 八、评审记录

| 评审专家 | 评分 | 意见 | 日期 |
|---------|:----:|------|------|
| 架构专家 | TBD | TBD | TBD |
| 工程化专家 | TBD | TBD | TBD |
| 风险评估专家 | TBD | TBD | TBD |

---

**RFC 状态**：📝 草案 → 待 3 专家评审 → 修订 → 定稿 → 进入 Phase 0
