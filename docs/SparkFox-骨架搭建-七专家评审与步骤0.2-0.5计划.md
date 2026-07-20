# SparkFox 骨架搭建 - 七专家评审与步骤 0.2-0.5 实施计划

> 生成时间：2026-07-18
> 阶段：Phase 0 骨架搭建前置评审
> 基于文档：SparkFox-最终融合蓝图-1.0.md + RFC-001~005 + 四项目深度分析报告
> 当前项目状态：D:\xin kaifa\SparkFox 是 NomiFun 0.2.28 的完整 clone

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第一部分：七专家评审结论
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 🔵 架构专家

**评审结论**：现有项目（NomiFun 0.2.28 clone）技术栈 100% 契合蓝图，**强烈建议渐进改造而非重建**。

**关键建议**：
1. 不要大规模重命名 `nomi-*` / `nomifun-*` crate 为 `sparkfox-*`：46 个 crate 重命名会导致 git 历史断裂 + 46 个 Cargo.toml + 数千处 `use` 语句修改，2-3 周工作量且容易出错。建议**新建 `crates/sparkfox/` 子目录，按 RFC-001 新增 14 个 sparkfox-* crate**，旧 crate 作为依赖保留。
2. 不要修改 `apps/desktop/main.rs` 核心逻辑：NomiFun 的 Tauri 2 进程内后端 + macOS traffic light 定位 + companion 多窗口 + keepawake + 系统托盘是数千小时打磨成果，应作为 SparkFox 桌面壳直接复用。
3. 品牌替换分两层：顶层 `package.json` / `Cargo.toml` / `tauri.conf.json` 改 SparkFox；内部 crate 名保持 `nomi-*` / `nomifun-*` 不变（避免大规模改动）。
4. AGPL 替换 Apache-2.0 有法律风险：NomiFun 是 Apache-2.0 项目，我们 clone 后改 AGPL 在法律上可以（Apache-2.0 兼容 AGPL-3.0），但需要在 NOTICE 文件中明确声明"基于 NomiFun Apache-2.0 二次开发 + SparkFox 新增代码 AGPL-3.0"。

### 🟣 记忆系统专家

**评审结论**：当前 `nomi-memory` 仅 1 层 YAML frontmatter，与蓝图 6 层 L0-L5 差距极大，但**步骤 0.2-0.5 不应实现记忆系统**，仅创建骨架。

**关键建议**：
1. 步骤 0.2 只加依赖占位：根 `Cargo.toml` 加 `automerge` + `double-ratchet` + `sqlite-vec` 绑定；不加实际代码。
2. 步骤 0.3 创建 14 个 sparkfox-* crate 空壳：每个仅 `Cargo.toml` + `src/lib.rs`（`pub fn init() {}` 占位），不写实际逻辑。
3. 步骤 0.4 创建 8 个 Zustand store 空壳：`chatStore.ts` / `agentStore.ts` / `memoryStore.ts` / `monitorStore.ts` / `thinkingStore.ts` / `hotspotStore.ts` / `settingsStore.ts` / `sceneStore.ts`，每个仅导出空 state + 1 个示例 selector。
4. 不要在步骤 0.5 验证时启动记忆系统：6 层记忆实现属于 Phase -1 PoC，不是步骤 0.x。

### 🔴 安全专家

**评审结论**：当前 `apps/desktop/src/main.rs` 的 `webui_init_script` fetch/XHR 拦截器 + `x-nomi-local-trust` 本地信任机制是合理的安全基座，但**CSP=null 是高危项**。

**关键建议**：
1. 步骤 0.2 必须加 CSP：`tauri.conf.json` 的 `app.security.csp` 从 `null` 改为显式白名单。
2. AGPL 清洁室流程必须前置：在步骤 0.3 创建 crate 骨架时，每个 sparkfox-* crate 的 `Cargo.toml` 必须声明 `license = "AGPL-3.0-only"`，`NOTICE` 文件中明确"参考 NomiFun Apache-2.0 / BaiLongma MIT / OpenAkita 实现，清洁室重写"。
3. Double Ratchet 依赖选择要谨慎：crates.io 上有多个 `double-ratchet` 实现，建议用 `aws-nitro-enclaves-double-ratchet` 或自实现，避免用社区未审计版本。
4. automerge 依赖版本固定：`automerge = "0.5"` 当前稳定，但 API 仍在演进，建议锁定 minor 版本。
5. sqlite-vec 集成方式：不要用 `sqlite-vec` crate（不成熟），用 `rusqlite` 加载 `sqlite-vec.dll/.so/.dylib` 扩展方式（BaiLongma 已验证）。

### 🟠 性能专家

**评审结论**：NomiFun 的 release profile（opt-level=3 + lto=thin + codegen-units=1 + strip）+ dev profile（line-tables-only）已经是最佳实践，**无需调整**。

**关键建议**：
1. 步骤 0.2 不要升级 Rust 工具链：当前 `edition = "2024"` 已对齐蓝图，不要强升 nightly（stable Rust 1.85+ 已支持 edition 2024）。
2. 步骤 0.3 新增 crate 时 `[profile.dev]` 保持现状：dev 调试需要 line-tables-only，不要为了体积牺牲调试体验。
3. 步骤 0.4 Zustand store 拆分要细：8 个 store 不要合并，避免单 store 过大导致 selector 性能下降。
4. 步骤 0.5 验证时关注首屏加载：`bun run dev` 启动后首屏应 <2s，Tauri webview 内存应 <100MB（参考 NomiFun 基线 50-80MB）。
5. 依赖增量要小：步骤 0.2 新增依赖建议分批加（先 Rust 后 JS），避免一次性 cargo build 拉取数百个 crate 编译 10+ 分钟。

### 🟢 工程专家

**评审结论**：NomiFun 的 50+ 构建脚本 + CI/CD + i18n 类型生成 + theme contract 检查 + agent vocabulary 检查是工程化最佳实践，**应全部保留**。

**关键建议**：
1. 步骤 0.2 不要破坏 `bun.lock` / `Cargo.lock`：新增依赖后立即 `bun install` + `cargo fetch`，提交 lockfile。
2. 步骤 0.3 目录结构按蓝图创建，但不删除现有目录：现有 `crates/agent/` / `crates/backend/` / `crates/shared/` 保留，新增 `crates/sparkfox/` 子目录。
3. 步骤 0.3 每个新 crate 必须有 `Cargo.toml` + `src/lib.rs` + `README.md`（占位），否则 workspace 编译失败。
4. 步骤 0.4 入口文件改动要最小化：`ui/src/renderer/main.tsx` 仅修改 Arco `primaryColor` 为 Apple 系统蓝 `#007AFF`，不重写整个入口。
5. 步骤 0.4 路由表新增 `/agents` / `/monitor` / `/hotspot` / `/memory` 4 个路由，但不创建实际页面组件，仅 `<div>Placeholder</div>`。
6. 步骤 0.5 验证清单：`cargo check --workspace` + `bun run typecheck` + `bun run dev`（手动验证 5 分钟）。

### 🟡 产品专家

**评审结论**：用户明确要求"Apple 系统风格 + NomiFun UI"，但 NomiFun 默认主题是灰色（`primaryColor: '#4E5969'`），**需要在步骤 0.4 替换为 Apple 系统蓝**。

**关键建议**：
1. 步骤 0.2 新增 Apple 主题预设依赖：`ui/package.json` 加 `@tauri-apps/plugin-os`（检测系统深浅色）。
2. 步骤 0.4 创建 `ui/src/renderer/utils/theme/presets/macosLight.ts` / `macosDark.ts` / `macosAuto.ts` 三个 Apple 系统风格预设，内容为 CSS 变量映射。
3. 步骤 0.4 修改 `main.tsx` Arco `ConfigProvider` primaryColor：从 `#4E5969`（NomiFun 灰）改为 `#007AFF`（Apple 系统蓝）。
4. 步骤 0.4 侧边栏导航改为 SparkFox 6 大路由：对话/Agent/监视/热点/记忆/设置（覆盖 NomiFun 原导航）。
5. 步骤 0.5 验证时截图确认：首屏应看到 Apple 系统蓝主色 + macOS 圆角 + SF Pro 字体。

### ⚫ 风险评估专家

**评审结论**：步骤 0.2-0.5 属于"骨架搭建"阶段，风险等级**中**，主要风险 6 个。

**风险清单**：

| 风险 | 等级 | 影响 | 缓解措施 |
|------|------|------|---------|
| R1：automerge / double-ratchet / sqlite-vec 依赖版本不兼容 | 🔴 高 | 步骤 0.2 后 cargo build 失败 | 先用 `cargo search` 确认最新稳定版本，加依赖后立即 `cargo check` |
| R2：新增 sparkfox-* crate 漏写 `Cargo.toml` 字段导致 workspace 编译失败 | 🟡 中 | 步骤 0.3 后 cargo check 报错 | 用模板批量生成，每个 crate 创建后立即 `cargo check -p <name>` |
| R3：修改 `main.tsx` primaryColor 后 Arco 组件样式异常 | 🟡 中 | 步骤 0.5 首屏样式错乱 | 仅改 primaryColor 一个变量，不动画其他 Arco 配置；如异常可回退 |
| R4：CSP 严格化导致 Tauri webview 加载失败 | 🟡 中 | 步骤 0.5 白屏 | CSP 用宽松白名单（`default-src 'self' 'unsafe-inline'`），不强制 `unsafe-inline` 移除 |
| R5：AGPL license 替换后 NOTICE 文件缺失引发法律风险 | 🟢 低 | 后续发布受阻 | 步骤 0.2 同步创建 `NOTICE` 文件，明确 Apache-2.0 / AGPL-3.0 边界 |
| R6：bun install 拉取新依赖失败（网络问题） | 🟢 低 | 步骤 0.2 阻塞 | 配置 npm registry 镜像（`npmmirror.com`）+ cargo registry 镜像（`rsproxy.cn`） |

### 七专家共识

- ✅ 渐进改造路径正确
- ✅ 步骤 0.2-0.5 仅搭骨架，不实现业务逻辑
- ✅ 14 个 sparkfox-* crate 新建在 `crates/sparkfox/` 子目录
- ⚠️ 步骤 0.2 依赖版本需先 `cargo search` 确认
- ⚠️ 步骤 0.2 必须同步创建 NOTICE 文件
- ⚠️ 步骤 0.4 仅改 primaryColor + 加路由，不重写入口

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第二部分：步骤 0.2 详细计划 - 调整/创建项目基础配置
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 0.2.1 Rust 依赖增量（根 `Cargo.toml` `[workspace.dependencies]`）

**新增条目**（先 `cargo search` 确认版本）：
```toml
# CRDT（RFC-004）
automerge = "0.5"
# E2EE（RFC-004）
double-ratchet = "0.4"
# 向量检索（决策 5）
sqlite-vec = "0.1"
```

### 0.2.2 前端依赖增量（`ui/package.json`）

**新增 dependencies**：
```json
{
  "zustand": "^5.0.0",
  "three": "^0.169.0",
  "@types/three": "^0.169.0",
  "d3": "^7.9.0",
  "@types/d3": "^7.4.3"
}
```

### 0.2.3 品牌与 License 替换

| 文件 | 字段 | 旧值 | 新值 |
|------|------|------|------|
| `package.json` | `name` | `nomifun-tauri` | `sparkfox` |
| `package.json` | `license` | `Apache-2.0` | `AGPL-3.0-only` |
| `Cargo.toml` | `workspace.package.license` | `Apache-2.0` | `AGPL-3.0-only` |
| `Cargo.toml` | `workspace.package.repository` | `local/nomifun-tauri` | `local/sparkfox` |
| `apps/desktop/tauri.conf.json` | `productName` | `NomiFun` | `SparkFox` |
| `apps/desktop/tauri.conf.json` | `identifier` | `com.nomifun.desktop` | `com.sparkfox.desktop` |
| `apps/desktop/tauri.conf.json` | `deep-link.schemes` | `["nomifun"]` | `["sparkfox"]` |
| `apps/desktop/tauri.conf.json` | `updater.endpoints` | `github.com/nomifun/...` | `github.com/sparkfox/...`（占位） |
| `apps/desktop/tauri.conf.json` | `app.security.csp` | `null` | 显式白名单 |
| `ui/package.json` | `name` | `@nomifun-tauri/ui` | `@sparkfox/ui` |

### 0.2.4 创建 NOTICE 文件

根目录新建 `NOTICE` 文件，明确第三方代码来源与 License 边界。

### 0.2.5 镜像配置（如需）

如 `bun install` / `cargo fetch` 网络慢，配置：
- npm 镜像：`.npmrc` → `registry=https://registry.npmmirror.com/`
- cargo 镜像：`.cargo/config.toml` 加 `[source.crates-io]` + `replace-with`

### 0.2.6 验证

```powershell
cargo check --workspace
bun install
bun run typecheck
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第三部分：步骤 0.3 详细计划 - 创建目录结构
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 0.3.1 新增 14 个 sparkfox-* crate（`crates/sparkfox/`）

按 RFC-001 创建：
```
crates/sparkfox/
├── sparkfox-core/              # 核心类型与接口
├── sparkfox-memory/            # 6 层记忆 L0-L5
├── sparkfox-orchestrator/      # DAG 编排
├── sparkfox-agent/             # Agent 引擎
├── sparkfox-chat/              # 对话引擎
├── sparkfox-thinking/          # 思考过程流
├── sparkfox-hotspot/           # 热点追踪
├── sparkfox-monitor/           # 监视数据
├── sparkfox-crdt/              # automerge-rs 封装
├── sparkfox-e2ee/              # Double Ratchet
├── sparkfox-store/             # SQLite + sqlite-vec
├── sparkfox-ipc/               # Tauri IPC 桥
├── sparkfox-llm/               # LLM Provider
└── sparkfox-security/          # 11 安全栈
```

每个 crate 包含：
- `Cargo.toml`（name + version.workspace + edition.workspace + license.workspace + 占位 deps）
- `src/lib.rs`（`//! SparkFox <module> — 占位` + `pub fn init() {}`）
- `README.md`（一行说明 + License + 来源）

### 0.3.2 修改根 `Cargo.toml` workspace members

新增 `"crates/sparkfox/*"` 到 members 列表。

### 0.3.3 新增前端目录（`ui/src/renderer/`）

```
ui/src/renderer/
├── components/
│   ├── chat/                   # 模块 E（BaiLongma 对话）
│   ├── thinking/               # 模块 F（BaiLongma 思考过程）
│   ├── agent/                  # 模块 B（OpenAkita Agent 菜单）
│   ├── monitor/                # 模块 C（OpenAkita 监视面板）
│   ├── hotspot/                # 模块 G（BaiLongma 热点追踪）
│   ├── memory/                 # 模块 D UI（OpenAkita 记忆管理）
│   ├── settings/               # 模块 J（NomiFun 设置）
│   └── companion/              # NomiFun 桌宠（保留）
├── views/
│   ├── ChatView/               # 主对话页
│   ├── AgentView/              # Agent 管理页
│   ├── MonitorView/            # 监视面板页
│   ├── HotspotView/            # 热点追踪页
│   ├── MemoryView/             # 记忆管理页
│   └── SettingsView/           # 设置页
├── store/                      # Zustand stores
│   ├── chatStore.ts
│   ├── agentStore.ts
│   ├── memoryStore.ts
│   ├── monitorStore.ts
│   ├── thinkingStore.ts
│   ├── hotspotStore.ts
│   ├── settingsStore.ts
│   └── sceneStore.ts
├── router/                     # 路由配置
│   ├── index.tsx
│   ├── routes.tsx
│   └── shortcuts.ts
├── hooks/sparkfox/             # SparkFox 专用 hooks
└── utils/theme/presets/        # Apple 主题预设
    ├── macosLight.ts
    ├── macosDark.ts
    └── macosAuto.ts
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第四部分：步骤 0.4 详细计划 - 创建入口文件和基础配置
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 0.4.1 修改 `ui/src/renderer/main.tsx`

**仅 2 处改动**：
1. Arco `ConfigProvider` 的 `primaryColor`：`'#4E5969'` → `'#007AFF'`（Apple 系统蓝）
2. 加载 Apple 主题预设 CSS（在 `themes/index.css` 之后）

### 0.4.2 创建 Apple 主题预设（3 个文件）

- `macosLight.ts`：CSS 变量映射（亮色）
- `macosDark.ts`：CSS 变量映射（暗色）
- `macosAuto.ts`：跟随系统（`prefers-color-scheme`）

### 0.4.3 创建路由配置（`ui/src/renderer/router/`）

- `index.tsx`：`createBrowserRouter` + `RouterProvider`
- `routes.tsx`：6 大路由
- `shortcuts.ts`：Cmd/Ctrl+1~5 + Cmd/Ctrl+,

### 0.4.4 创建 8 个 Zustand store（占位）

### 0.4.5 创建 6 个 View 占位

### 0.4.6 修改 Sider 导航

导航项改为：对话 / Agent / 监视 / 热点 / 记忆 / 设置

### 0.4.7 修改 `apps/desktop/tauri.conf.json`

CSP + 品牌替换

### 0.4.8 修改 `apps/desktop/src/main.rs`

仅改 2 处文案：title + tooltip

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第五部分：步骤 0.5 详细计划 - 验证骨架能跑起来
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 0.5.1 验证清单

| 检查项 | 命令 | 期望结果 |
|--------|------|---------|
| 1. Rust workspace 编译 | `cargo check --workspace` | 0 error |
| 2. Rust 测试编译 | `cargo test --workspace --no-run` | 0 error |
| 3. 前端类型检查 | `bun run typecheck` | 0 error |
| 4. 前端构建 | `bun run build:ui` | 0 error，生成 `ui/dist/` |
| 5. Tauri dev 启动 | `bun run dev` | Tauri 窗口弹出，加载 `http://localhost:5173` |
| 6. 首屏渲染 | 手动观察 | 看到 SparkFox 6 大导航项 + Apple 系统蓝主色 |
| 7. 路由跳转 | 手动点击 | 6 个路由都能跳转，显示 Placeholder 文案 |
| 8. DevTools 控制台 | F12 | 无 error（warning 可接受） |
| 9. 内存占用 | Activity Monitor / Task Manager | <100MB |
| 10. 首屏加载时间 | Performance 面板 | <2s |

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第六部分：待确认事项的大白话解释
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

### 待确认事项 1：改造方式 - "渐进改造" vs "推倒重建"

**技术术语**：渐进改造路径 vs 重建

**大白话解释**：
- 你现在 D:\xin kaifa\SparkFox 文件夹里其实是一份完整的 NomiFun 项目（一个已经能跑的软件）。
- **"渐进改造"** 意思是：在这个能跑的项目上动手改，一步一步把它变成 SparkFox。好处是不丢掉 NomiFun 已经写好的好用代码（比如窗口管理、托盘、自动更新、50 多个工具脚本），坏处是项目里会暂时混着 "nomifun" 和 "sparkfox" 两种名字。
- **"推倒重建"** 意思是：把现在的文件夹删了，从空文件夹开始建一个全新的 SparkFox 项目。好处是名字干净，坏处是 NomiFun 那些打磨了几千小时的代码全没了，要重新写，可能要多花几个月。

**七专家建议**：渐进改造（在现有基础上改）。

**你的选择**：
- A. 同意渐进改造（推荐）
- B. 要推倒重建
- C. 不确定，需要更多说明

---

### 待确认事项 2：新代码放哪里 - "14 个 crate 新建在 crates/sparkfox/"

**技术术语**：14 个 sparkfox-* crate 新建在 `crates/sparkfox/` 子目录，不重命名旧 crate

**大白话解释**：
- "crate" 是 Rust 语言里对"一个代码模块/包"的叫法，就像 JavaScript 里的"一个 npm 包"。
- 现在 SparkFox 项目里有 46 个 NomiFun 留下来的代码包（名字都是 `nomi-xxx` 或 `nomifun-xxx`）。
- 蓝图规划 SparkFox 自己需要 14 个新代码包（名字是 `sparkfox-xxx`，比如 `sparkfox-memory` 记忆系统、`sparkfox-crdt` 同步、`sparkfox-e2ee` 加密等）。
- **问题**：这 14 个新包放哪里？要不要把旧的 46 个包改名？
- **方案 A（专家建议）**：新包放在 `crates/sparkfox/` 这个新文件夹下，旧的 46 个包名字不动。好处是改动小、不容易出错；坏处是项目里会同时有 nomifun 和 sparkfox 两种名字的包。
- **方案 B**：把旧的 46 个包全改名为 sparkfox-xxx。好处是名字统一；坏处是要改 46 个配置文件 + 几千处引用，2-3 周改不完，还容易改错。

**你的选择**：
- A. 同意方案 A：新包放新文件夹，旧包不改名（推荐）
- B. 要方案 B：全部改名
- C. 不确定，需要更多说明

---

### 待确认事项 3：软件许可证 - "AGPL 替换 Apache-2.0"

**技术术语**：AGPL-3.0 替换 Apache-2.0 + 创建 NOTICE 文件

**大白话解释**：
- 软件许可证（License）就是"别人能不能用你的代码、怎么用"的法律规则。
- NomiFun 用的是 **Apache-2.0**（比较宽松：别人可以拿去用、可以闭源、可以卖钱）。
- 蓝图规划 SparkFox 要用 **AGPL-3.0**（比较严格：别人如果拿去用、做成网络服务给别人用，必须也把改过的代码全部公开）。
- **为什么用 AGPL**：你之前定的"数据主权"理念（"别把第二大脑租给别人"），AGPL 能强制后续使用者必须公开代码，符合"开源精神"。
- **法律上能不能改**：能。Apache-2.0 允许别人改成 AGPL（这叫"兼容"）。
- **NOTICE 文件**：一份声明文件，写清楚"哪些代码是 NomiFun 的、哪些是 SparkFox 新写的、各自用什么许可证"。这是法律要求，不发 NOTICE 就是侵权。

**你的选择**：
- A. 同意改成 AGPL-3.0 + 创建 NOTICE 文件（推荐，符合你之前定的"数据主权"理念）
- B. 保留 Apache-2.0 不改
- C. 用别的许可证（比如 MIT、GPL）
- D. 不确定，需要更多说明

---

### 待确认事项 4：要装哪些新的第三方库

**技术术语**：步骤 0.2 依赖增量是否确认 - Rust: automerge / double-ratchet / sqlite-vec；前端: zustand / three / d3

**大白话解释**：
- "依赖"就是"别人写好的、我们直接拿来用的代码库"。
- 蓝图规划 SparkFox 需要装 6 个新的依赖：

| 名字 | 是什么 | 用在哪里 | 通俗比喻 |
|------|--------|---------|---------|
| **automerge** | Rust 库 | 多设备同步记忆 | 像"石墨文档"那样，多台电脑改同一份记忆不会冲突 |
| **double-ratchet** | Rust 库 | 端到端加密 | 像"Signal/微信加密聊天"那样，别人偷不到内容 |
| **sqlite-vec** | Rust 库 | 记忆向量检索 | 像"语义搜索"——搜"开心的事"能找到"快乐、高兴"的相关记忆 |
| **zustand** | JS 库 | 前端状态管理 | 像"全局变量"，但更智能，能让组件按需订阅 |
| **three** | JS 库 | 3D 地球 | 热点追踪页那个 3D 转动的地球就是用它画的 |
| **d3** | JS 库 | 2D 数据图表 | 记忆图谱那个力导向图就是用它画的 |

- 这 6 个库都是免费的、开源的、经过很多人验证的。

**你的选择**：
- A. 同意装这 6 个库（推荐）
- B. 有想删的（说明哪个）
- C. 有想加的（说明加什么）
- D. 不确定，需要更多说明

---

### 待确认事项 5：要不要配国内镜像

**技术术语**：是否需要配 npm/cargo 镜像？

**大白话解释**：
- 装依赖要从国外的服务器下载，国内网络可能很慢甚至失败。
- "镜像"就是国内的"中转服务器"，把国外的库复制一份到国内，下载快很多。
- npm 镜像（前端库用）：比如淘宝的 `npmmirror.com`
- cargo 镜像（Rust 库用）：比如字节的 `rsproxy.cn`

**你的选择**：
- A. 配镜像（推荐，国内网络必配）
- B. 不配（我有梯子/网络很好）
- C. 不确定，需要更多说明

---

### 待确认事项 6：目录结构按蓝图建

**技术术语**：步骤 0.3 目录结构是否按蓝图创建 - 14 个 sparkfox-* crate / 8 个 Zustand store / 6 个 View 占位 / 3 个 Apple 主题预设

**大白话解释**：
- 蓝图里规划了 SparkFox 的文件夹结构（哪个文件夹放什么代码）。
- 步骤 0.3 要按蓝图建好所有空文件夹 + 占位文件（占位文件就是只有一句注释、没有实际功能的文件，先把位置占住）。
- 具体要建：
  - 14 个 Rust 代码包的空文件夹（在 `crates/sparkfox/` 下）
  - 8 个前端状态管理的空文件（在 `ui/src/renderer/store/` 下）
  - 6 个页面的空文件（在 `ui/src/renderer/views/` 下，分别是对话/Agent/监视/热点/记忆/设置 6 个页面）
  - 3 个 Apple 主题风格的空文件（亮色/暗色/跟随系统）

**你的选择**：
- A. 按蓝图建（推荐）
- B. 想改蓝图里的结构（说明改哪里）
- C. 不确定，需要更多说明

---

### 待确认事项 7：哪些文件能改、哪些不能动

**技术术语**：步骤 0.4 改动范围是否确认 - 仅改 main.tsx primaryColor / 仅改 main.rs 2 处文案 / Sider 导航改为 6 大路由

**大白话解释**：
- NomiFun 的代码里有几个"关键入口文件"，改错了整个软件就跑不起来。
- 七专家建议：步骤 0.4 只动 3 个地方，其他都不动：
  1. **前端入口 `main.tsx`**：只改一个颜色值（主色调，从 NomiFun 的灰色 `#4E5969` 改成 Apple 系统蓝 `#007AFF`），其他不动。
  2. **后端入口 `main.rs`**：只改 2 处文字（窗口标题 "NomiFun" → "SparkFox"、托盘提示 "NomiFun" → "SparkFox"），其他不动。
  3. **侧边栏导航**：把 NomiFun 原来的导航项（会话/终端/知识等）改成 SparkFox 的 6 大路由（对话/Agent/监视/热点/记忆/设置）。
- **不动的地方**：NomiFun 的窗口管理、托盘、自动更新、桌宠、PWA、i18n、50+ 构建脚本全部保留。

**你的选择**：
- A. 同意这个改动范围（推荐，最小改动最安全）
- B. 想多改一些（说明改哪里）
- C. 不确定，需要更多说明

---

### 待确认事项 8：怎么算搭建成功

**技术术语**：步骤 0.5 验证标准是否采纳 - 10 项验证清单 / 是否需要截图

**大白话解释**：
- 搭完骨架后要检查"骨架能不能跑起来"，需要一份检查清单。
- 10 项检查：

| 序号 | 检查内容 | 大白话 |
|------|---------|--------|
| 1 | Rust 编译通过 | 后端代码没有语法错误 |
| 2 | Rust 测试编译通过 | 测试代码没有语法错误 |
| 3 | 前端类型检查通过 | 前端代码没有类型错误 |
| 4 | 前端构建成功 | 前端能打包成可发布的文件 |
| 5 | Tauri dev 启动 | 双击能打开 SparkFox 窗口 |
| 6 | 首屏渲染 | 窗口里能看到 6 个导航项 + Apple 蓝色 |
| 7 | 路由跳转 | 点击 6 个导航项能切换页面 |
| 8 | 控制台无错误 | F12 看不到红色报错 |
| 9 | 内存占用 <100MB | 任务管理器里 SparkFox 进程内存 <100MB |
| 10 | 首屏加载 <2 秒 | 从启动到看到界面不超过 2 秒 |

- 截图：建议要，方便留档对比。

**你的选择**：
- A. 同意这 10 项检查 + 要截图（推荐）
- B. 10 项太多，想精简（说明保留哪几项）
- C. 不需要截图
- D. 不确定，需要更多说明

---

### 待确认事项 9：执行节奏 - 一步步来还是一次做完

**技术术语**：是否需要分批执行（每完成一步等确认再继续）vs 一次性执行 0.2→0.5

**大白话解释**：
- 4 个步骤（0.2 装依赖 + 0.3 建目录 + 0.4 改入口 + 0.5 验证）有两种执行方式：
- **方案 A（分批）**：每做完一步停下来等你确认，没问题再继续下一步。好处：每一步都能控制、出错好回退；坏处：慢，需要你多次确认。
- **方案 B（一次性）**：从 0.2 一口气做到 0.5，中间不停。好处：快；坏处：如果中间某步出错，回退比较麻烦。

**你的选择**：
- A. 分批执行（推荐，安全）
- B. 一次性执行（快）
- C. 不确定，需要更多说明

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## 第七部分：执行顺序总览
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

```
步骤 0.2：调整/创建项目基础配置
├── 0.2.1 cargo search 确认依赖版本
├── 0.2.2 修改根 Cargo.toml 加 Rust 依赖
├── 0.2.3 修改 ui/package.json 加前端依赖
├── 0.2.4 bun install + cargo fetch
├── 0.2.5 品牌替换（package.json / Cargo.toml / tauri.conf.json）
├── 0.2.6 创建 NOTICE 文件
└── 0.2.7 cargo check + bun typecheck 验证

步骤 0.3：创建目录结构
├── 0.3.1 创建 14 个 sparkfox-* crate 目录与占位文件
├── 0.3.2 修改根 Cargo.toml workspace members
├── 0.3.3 创建前端目录与 .gitkeep
└── 0.3.4 cargo check 验证

步骤 0.4：创建入口文件和基础配置
├── 0.4.1 修改 main.tsx primaryColor
├── 0.4.2 创建 Apple 主题预设 3 个文件
├── 0.4.3 创建 router/ 配置 3 个文件
├── 0.4.4 创建 8 个 Zustand store 占位
├── 0.4.5 创建 6 个 View 占位
├── 0.4.6 修改 Sider 导航
├── 0.4.7 修改 tauri.conf.json CSP + 品牌
├── 0.4.8 修改 main.rs 文案
└── 0.4.9 cargo check + typecheck 验证

步骤 0.5：验证骨架能跑起来
├── 0.5.1 cargo check --workspace
├── 0.5.2 bun run typecheck
├── 0.5.3 bun run build:ui
├── 0.5.4 bun run dev（手动验证）
└── 0.5.5 截图 + 日志输出
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

**文档版本**：v1.0
**生成时间**：2026-07-18
**下一步**：用户确认 9 个待确认事项后，开始执行步骤 0.2.1（cargo search 确认依赖版本）
