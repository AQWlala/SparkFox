# RFC-004: CRDT 选型

> **RFC 编号**: RFC-004
> **主题**: 跨设备同步的 CRDT 实现选型
> **状态**: 📝 草案（待评审）
> **创建日期**: 2026-07-18
> **评审专家**: 架构 / 安全 / 性能
> **影响 Phase**: Phase 1（必须前置决策）

---

## 一、背景与问题

### 1.1 原方案问题（架构 + 安全 + 风险 3 专家点名）

原方案采用 Pangu 自研 LWW-Register + OR-Set CRDT，但：

1. **Rust 生态无成熟实现**：自研 LWW+OR-Set 需 4-8 周开发 + 测试
2. **自研 CRDT + E2EE 组合无团队经验**：风险极高
3. **跨设备配对缺安全引导**：安全专家警告
4. **与 automerge-rs 对比无优势**：automerge 已验证、有社区、支持 JSON CRDT

### 1.2 用户决策（2026-07-18）

用户已确认：**改用 automerge-rs**（放弃自研）

---

## 二、设计目标

1. **采用 automerge-rs**：已验证、社区成熟、支持 JSON CRDT
2. **与 E2EE 集成**：CRDT 同步数据需经 E2EE 加密
3. **跨设备配对安全引导**：解决安全专家警告
4. **性能可接受**：10 万记忆同步 < 30s

---

## 三、automerge-rs 集成方案

### 3.1 automerge-rs 简介

- **GitHub**: https://github.com/automerge/automerge-rs
- **协议**: MIT，商用友好
- **特性**: JSON CRDT、支持嵌套结构、增量同步、二进制编码
- **成熟度**: 已在生产环境使用（Ink & Switch 推荐）

### 3.2 与 SparkFox 6 层记忆的映射

```rust
// automerge Document 作为同步根
let mut doc = AutoCommit::new();

// L0 工作记忆 → automerge list
let l0 = doc.set_object(ObjectId::Root, "l0", ObjType::List)?;

// L1 情景记忆 → automerge map
let l1 = doc.set_object(ObjectId::Root, "l1", ObjType::Map)?;

// L3 语义记忆 → automerge table（带向量索引）
let l3 = doc.set_object(ObjectId::Root, "l3", ObjType::Table)?;

// L5 元认知 → automerge map（跨层元数据）
let l5 = doc.set_object(ObjectId::Root, "l5", ObjType::Map)?;
```

### 3.3 同步流程

```
设备 A 编辑记忆
  ↓
automerge AutoCommit 生成变更
  ↓
E2EE 加密（X25519 + AES-256-GCM + Double Ratchet）
  ↓
WebSocket / HTTP 同步到设备 B
  ↓
设备 B 解密 + 应用变更
  ↓
automerge 自动合并（无冲突）
  ↓
触发 memory.update 事件，刷新 UI
```

---

## 四、E2EE 加密增强（安全专家要求）

### 4.1 原方案问题

原方案 E2EE 用 X25519 + HKDF + AES-256-GCM，但**缺 Double Ratchet**（无前向保密）。

### 4.2 优化后 E2EE

- **X25519**：密钥交换
- **HKDF**：密钥派生
- **AES-256-GCM**：对称加密
- **Double Ratchet**（新增）：前向保密 + 后向保密
- **参考实现**: Signal Protocol / libsignal

### 4.3 跨设备配对安全引导（安全专家要求）

```
设备 A → 显示 QR Code（含公钥 + 一次性配对令牌）
  ↓
设备 B → 扫码 → 建立 X25519 会话
  ↓
双向验证（短密钥比对，类似 Bluetooth Pairing）
  ↓
交换长期公钥 → 完成 Double Ratchet 初始化
  ↓
后续同步使用 Double Ratchet 自动轮换
```

---

## 五、性能预期

| 场景 | 目标 | 验证方式 |
|------|------|---------|
| 2 设备 1000 条同步 | < 2s | PoC-2 实测 |
| 2 设备 10000 条同步 | < 30s | PoC-2 实测 |
| 离线 1 小时后同步 | 0 冲突丢失 | PoC-2 实测 |
| 3 设备并发编辑 | 0 冲突丢失 | PoC-2 实测 |
| 24 小时稳定性 | 无崩溃 | PoC-2 实测 |

---

## 六、降级策略

### 6.1 PoC-2 失败时

如果 PoC-2 验证 automerge-rs 不达标：
- **降级方案**：CRDT 推迟到 v0.5+，先做单机版
- **影响**：失去跨设备同步卖点，但保留其他护城河

### 6.2 E2EE 失败时

如果 Double Ratchet 实现复杂度过高：
- **降级方案**：v0.1 仅用 X25519 + AES-256-GCM（无前向保密）
- **影响**：安全性降低，但 v0.3+ 补 Double Ratchet

---

## 七、待评审决策点

- [ ] automerge-rs 版本锁定？（最新稳定版）
- [ ] Double Ratchet 是否 v0.1 必做？（还是 v0.3+ 补）
- [ ] 跨设备配对 QR Code 协议细节？
- [ ] 同步传输层：WebSocket / HTTP / 两者都支持？
- [ ] 离线冲突解决 UI：自动合并 / 用户手动选择 / 两者结合？

---

## 八、评审记录

| 评审专家 | 评分 | 意见 | 日期 |
|---------|:----:|------|------|
| 架构专家 | TBD | TBD | TBD |
| 安全专家 | TBD | TBD | TBD |
| 性能专家 | TBD | TBD | TBD |

---

**RFC 状态**：📝 草案 → 待 3 专家评审 → 修订 → 定稿 → 进入 Phase 1
