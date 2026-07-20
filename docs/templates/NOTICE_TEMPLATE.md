# SparkFox NOTICE 文件模板

> **用途**：本模板用于规范 SparkFox 全局 NOTICE 与各 crate NOTICE 文件的结构与内容。
> 新建 crate 或更新 NOTICE 时，复制本模板对应章节并填充实际值。
>
> **维护者**：SparkFox Contributors
> **更新日期**：2026-07-21（Sub-Step 12.6.1 REFACTOR 阶段提取）
> **关联文档**：[合规审计清单](../合规审计清单.md) / [compliance_check.sh](../../scripts/compliance_check.sh)

## 合规要求（必须遵守）

1. **AGPL-3.0-only 主许可证**：全局 NOTICE 必须含 AGPL-3.0-only 字符串声明
2. **上游致谢**：每个 crate NOTICE 必须含 `致谢` / `Attribution` / `上游` 关键词之一
   （Apache-2.0 §4(d) 与 MIT 许可证共同要求）
3. **清洁室 / schema borrowing 声明**：衍生自上游的代码必须标注重写方式
4. **许可证兼容性**：仅允许 MIT / Apache-2.0 / CC BY-SA 4.0（test-only）依赖
5. **致谢规范**：每项上游依赖需列出 License / Source / Use / Copyright 四字段

---

## 模板 A：全局 NOTICE（根目录 NOTICE 文件）

```text
SparkFox
Copyright 2026 SparkFox Contributors

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published
by the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
                      Third-Party Notices
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

This product includes code derived from the following upstream projects.
Each project's original license terms apply to its respective contributions;
SparkFox original code is licensed under AGPL-3.0-only.

───────────────────────────────────────────────────────────────────────
1. <上游项目名>（<URL 或来源说明>）
───────────────────────────────────────────────────────────────────────
License: <MIT / Apache-2.0 / Proprietary 等>
Original Copyright: <Copyright 行>

Contributions to SparkFox:
  - <贡献项 1>
  - <贡献项 2>

License Compatibility: <与 AGPL 兼容性说明>
  See: https://www.gnu.org/licenses/license-compatibility.en.html

Clean-Room Rewrite Requirement (RFC-001):
  <若是 MIT 上游，需声明清洁室重写要求：Team A 写规格 → Team B 实现>

Schema Borrowing Statement (C-02):
  <若是 SAG 等 schema 借用，需声明字段级映射 ≠ 清洁室重写>

License Verification (C-01, resolved <日期>):
  <许可证验证记录与参考文档>

───────────────────────────────────────────────────────────────────────
N. vX.Y.Z 新增第三方依赖致谢（Third-Party Dependencies Introduced in vX.Y.Z）
───────────────────────────────────────────────────────────────────────
说明（致谢规范）：
  本章节依据 Apache-2.0 §4(d) 与 MIT 许可证条款，对 vX.Y.Z 引入的上游依赖
  进行显式致谢。所有依赖仅以 Rust crate 形式静态链接，未修改其源代码。
  上游许可证原文保留在各自 crate 仓库中（见每项 Source 字段）。
  AGPL-3.0-only 与 MIT / Apache-2.0 / CC BY-SA 4.0 兼容（详见下方矩阵）。

【<功能分类，如：SAG 中文检索增强流程依赖>】

N.1 <依赖名>（<用途简述>）
    - License: <MIT / Apache-2.0 / MIT OR Apache-2.0 / CC BY-SA 4.0 等>
    - Source: <上游仓库 URL>
    - Use: <在 SparkFox 中的使用位置，指明 crate 与模块路径>
    - Copyright: <Copyright 行>
    - 致谢上游：<原作者 / 项目贡献者>
    - Note: <可选：补充说明，如 unsafe 局部允许、Windows 兼容性等>

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
                      License Compatibility Matrix
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

┌─────────────────────────┬──────────────────────┬───────────────────┬──────────────────────┐
│ Upstream Project        │ License              │ Compatible w/AGPL?│ Inclusion Mode       │
├─────────────────────────┼──────────────────────┼───────────────────┼──────────────────────┤
│ <项目名>                 │ <许可证>              │ Yes / N/A         │ <Direct/Clean-room>  │
└─────────────────────────┴──────────────────────┴───────────────────┴──────────────────────┘

SparkFox original code: AGPL-3.0-only
  - All new Rust crates under crates/sparkfox/
  - All new TypeScript modules under ui/src/renderer/views/SparkFox*/

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
                      Attribution Retention
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Per Apache-2.0 Section 4(d): All attribution notices in <上游项目> source files
are retained in their original form. Files modified by SparkFox contributors
include an additional notice stating "Modified by SparkFox Contributors, 2026".

Per MIT license: The <上游项目> copyright notice and permission notice are
retained in all <clean-room rewrite / schema borrowing> files that derive from
<上游项目> concepts.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

For questions about licensing, contact: SparkFox Contributors
For the full text of AGPL-3.0, see: LICENSE file
For the full text of Apache-2.0, see: LICENSES/Apache-2.0.txt
For the full text of MIT, see: LICENSES/MIT.txt
```

---

## 模板 B：crate 级 NOTICE（`crates/sparkfox/<crate-name>/NOTICE`）

```text
<crate-name>
Copyright 2026 SparkFox Contributors (AGPL-3.0-only)

本 crate 是 <crate 用途简述>，<vX.Y.Z 引入的依赖说明>。
本 NOTICE 致谢上游（致谢规范：依据 Apache-2.0 §4(d) / MIT / CC BY-SA 4.0）。

This product includes software and design concepts derived from the following
upstream projects (attribution retained per upstream license terms):

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1. <上游项目名>（<项目说明>）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   - Repository: <上游仓库 URL 或本地路径>
   - License: <许可证>（Copyright <版权行>）— verified <验证日期>
   - Use: <在本 crate 中的使用位置，指明源文件路径>
   - Note: <可选：清洁室重写 / schema borrowing / 间接依赖说明>

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
N. vX.Y.Z 新增第三方依赖致谢（Third-Party Dependencies — Attribution）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

N.1 <依赖名> <版本>（<用途简述>）
    - License: <MIT / Apache-2.0 / MIT OR Apache-2.0 等>
    - Source: <上游仓库 URL>
    - Use: <src/<模块>.rs 中的使用位置与用途>
    - Tests: <tests/<测试>.rs 中的测试覆盖>
    - Copyright: <Copyright 行>
    - 致谢上游：<原作者 / 贡献者>

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

All <本 crate 原创 trait / 实现 / 测试> are independently authored by
SparkFox Contributors and licensed under AGPL-3.0-only.

For questions about licensing, contact: SparkFox Contributors
For the full text of AGPL-3.0, see: /LICENSE
For the full text of MIT / Apache-2.0, see upstream repositories linked above.
```

---

## 字段填写规范

### License 字段允许值

| 许可证 | 写法 | AGPL 兼容性 | 适用场景 |
|--------|------|------------|---------|
| MIT | `MIT` | ✅ Yes | 大多数 Rust crate |
| Apache-2.0 | `Apache-2.0` | ✅ Yes | NomiFun / DuReader |
| MIT OR Apache-2.0 | `MIT OR Apache-2.0 (dual-licensed, 选择 Apache-2.0 路径)` | ✅ Yes | petgraph / candle / hnsw_rs |
| CC BY-SA 4.0 | `CC BY-SA 4.0` | ⚠️ 仅 test-only | CMRC2018 数据集 |
| Proprietary | `Proprietary (blueprint reference only)` | N/A | Pangu Nebula / OpenAkita |

### Inclusion Mode 字段允许值

| 模式 | 说明 | 示例 |
|------|------|------|
| `Direct code reuse` | 直接复用上游代码（Apache-2.0 兼容） | NomiFun |
| `Clean-room rewrite` | 清洁室重写（Team A 规格 → Team B 实现） | BaiLongma / OpenAkita |
| `Schema borrowing` | 字段级映射，非清洁室重写（C-02 决策） | SAG |
| `Crate dependency` | Rust crate 静态链接依赖 | jieba-rs / petgraph / candle |
| `Static model weights` | 静态模型权重加载 | XLM-RoBERTa |
| `Test fixtures only` | 仅测试数据，不编译进发行版 | DuReader / CMRC2018 |
| `Interface reserved` | 接口预留，未实际启用 | sqlite-vec |
| `Blueprint reference` | 仅架构蓝图参考，不含代码 | Pangu Nebula |

---

## 使用示例

### 新增 crate 的 NOTICE 步骤

1. 复制「模板 B」到 `crates/sparkfox/<new-crate>/NOTICE`
2. 替换 `<crate-name>`、`<crate 用途简述>` 等占位符
3. 列出该 crate 的 `Cargo.toml` 中所有非 workspace 内部依赖
4. 为每个依赖填写 License / Source / Use / Copyright 字段
5. 运行 `bash scripts/compliance_check.sh` 验证通过

### 新版本引入新依赖的步骤

1. 在全局 `NOTICE` 第 N 章节追加新依赖致谢项
2. 在对应 crate 的 `NOTICE` 第 N 章节追加详细致谢
3. 更新 License Compatibility Matrix 表格
4. 运行 `bash scripts/compliance_check.sh` 验证通过
5. 更新 `docs/合规审计清单.md` 的变更历史

---

## 相关文档

- [全局 NOTICE](../../NOTICE) — 实际全局 NOTICE 文件
- [合规审计清单](../合规审计清单.md) — 8 项合规检查项
- [compliance_check.sh](../../scripts/compliance_check.sh) — 自动化合规检查脚本
- [RFC-001-crate-boundaries](../rfc/RFC-001-crate-boundaries.md) — crate 边界与清洁室重写规范
