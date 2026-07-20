# 中文多跳 Benchmark 数据集（Sub-Step 12.3.1）

> SparkFox v1.1.0 第十九波 Sub-Step 12.3.1 — 用于 4 策略对比测试（12.3.2）与 Recall@10 > 0.85 调优（12.3.3）的中文多跳检索评测数据集。

## 数据集规模

| 文件 | 条目数 | 说明 |
| --- | ---: | --- |
| `entities.json`   |  200 | 11 类实体（人/地/机构/时间/事件/概念/产品/软件/硬件/文档/其他） |
| `events.json`     |  500 | 1-3 句中文描述，每事件关联 1-5 个实体 |
| `relations.json`  | 1500 | event ↔ entity 关系（含 relation_type 语义角色） |
| `queries.json`    |   50 | 查询 + ground truth（expected_event_ids + expected_hop） |

## 主题选择

围绕 **中国科技场景**（互联网公司 / 技术概念 / 产品发布 / 创始人动向）构造，覆盖：
- 8 位虚构中文姓名（张三/李四/...）+ 32 位真实科技创始人（马化腾/雷军/张一鸣/...）
- 30 个中国城市（北京/上海/深圳/杭州/...）
- 30 个中国科技公司（腾讯/阿里/字节/百度/美团/...）
- 20 个技术概念（大模型/智能体/RAG/向量数据库/多跳检索/...）
- 15 个产品（微信/抖音/飞书/钉钉/...）
- 10 个软件（SparkFox/Tauri/Rust/TypeScript/...）

## 11 类实体分布（200）

| entity_type_id | entity_type     | 数量 | 示例 |
| -------------: | --------------- | ---: | --- |
| 0  | PERSON         | 40 | 张三 / 李四 / 马化腾 / 张一鸣 |
| 1  | LOCATION       | 30 | 北京 / 上海 / 深圳 / 杭州 |
| 2  | ORGANIZATION   | 30 | 腾讯 / 阿里巴巴 / 字节跳动 / 百度 |
| 3  | TIME           | 20 | 2023年 / 春季 / 一月 / 上半年 |
| 4  | EVENT          | 20 | 产品发布 / 年度大会 / 战略收购 / 融资轮次 |
| 5  | CONCEPT        | 20 | 大模型 / 智能体 / RAG / 向量数据库 |
| 6  | ARTIFACT       | 15 | 微信 / 抖音 / 飞书 / 钉钉 |
| 7  | SOFTWARE       | 10 | SparkFox / Tauri / Rust / TypeScript |
| 8  | HARDWARE       |  5 | iPhone / Mac / 服务器 / GPU加速卡 |
| 9  | DOCUMENT       |  5 | README文档 / 设计文档 / 技术规范 |
| 10 | OTHER          |  5 | 开源协议 / 测试数据集 / 工具链 |

## 查询跳数分布（50）

| expected_hop | 数量 | 说明 |
| -----------: | ---: | --- |
| 1 | 15 | 单跳：查询直接命中一个实体（如「张三参加了什么会议？」） |
| 2 | 20 | 双跳：查询含 2 个实体，需 2 跳 BFS（如「张三在北京参加了哪些腾讯的活动？」） |
| 3 | 15 | 三跳：查询含 3+ 实体，需 3 跳 BFS（如「2023年张三通过飞书与李四合作的大模型项目是什么？」） |

## 事件 hop 分布（500）

- hop=1：59 个（单实体直接命中）
- hop=2：279 个（2 实体关联）
- hop=3：162 个（3+ 实体关联）

## 数据生成方式

**手工构造 + 模板化**（`_generate.py`）：
1. **实体**：手工列举 11 类共 200 个真实/合理的中文实体名（含真实公司、城市、创始人姓名）。
2. **事件**：30 个中文事件模板（如「{P}在{L}参加了{O}的产品发布会。」），随机填充实体生成 500 条事件，去重保证唯一。
3. **关系**：从每个事件的 `entities` 字段派生 `(event_id, entity_id, relation_type)` 三元组，按实体类型分配 `relation_type`（PERSON→subject/agent、LOCATION→location/venue、ORGANIZATION→organization/host 等），不足 1500 条时从已有事件随机追加更多实体关联。
4. **查询**：手工设计 50 个查询（15 单跳 + 20 双跳 + 15 三跳），每个查询的 `expected_event_ids` 由「查询实体 → 关联事件」反向计算得出，确保 ground truth 与 events/relations 一致。

固定随机种子 `20260721`，数据可复现。

## 文件结构

```
tests/fixtures/zh_multihop/
├── README.md          # 本文件（数据集说明）
├── entities.json      # 200 实体（11 类）
├── events.json        # 500 事件（含 entities 引用 + hop 标注）
├── relations.json     # 1500 关系（event_id ↔ entity_id + relation_type）
├── queries.json       # 50 查询（含 expected_event_ids + expected_hop）
└── _generate.py       # 数据生成脚本（手工实体 + 模板化事件）
```

## 使用方式

### 在 Rust 测试中加载

参考 `tests/zh_multihop_dataset_test.rs`：

```rust
#![forbid(unsafe_code)]

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct Entity {
    id: String,
    name: String,
    entity_type: String,
    // ... 其他字段
}

fn load_entities() -> Vec<Entity> {
    let json = include_str!("../fixtures/zh_multihop/entities.json");
    serde_json::from_str(json).expect("entities.json 解析失败")
}
```

### 12.3.2 / 12.3.3 后续用途

- **12.3.2 4 策略对比**：用 50 个查询分别跑 MULTI1 / MULTI2 / MULTI3 / MULTI_LLM 策略，对比 Recall@10 / latency_ms。
- **12.3.3 Recall@10 > 0.85 调优**：基于本数据集的 ground truth 评估各策略召回率，调优参数达到 spec 目标。

## 数据完整性保证

`_generate.py` 在生成时已做以下断言：
- 实体规模 = 200，事件规模 = 500，关系规模 = 1500，查询规模 = 50
- 所有 `events.entities` 引用的实体 ID 都在 `entities.json` 中存在
- 所有 `relations` 引用的 event_id / entity_id 都在对应文件中存在
- 所有 `queries.query_entities` 的实体名都在 `entities.json` 中存在
- 所有 `queries.expected_event_ids` 都在 `events.json` 中存在
- 查询跳数分布精确为 15/20/15

`tests/zh_multihop_dataset_test.rs` 在编译时再次验证上述完整性约束（6 个测试）。

## License

AGPL-3.0-only（与 SparkFox 主项目一致）
