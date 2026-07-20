# SparkFox Crate 重命名映射表

> **目的**：将项目中残留的 nomifun-*/nomi-* crate 全部重命名为 sparkfox-* 命名，统一项目品牌。
>
> **命名方案**：
> - `nomifun-*` (backend) → `sparkfox-be-*`
> - `nomi-*` (shared) → `sparkfox-sh-*`
> - `nomi-*` (agent) → `sparkfox-ag-*`
> - `nomifun-desktop` → `sparkfox-desktop`
> - `nomifun-web` → `sparkfox-web`

---

## 一、Backend crates (crates/backend/) — 31 个

| # | 旧名 | 新名 |
|---|---|---|
| 1 | nomifun-agent-execution | sparkfox-be-agent-execution |
| 2 | nomifun-ai-agent | sparkfox-be-ai-agent |
| 3 | nomifun-api-types | sparkfox-be-api-types |
| 4 | nomifun-app | sparkfox-be-app |
| 5 | nomifun-assets | sparkfox-be-assets |
| 6 | nomifun-auth | sparkfox-be-auth |
| 7 | nomifun-channel | sparkfox-be-channel |
| 8 | nomifun-common | sparkfox-be-common |
| 9 | nomifun-companion | sparkfox-be-companion |
| 10 | nomifun-conversation | sparkfox-be-conversation |
| 11 | nomifun-creation | sparkfox-be-creation |
| 12 | nomifun-cron | sparkfox-be-cron |
| 13 | nomifun-db | sparkfox-be-db |
| 14 | nomifun-extension | sparkfox-be-extension |
| 15 | nomifun-file | sparkfox-be-file |
| 16 | nomifun-gateway | sparkfox-be-gateway |
| 17 | nomifun-idmm | sparkfox-be-idmm |
| 18 | nomifun-knowledge | sparkfox-be-knowledge |
| 19 | nomifun-mcp | sparkfox-be-mcp |
| 20 | nomifun-office | sparkfox-be-office |
| 21 | nomifun-preset | sparkfox-be-preset |
| 22 | nomifun-public | sparkfox-be-public |
| 23 | nomifun-public-agent | sparkfox-be-public-agent |
| 24 | nomifun-realtime | sparkfox-be-realtime |
| 25 | nomifun-requirement | sparkfox-be-requirement |
| 26 | nomifun-runtime | sparkfox-be-runtime |
| 27 | nomifun-secret | sparkfox-be-secret |
| 28 | nomifun-shell | sparkfox-be-shell |
| 29 | nomifun-system | sparkfox-be-system |
| 30 | nomifun-terminal | sparkfox-be-terminal |
| 31 | nomifun-webhook | sparkfox-be-webhook |
| 32 | nomifun-workshop | sparkfox-be-workshop |

## 二、Shared crates (crates/shared/) — 3 个

| # | 旧名 | 新名 |
|---|---|---|
| 1 | nomifun-net | sparkfox-be-net |
| 2 | nomi-redact | sparkfox-sh-redact |
| 3 | nomi-process-runtime | sparkfox-sh-process-runtime |

## 三、Agent crates (crates/agent/) — 15 个

| # | 旧名 | 新名 |
|---|---|---|
| 1 | nomi-a11y | sparkfox-ag-a11y |
| 2 | nomi-agent | sparkfox-ag-agent |
| 3 | nomi-browser | sparkfox-ag-browser |
| 4 | nomi-browser-engine | sparkfox-ag-browser-engine |
| 5 | nomi-cli | sparkfox-ag-cli |
| 6 | nomi-compact | sparkfox-ag-compact |
| 7 | nomi-computer | sparkfox-ag-computer |
| 8 | nomi-config | sparkfox-ag-config |
| 9 | nomi-mcp | sparkfox-ag-mcp |
| 10 | nomi-memory | sparkfox-ag-memory |
| 11 | nomi-protocol | sparkfox-ag-protocol |
| 12 | nomi-providers | sparkfox-ag-providers |
| 13 | nomi-skills | sparkfox-ag-skills |
| 14 | nomi-tools | sparkfox-ag-tools |
| 15 | nomi-types | sparkfox-ag-types |

## 四、Apps — 2 个

| # | 旧名 | 新名 |
|---|---|---|
| 1 | nomifun-desktop | sparkfox-desktop |
| 2 | nomifun-web | sparkfox-web |

## 五、其他标识符

| 类别 | 旧值 | 新值 |
|---|---|---|
| URI scheme | nomifun:// | sparkfox:// |
| 环境变量 | NOMIFUN_* | SPARKFOX_* |
| Bundle ID | com.nomifun.* | com.sparkfox.* |
| macOS bundle | NomiFun.app | SparkFox.app |
| 构建清单 | nomifun-build.json | sparkfox-build.json |
| 构建环境变量 | NOMIFUN_FRONTEND_BUILD_ID | SPARKFOX_FRONTEND_BUILD_ID |
| exe 名 | nomifun-desktop.exe | sparkfox-desktop.exe |

## 六、Rust 标识符替换规则

Rust crate 名在代码中用下划线形式（连字符转下划线）：

| 旧 crate 引用 | 新 crate 引用 |
|---|---|
| nomifun_common | sparkfox_be_common |
| nomifun_app | sparkfox_be_app |
| nomi_types | sparkfox_ag_types |
| nomi_agent | sparkfox_ag_agent |
| nomi_redact | sparkfox_sh_redact |
| ... | ...（按上述映射类推） |

## 七、保留不改的内容

以下内容是 AGPL/MIT/Apache 协议要求的法律声明，**绝对不能修改**：

1. NOTICE 文件中的上游项目归属声明（"NomiFun" 作为上游项目名）
2. README 中的"清洁室重写"、"设计参考 NomiFun"段落
3. 源代码注释中的"NOTICE: NomiFun MIT"、"清洁室重写"等技术声明
4. 致谢行（如 SettingsView 中的"BaiLongma · NomiFun · OpenAkita · Pangu Nebula"）
5. CHANGELOG.md 中的历史记录（"NomiFun is pre-1.0"是历史事实）
6. CODE_OF_CONDUCT.md 中的社区名引用
7. mock 数据中的历史事实记录
8. `agent_type === 'nomi'` / `backend === 'nomi'` API 字段值
