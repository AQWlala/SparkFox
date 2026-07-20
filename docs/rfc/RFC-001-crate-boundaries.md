# RFC-001: crate 边界重划

> **RFC 编号**: RFC-001
> **主题**: SparkFox Rust workspace 的 crate 边界划分
> **状态**: 📝 草案（待评审）
> **创建日期**: 2026-07-18
> **评审专家**: 架构 / 记忆系统 / 工程化
> **影响 Phase**: Phase 0（必须前置决策）

---

## 一、背景与问题

### 1.1 原方案问题（架构专家评审）

原《四项目融合重组方案》Phase 1 列出 14 个 crate，但未明确：

1. **数据层独占性问题**：memory crate 是否独占 sqlx 连接池？sponge-engine / blackhole-engine / loop-engine 是否直接访问数据库？
2. **依赖方向问题**：sponge-engine 依赖 memory 还是 memory 依赖 sponge-engine？双向依赖如何避免？
3. **crate 粒度问题**：security 11 项是 1 个 crate 还是 11 个 crate？dag 与 swarm 是否合并？
4. **NomiFun 既有 crate 边界冲突**：NomiFun 已有 nomifun-db / nomifun-knowledge / nomifun-companion（含 memory.db），新增 Pangu memory crate 如何与之共存？

### 1.2 风险

- crate 边界不清 → 14 并行 agent 强耦合无法真并行（架构专家 P0 风险）
- 数据层多 crate 直连 → sqlx 锁竞争（性能专家 P0 风险）
- 双向依赖 → 编译失败 + 测试隔离困难

---

## 二、设计目标

1. **单一职责**：每个 crate 只做一件事
2. **依赖单向**：依赖方向清晰，无循环依赖
3. **数据层独占**：sqlx 连接池由单一 crate 管理
4. **可并行开发**：crate 之间接口契约明确，可独立开发测试
5. **与 NomiFun 既有结构兼容**：不破坏 NomiFun 50+ crate workspace

---

## 三、候选方案

### 方案 A: memory 独占数据层（推荐）

```
nomifun-db (NomiFun 既有)
  ├─ SqlitePool 独占管理
  ├─ ~20 对 repository trait + 实现
  └─ schema 迁移

sparkfox-memory (新增)
  ├─ 依赖 nomifun-db（通过 trait）
  ├─ 6 层记忆领域模型（L0-L5）
  ├─ 不直接访问 SqlitePool
  └─ 通过 MemoryRepository trait 读写

sparkfox-sponge (新增)
  ├─ 依赖 sparkfox-memory（通过 trait）
  ├─ 海绵吸收引擎
  └─ 调用 memory.write() 写入事实

sparkfox-blackhole (新增)
  ├─ 依赖 sparkfox-memory（通过 trait）
  ├─ 黑洞压缩引擎
  └─ 调用 memory.compress() 压缩下层
```

**优点**：数据层独占，无锁竞争；依赖单向清晰
**缺点**：所有数据访问必须经过 trait 抽象，开发成本略高

### 方案 B: 各 crate 共享 SqlitePool

```
nomifun-db
  └─ 暴露 SqlitePool 给所有 crate

sparkfox-memory / sponge / blackhole
  └─ 直接拿 SqlitePool 操作
```

**优点**：开发简单
**缺点**：锁竞争（性能专家 P0 风险），无隔离

### 方案 C: 按 6 层记忆分 crate

```
sparkfox-memory-l0 (工作记忆)
sparkfox-memory-l1 (情景)
sparkfox-memory-l2 (叙事)
sparkfox-memory-l3 (语义)
sparkfox-memory-l4 (程序)
sparkfox-memory-l5 (元认知)
```

**优点**：每层独立可并行
**缺点**：crate 数量爆炸（14 → 20+），跨层查询困难

---

## 四、推荐决策

**推荐方案 A**：memory 独占数据层

理由：
1. 性能专家明确警告 sqlx 锁竞争 P0 风险
2. 架构专家要求 crate 边界前置决策
3. trait 抽象符合 DDD 原则，长期可维护性高

---

## 五、crate 边界最终划分（待评审确认）

| crate | 职责 | 依赖 | 数据访问方式 |
|-------|------|------|-------------|
| nomifun-db | SqlitePool + 迁移 + 基础 repository | 无 | 直接 |
| sparkfox-memory | 6 层记忆领域模型 | nomifun-db (trait) | 通过 trait |
| sparkfox-sponge | 海绵吸收 | sparkfox-memory (trait) | 通过 memory |
| sparkfox-blackhole | 黑洞压缩 | sparkfox-memory (trait) | 通过 memory |
| sparkfox-loop | 4 阶段反思 | sparkfox-memory + sponge + blackhole | 通过 memory |
| sparkfox-crdt | CRDT 同步（automerge-rs） | sparkfox-memory (trait) | 通过 memory |
| sparkfox-crypto | E2EE 加密 | 无 | 无（纯计算） |
| sparkfox-security | 11 项安全栈 | 无 | 无（纯计算 + 装饰器） |
| sparkfox-budget | 三维预算 | nomifun-db (trait) | 通过 trait |
| sparkfox-cu-safety | CU 安全四件套 | 无 | 无 |
| sparkfox-evolution | 4 阶段进化 | sparkfox-memory + loop | 通过 memory |
| sparkfox-swarm | 蜂群编排 | sparkfox-budget (trait) | 通过 budget |
| sparkfox-distiller | Skill 蒸馏 | sparkfox-memory (trait) | 通过 memory |
| sparkfox-dag | DAG 编排 | sparkfox-swarm (trait) | 通过 swarm |
| sparkfox-mcp | MCP Client/Server | 无 | 无 |

**依赖方向图**：
```
nomifun-db
  ↑
sparkfox-memory ←──── sparkfox-sponge
  ↑                     ↑
sparkfox-blackhole      │
  ↑                     │
sparkfox-loop ──────────┘
  ↑
sparkfox-evolution
  ↑
sparkfox-distiller

sparkfox-crdt → sparkfox-memory
sparkfox-swarm → sparkfox-budget → nomifun-db
sparkfox-dag → sparkfox-swarm

独立 crate（无依赖）:
  sparkfox-crypto / sparkfox-security / sparkfox-cu-safety / sparkfox-mcp
```

---

## 六、待评审决策点

- [ ] 方案 A / B / C 选哪个？
- [ ] security 11 项是 1 个 crate 还是拆为多个？
- [ ] swarm 与 dag 是否合并为 sparkfox-orchestration？
- [ ] sponge 与 blackhole 是否合并为 sparkfox-engine？
- [ ] 与 NomiFun 既有 nomifun-companion（含 memory.db）如何整合？

---

## 七、评审记录

| 评审专家 | 评分 | 意见 | 日期 |
|---------|:----:|------|------|
| 架构专家 | TBD | TBD | TBD |
| 记忆系统专家 | TBD | TBD | TBD |
| 工程化专家 | TBD | TBD | TBD |

---

**RFC 状态**：📝 草案 → 待 3 专家评审 → 修订 → 定稿 → 进入 Phase 0
