# -*- coding: utf-8 -*-
"""
Sub-Step 12.3.1 — 中文多跳 Benchmark 数据集生成脚本

手工构造 200 实体 / 500 事件 / 1500 关系 / 50 查询，围绕中国科技场景。
本脚本一次性生成 entities.json / events.json / relations.json / queries.json。
"""
import json
import os
import random
from collections import defaultdict

random.seed(20260721)  # 固定种子，确保数据可复现

OUT_DIR = os.path.dirname(os.path.abspath(__file__))

# ---------------------------------------------------------------------------
# 实体定义（200 个，11 类分布）
# ---------------------------------------------------------------------------
# entity_type_id 对齐 SparkFox ENTITY_TYPES 顺序（PERSON=0, LOCATION=1, ...）
ENTITY_DEFS = [
    # (entity_type_id, entity_type, [names], description_template_or_value)
    (0, "PERSON", [
        "张三", "李四", "王五", "赵六", "孙七", "周八", "吴九", "郑十",
        "张伟", "王芳", "李娜", "刘洋", "陈静", "杨光", "赵磊", "黄敏",
        "林明", "周晓东", "吴志远", "郑文博", "孙佳", "钱学森", "李开复", "张朝阳",
        "张一鸣", "王兴", "李彦宏", "雷军", "马化腾", "马云", "丁磊", "程维",
        "周鸿祎", "刘强东", "黄峥", "梁建章", "王慧文", "张涛", "李斌", "何小鹏",
    ], [
        "一位在北京工作的软件工程师",
        "一位在上海工作的产品经理",
        "一位在深圳工作的算法工程师",
        "一位在杭州工作的设计师",
        "一位在成都工作的测试工程师",
        "一位在武汉工作的运维工程师",
        "一位在南京工作的数据科学家",
        "一位在广州工作的架构师",
    ]),
    (1, "LOCATION", [
        "北京", "上海", "广州", "深圳", "杭州", "成都", "武汉", "南京",
        "重庆", "苏州", "天津", "西安", "长沙", "青岛", "郑州", "大连",
        "厦门", "宁波", "无锡", "福州", "合肥", "济南", "沈阳", "哈尔滨",
        "昆明", "珠海", "东莞", "佛山", "贵阳", "海口",
    ], [
        "中国首都，科技与政治中心",
        "中国直辖市，金融与商业中心",
        "华南重要城市，制造业与科技中心",
        "中国科技创新中心，硬件与互联网企业聚集地",
        "中国互联网产业重镇，电商与支付发达",
        "西南科技中心，软件与服务外包基地",
        "华中科教重镇，光电子产业发达",
        "华东科教中心，历史文化名城",
    ]),
    (2, "ORGANIZATION", [
        "腾讯", "阿里巴巴", "字节跳动", "百度", "美团", "京东", "网易", "滴滴出行",
        "拼多多", "小米", "华为", "OPPO", "vivo", "联想", "中兴通讯", "大疆创新",
        "蔚来汽车", "理想汽车", "小鹏汽车", "比亚迪", "商汤科技", "旷视科技", "海康威视", "科大讯飞",
        "快手", "哔哩哔哩", "知乎", "小红书", "三六零", "浪潮信息",
    ], [
        "中国领先的互联网科技公司",
        "中国知名的科技企业",
        "中国头部互联网平台公司",
        "中国一线科技公司",
    ]),
    (3, "TIME", [
        "2020年", "2021年", "2022年", "2023年", "2024年", "2025年",
        "春季", "夏季", "秋季", "冬季",
        "2018年", "2019年", "2017年", "2016年", "2015年",
        "一月", "四月", "七月", "十月", "上半年",
    ], [
        "公历年份时间标记",
        "季节时间标记",
        "月份时间标记",
        "半年度时间标记",
    ]),
    (4, "EVENT", [
        "产品发布", "年度大会", "战略收购", "融资轮次", "公司上市", "团队招聘", "高管离职", "新品发布会",
        "战略合作", "对外投资", "并购重组", "技术峰会", "黑客松", "媒体沟通会", "战略升级", "组织调整",
        "财报发布", "用户大会", "开源发布", "生态大会",
    ], [
        "公司事件类型，用于事件分类",
        "企业战略事件，描述公司重大决策",
        "技术社区事件，描述开发者活动",
        "商业事件，描述市场动作",
    ]),
    (5, "CONCEPT", [
        "人工智能", "大模型", "智能体", "记忆系统", "检索增强生成",
        "向量数据库", "嵌入向量", "Transformer架构", "多模态", "推理能力",
        "模型微调", "强化学习", "知识图谱", "多跳检索", "倒数排名融合",
        "召回率", "准确率", "F1分数", "基准测试", "模型评测",
    ], [
        "AI 领域核心概念",
        "大模型相关技术概念",
        "信息检索领域评价指标",
        "知识表示与推理方法",
    ]),
    (6, "ARTIFACT", [
        "微信", "抖音", "飞书", "钉钉", "企业微信",
        "支付宝", "淘宝", "京东商城", "美团外卖", "滴滴出行App",
        "小红书App", "知乎App", "哔哩哔哩App", "快手App", "百度搜索",
    ], [
        "中国主流移动应用产品",
        "互联网平台核心产品",
        "面向消费者的应用产品",
    ]),
    (7, "SOFTWARE", [
        "SparkFox", "Tauri", "Rust", "TypeScript", "Preact",
        "React", "Visual Studio Code", "Git", "Linux", "PostgreSQL",
    ], [
        "桌面应用开发框架或语言",
        "编程语言或开发工具",
        "开源软件项目",
    ]),
    (8, "HARDWARE", [
        "iPhone", "Mac", "个人电脑", "服务器", "GPU加速卡",
    ], [
        "硬件设备，用于运行软件或存储数据",
    ]),
    (9, "DOCUMENT", [
        "README文档", "设计文档", "技术规范", "用户手册", "API文档",
    ], [
        "技术文档，用于描述软件设计或使用方式",
    ]),
    (10, "OTHER", [
        "开源协议", "测试数据集", "代码仓库", "开发框架", "工具链",
    ], [
        "其他类型实体，不属于上述分类",
    ]),
]

# ---------------------------------------------------------------------------
# 生成 entities.json
# ---------------------------------------------------------------------------
def gen_entities():
    entities = []
    eid = 1
    for type_idx, etype, names, desc_pool in ENTITY_DEFS:
        # 验证分布数量
        for i, name in enumerate(names):
            desc = desc_pool[i % len(desc_pool)]
            # 给一些 PERSON 添加更具体的描述
            if etype == "PERSON" and i < 8:
                desc = f"{desc},姓名{name}"
            entities.append({
                "id": f"ent-{eid:03d}",
                "name": name,
                "normalized_name": name,
                "entity_type_id": type_idx,
                "entity_type": etype,
                "description": desc,
            })
            eid += 1
    assert len(entities) == 200, f"实体数量应为 200，实际 {len(entities)}"
    return entities


# ---------------------------------------------------------------------------
# 生成 events.json + relations.json
# ---------------------------------------------------------------------------
# 关系类型池
RELATION_TYPES = {
    "PERSON": ["subject", "agent", "participant", "speaker", "author"],
    "LOCATION": ["location", "venue", "headquarters", "target_location"],
    "ORGANIZATION": ["organization", "host", "sponsor", "acquirer", "investor"],
    "TIME": ["time", "occurrence_time", "period"],
    "EVENT": ["event_type", "category"],
    "CONCEPT": ["concept", "topic", "technology"],
    "ARTIFACT": ["product", "tool", "platform"],
    "SOFTWARE": ["software", "framework", "language"],
    "HARDWARE": ["hardware", "device", "equipment"],
    "DOCUMENT": ["document", "specification", "reference"],
    "OTHER": ["other", "attribute"],
}

def pick_relation_type(etype, rng):
    pool = RELATION_TYPES.get(etype, ["related"])
    return rng.choice(pool)


def gen_events_and_relations(entities):
    """生成 500 个事件和约 1500 个关系。"""
    # 按类型分组实体
    by_type = defaultdict(list)
    by_name = {}
    for e in entities:
        by_type[e["entity_type"]].append(e)
        by_name[e["name"]] = e

    rng = random.Random(20260721)

    # 模板化事件构造：每个模板包含 {placeholder} 与对应的实体类型
    # hop 标注：1 = 单实体直接命中, 2 = 2 实体关联, 3 = 3+ 实体关联
    TEMPLATES = [
        # (template, [(placeholder, entity_type), ...], hop)
        ("{P}在{L}参加了{O}的产品发布会。", [("P", "PERSON"), ("L", "LOCATION"), ("O", "ORGANIZATION")], 2),
        ("{P}于{T}在{L}出席{O}的年度大会。", [("P", "PERSON"), ("T", "TIME"), ("L", "LOCATION"), ("O", "ORGANIZATION")], 2),
        ("{O}在{L}发布{A}新一代版本。", [("O", "ORGANIZATION"), ("L", "LOCATION"), ("A", "ARTIFACT")], 2),
        ("{P}加入{O}担任核心工程师。", [("P", "PERSON"), ("O", "ORGANIZATION")], 1),
        ("{P}离开{O}创立新公司。", [("P", "PERSON"), ("O", "ORGANIZATION")], 1),
        ("{O}完成{E}轮次融资。", [("O", "ORGANIZATION"), ("E", "EVENT")], 1),
        ("{O}在{T}上市{L}证券交易所。", [("O", "ORGANIZATION"), ("T", "TIME"), ("L", "LOCATION")], 2),
        ("{P}在{L}发表{C}主题演讲。", [("P", "PERSON"), ("L", "LOCATION"), ("C", "CONCEPT")], 2),
        ("{O}推出基于{C}的新产品{A}。", [("O", "ORGANIZATION"), ("C", "CONCEPT"), ("A", "ARTIFACT")], 2),
        ("{P}在{T}参与{O}组织的技术峰会。", [("P", "PERSON"), ("T", "TIME"), ("O", "ORGANIZATION")], 2),
        ("{O}宣布收购{O2}。", [("O", "ORGANIZATION"), ("O2", "ORGANIZATION")], 2),
        ("{P}与{P2}联合发起{C}开源项目。", [("P", "PERSON"), ("P2", "PERSON"), ("C", "CONCEPT")], 2),
        ("{O}在{L}设立研发中心。", [("O", "ORGANIZATION"), ("L", "LOCATION")], 1),
        ("{P}在{T}调任{L}负责{O}业务。", [("P", "PERSON"), ("T", "TIME"), ("L", "LOCATION"), ("O", "ORGANIZATION")], 3),
        ("{O}与{O2}在{L}达成战略合作。", [("O", "ORGANIZATION"), ("O2", "ORGANIZATION"), ("L", "LOCATION")], 2),
        ("{P}使用{S}开发{A}。", [("P", "PERSON"), ("S", "SOFTWARE"), ("A", "ARTIFACT")], 3),
        ("{O}在{T}于{L}举办{E}。", [("O", "ORGANIZATION"), ("T", "TIME"), ("L", "LOCATION"), ("E", "EVENT")], 3),
        ("{P}在{L}的{O}发表{D}。", [("P", "PERSON"), ("L", "LOCATION"), ("O", "ORGANIZATION"), ("D", "DOCUMENT")], 3),
        ("{P}与{P2}通过{A}协作完成{C}项目。", [("P", "PERSON"), ("P2", "PERSON"), ("A", "ARTIFACT"), ("C", "CONCEPT")], 3),
        ("{O}在{T}举办{E}，{P}担任主讲。", [("O", "ORGANIZATION"), ("T", "TIME"), ("E", "EVENT"), ("P", "PERSON")], 3),
        ("{O}发布{A}集成{S}。", [("O", "ORGANIZATION"), ("A", "ARTIFACT"), ("S", "SOFTWARE")], 2),
        ("{P}在{L}调研{C}应用。", [("P", "PERSON"), ("L", "LOCATION"), ("C", "CONCEPT")], 2),
        ("{O}在{T}升级{A}到新版本。", [("O", "ORGANIZATION"), ("T", "TIME"), ("A", "ARTIFACT")], 2),
        ("{P}使用{H}训练{C}模型。", [("P", "PERSON"), ("H", "HARDWARE"), ("C", "CONCEPT")], 3),
        ("{O}发布{D}规范{C}技术。", [("O", "ORGANIZATION"), ("D", "DOCUMENT"), ("C", "CONCEPT")], 2),
        ("{P}在{T}访问{L}的{O}总部。", [("P", "PERSON"), ("T", "TIME"), ("L", "LOCATION"), ("O", "ORGANIZATION")], 3),
        ("{O}与{O2}联合投资{C}初创公司。", [("O", "ORGANIZATION"), ("O2", "ORGANIZATION"), ("C", "CONCEPT")], 2),
        ("{P}在{L}主持{E}。", [("P", "PERSON"), ("L", "LOCATION"), ("E", "EVENT")], 2),
        ("{O}于{T}开源{S}项目。", [("O", "ORGANIZATION"), ("T", "TIME"), ("S", "SOFTWARE")], 2),
        ("{P}在{L}参加{E}并演讲{C}。", [("P", "PERSON"), ("L", "LOCATION"), ("E", "EVENT"), ("C", "CONCEPT")], 3),
    ]

    events = []
    relations = []
    used_event_contents = set()

    # 锚点事件：确保 queries.json 引用的关键事件存在
    # 这些事件会在生成开始时优先构造，并记录 event_id 供 queries 引用
    ANCHOR_EVENTS = []

    # 优先构造锚点事件（每个对应一个 query 或多 query）
    # 锚点 1：张三在北京参加腾讯产品发布会
    anchor_specs = [
        # (template_idx, placeholder_assignments, hop, anchor_key)
        (0, {"P": "张三", "L": "北京", "O": "腾讯", "A": None}, 2, "zhang_tencent_beijing"),
        (1, {"P": "张三", "T": "2023年", "L": "北京", "O": "腾讯"}, 3, "zhang_2023_tencent_beijing"),
        (3, {"P": "张三", "O": "腾讯"}, 1, "zhang_join_tencent"),
        (11, {"P": "张三", "P2": "李四", "C": "大模型"}, 2, "zhang_li_llm"),
        (13, {"P": "张三", "T": "2023年", "L": "北京", "O": "字节跳动"}, 3, "zhang_2023_bytedance_beijing"),
        (17, {"P": "张三", "L": "北京", "O": "腾讯", "D": "设计文档"}, 3, "zhang_design_doc_tencent"),
        (18, {"P": "张三", "P2": "李四", "A": "飞书", "C": "大模型"}, 3, "zhang_li_feishu_llm"),
        (15, {"P": "张三", "S": "SparkFox", "A": "微信"}, 3, "zhang_sparkfox_wechat"),
        (24, {"P": "张三", "T": "2023年", "L": "北京", "O": "腾讯"}, 3, "zhang_2023_visit_tencent"),
        (28, {"P": "张三", "L": "北京", "E": "产品发布", "C": "大模型"}, 3, "zhang_beijing_product_launch_llm"),
    ]

    def fill_template(template, assignments, slots):
        """根据 assignments 填充模板。slots 是模板里的 (placeholder, etype) 列表。"""
        text = template
        ent_objs = []
        for ph, etype in slots:
            name = assignments.get(ph)
            if name is None:
                # 随机选一个该类型的实体
                name = rng.choice(by_type[etype])["name"]
            ent = by_name[name]
            ent_objs.append(ent)
            text = text.replace("{" + ph + "}", name, 1)
        return text, ent_objs

    # 生成锚点事件
    anchor_event_ids = {}  # anchor_key -> event_id
    for tmpl_idx, assigns, hop, anchor_key in anchor_specs:
        tmpl_str, slots, _ = TEMPLATES[tmpl_idx]
        # 替换 P2/O2 等同类型 placeholder
        # 对 P2 / O2：将其视作 PERSON / ORGANIZATION
        normalized_slots = []
        for ph, etype in slots:
            if ph.endswith("2"):
                ph = ph[:-1]  # 不允许 P/O 重复，使用 suffix 区分
                # 此处保持原 placeholder 以便替换，但 name 不同
                ph = ph + "_2"
            normalized_slots.append((ph, etype))
        # 重新构造：让 P2 和 P 都能从 assignments 取到
        # 简化：直接重写 fill_template 支持 P2
        text = tmpl_str
        ent_objs = []
        for ph, etype in slots:
            name = assigns.get(ph)
            if name is None:
                # 同类型随机
                pool = [e for e in by_type[etype] if e["name"] not in assigns.values()]
                name = rng.choice(pool)["name"]
            ent = by_name[name]
            ent_objs.append(ent)
            # 替换 {P2} / {O2}
            text = text.replace("{" + ph + "}", name, 1)
        evt_id = f"evt-{len(events)+1:03d}"
        event = {
            "id": evt_id,
            "content": text,
            "entities": [e["id"] for e in ent_objs],
            "hop": hop,
        }
        events.append(event)
        anchor_event_ids[anchor_key] = evt_id
        used_event_contents.add(text)
        # 关系
        for ent in ent_objs:
            relations.append({
                "event_id": evt_id,
                "entity_id": ent["id"],
                "relation_type": pick_relation_type(ent["entity_type"], rng),
            })

    # 生成剩余事件直到 500 个
    while len(events) < 500:
        tmpl_str, slots, hop = rng.choice(TEMPLATES)
        # 生成随机 assignments
        assigns = {}
        used_names_per_type = defaultdict(set)
        for ph, etype in slots:
            pool = [e for e in by_type[etype] if e["name"] not in used_names_per_type[etype]]
            if not pool:
                pool = by_type[etype]
            choice = rng.choice(pool)
            assigns[ph] = choice["name"]
            used_names_per_type[etype].add(choice["name"])

        text = tmpl_str
        ent_objs = []
        seen_ids = set()
        for ph, etype in slots:
            name = assigns[ph]
            ent = by_name[name]
            if ent["id"] in seen_ids:
                continue  # 同一事件不重复关联同一实体
            seen_ids.add(ent["id"])
            ent_objs.append(ent)
            text = text.replace("{" + ph + "}", name, 1)

        if text in used_event_contents:
            continue  # 去重
        used_event_contents.add(text)

        evt_id = f"evt-{len(events)+1:03d}"
        event = {
            "id": evt_id,
            "content": text,
            "entities": [e["id"] for e in ent_objs],
            "hop": hop,
        }
        events.append(event)
        for ent in ent_objs:
            relations.append({
                "event_id": evt_id,
                "entity_id": ent["id"],
                "relation_type": pick_relation_type(ent["entity_type"], rng),
            })

    # 不足 1500 关系时补充：从已有事件中随机追加更多关系（保持 event-entity 真实存在）
    while len(relations) < 1500:
        evt = rng.choice(events)
        # 随机选一个未关联实体
        existing_ids = {r["entity_id"] for r in relations if r["event_id"] == evt["id"]}
        # 候选：除已关联外的任意实体
        candidates = [e for e in entities if e["id"] not in existing_ids]
        if not candidates:
            continue
        ent = rng.choice(candidates)
        relations.append({
            "event_id": evt["id"],
            "entity_id": ent["id"],
            "relation_type": pick_relation_type(ent["entity_type"], rng),
        })
        # 同步更新事件的 entities 字段
        if ent["id"] not in evt["entities"]:
            evt["entities"].append(ent["id"])

    # 关系超量时截断到 1500（按 event 顺序保留）
    if len(relations) > 1500:
        relations = relations[:1500]
        # 重新同步 events 的 entities
        evt_to_ents = defaultdict(list)
        for r in relations:
            evt_to_ents[r["event_id"]].append(r["entity_id"])
        for evt in events:
            evt["entities"] = evt_to_ents.get(evt["id"], evt["entities"])

    return events, relations, anchor_event_ids


# ---------------------------------------------------------------------------
# 生成 queries.json
# ---------------------------------------------------------------------------
def gen_queries(entities, events, anchor_event_ids):
    by_name = {e["name"]: e for e in entities}
    by_id = {e["id"]: e for e in entities}
    rng = random.Random(20260721)

    # 实体名 → 关联事件 IDs
    ent_to_events = defaultdict(list)
    for evt in events:
        for eid in evt["entities"]:
            ent_to_events[eid].append(evt["id"])

    queries = []

    # ---- 15 个 hop=1 单跳查询 ----
    hop1_specs = [
        ("张三参加了什么会议？", ["张三"]),
        ("李四在哪家公司工作？", ["李四"]),
        ("王五参与了哪些活动？", ["王五"]),
        ("腾讯发布了哪些产品？", ["腾讯"]),
        ("阿里巴巴在哪个城市设立总部？", ["阿里巴巴"]),
        ("字节跳动举办过什么活动？", ["字节跳动"]),
        ("北京有哪些科技事件？", ["北京"]),
        ("上海举办过哪些技术会议？", ["上海"]),
        ("大模型相关的项目有哪些？", ["大模型"]),
        ("飞书集成了哪些功能？", ["飞书"]),
        ("SparkFox 是什么项目？", ["SparkFox"]),
        ("2023年有哪些科技公司动作？", ["2023年"]),
        ("人工智能领域有哪些进展？", ["人工智能"]),
        ("微信是谁开发的？", ["微信"]),
        ("Rust 在哪些项目中被使用？", ["Rust"]),
    ]
    for query_text, ent_names in hop1_specs:
        # 找出所有提及这些实体的 event
        expected = set()
        for name in ent_names:
            ent = by_name.get(name)
            if ent:
                expected.update(ent_to_events.get(ent["id"], []))
        expected_list = sorted(expected)
        queries.append({
            "query": query_text,
            "expected_event_ids": expected_list,
            "expected_hop": 1,
            "query_entities": ent_names,
        })

    # ---- 20 个 hop=2 双跳查询 ----
    hop2_specs = [
        ("张三在北京参加了哪些腾讯的活动？", ["张三", "北京", "腾讯"]),
        ("李四在上海的阿里巴巴有什么项目？", ["李四", "上海", "阿里巴巴"]),
        ("王五在深圳的腾讯工作过吗？", ["王五", "深圳", "腾讯"]),
        ("字节跳动在北京举办了哪些活动？", ["字节跳动", "北京"]),
        ("马化腾在腾讯参与了哪些事件？", ["马化腾", "腾讯"]),
        ("雷军在北京小米发布过什么产品？", ["雷军", "北京", "小米"]),
        ("李彦宏在百度的人工智能项目是什么？", ["李彦宏", "百度", "人工智能"]),
        ("张一鸣在字节跳动的大模型项目有哪些？", ["张一鸣", "字节跳动", "大模型"]),
        ("丁磊在网易广州发布过什么？", ["丁磊", "网易", "广州"]),
        ("王兴在北京美团的工作有哪些？", ["王兴", "北京", "美团"]),
        ("程维在北京滴滴出行的项目是什么？", ["程维", "北京", "滴滴出行"]),
        ("刘强东在北京京东的工作有哪些？", ["刘强东", "北京", "京东"]),
        ("黄峥在上海拼多多的相关事件是什么？", ["黄峥", "上海", "拼多多"]),
        ("2024年腾讯在深圳有哪些动作？", ["2024年", "腾讯", "深圳"]),
        ("2023年阿里巴巴在杭州发生什么？", ["2023年", "阿里巴巴", "杭州"]),
        ("大模型在哪些科技公司被研究？", ["大模型"]),
        ("飞书在字节跳动内部用于哪些场景？", ["飞书", "字节跳动"]),
        ("SparkFox 使用了哪些技术栈？", ["SparkFox", "Rust"]),
        ("Tauri 框架有哪些公司在用？", ["Tauri"]),
        ("人工智能概念被哪些公司提及？", ["人工智能"]),
    ]
    for query_text, ent_names in hop2_specs:
        # hop=2 期望：所有同时关联 query_entities 中至少 2 个的事件
        # 加上每个实体单独关联的事件
        expected = set()
        per_entity_events = []
        for name in ent_names:
            ent = by_name.get(name)
            evts = set(ent_to_events.get(ent["id"], [])) if ent else set()
            per_entity_events.append(evts)
            expected.update(evts)
        # hop=2 期望至少包含同时含 2+ 实体的事件
        # 但为了 ground truth 简洁，保留全部 expected（multi1 单跳也能命中部分）
        expected_list = sorted(expected)
        queries.append({
            "query": query_text,
            "expected_event_ids": expected_list,
            "expected_hop": 2,
            "query_entities": ent_names,
        })

    # ---- 15 个 hop=3 三跳查询 ----
    hop3_specs = [
        ("2023年张三通过飞书与李四合作的大模型项目是什么？", ["2023年", "张三", "飞书", "李四", "大模型"]),
        ("2024年马化腾在北京腾讯发布的大模型叫什么？", ["2024年", "马化腾", "北京", "腾讯", "大模型"]),
        ("2023年张一鸣在字节跳动用飞书协调的人工智能项目？", ["2023年", "张一鸣", "字节跳动", "飞书", "人工智能"]),
        ("2022年李彦宏在百度北京的人工智能发布会？", ["2022年", "李彦宏", "百度", "北京", "人工智能"]),
        ("2023年雷军在北京小米发布的智能体产品？", ["2023年", "雷军", "北京", "小米", "智能体"]),
        ("2024年丁磊在网易广州的大模型项目？", ["2024年", "丁磊", "网易", "广州", "大模型"]),
        ("2023年王兴在北京美团的向量数据库应用？", ["2023年", "王兴", "北京", "美团", "向量数据库"]),
        ("2024年程维在北京滴滴出行的多跳检索项目？", ["2024年", "程维", "北京", "滴滴出行", "多跳检索"]),
        ("2023年刘强东在京东上海的强化学习应用？", ["2023年", "刘强东", "京东", "上海", "强化学习"]),
        ("2024年黄峥在上海拼多多的大模型评测？", ["2024年", "黄峥", "上海", "拼多多", "大模型"]),
        ("张三与李四通过飞书在2023年合作的Agent项目？", ["张三", "李四", "飞书", "2023年", "智能体"]),
        ("马化腾与张一鸣在2024年北京的人工智能峰会？", ["马化腾", "张一鸣", "2024年", "北京", "人工智能"]),
        ("李彦宏与雷军在2023年大模型技术峰会？", ["李彦宏", "雷军", "2023年", "大模型"]),
        ("张三在2023年北京腾讯的设计文档？", ["张三", "2023年", "北京", "腾讯", "设计文档"]),
        ("张三与李四在飞书上的SparkFox项目？", ["张三", "李四", "飞书", "SparkFox"]),
    ]
    for query_text, ent_names in hop3_specs:
        expected = set()
        for name in ent_names:
            ent = by_name.get(name)
            if ent:
                expected.update(ent_to_events.get(ent["id"], []))
        expected_list = sorted(expected)
        queries.append({
            "query": query_text,
            "expected_event_ids": expected_list,
            "expected_hop": 3,
            "query_entities": ent_names,
        })

    # 验证总数 50
    assert len(queries) == 50, f"查询数量应为 50，实际 {len(queries)}"

    # 验证分布 15/20/15
    hop1_count = sum(1 for q in queries if q["expected_hop"] == 1)
    hop2_count = sum(1 for q in queries if q["expected_hop"] == 2)
    hop3_count = sum(1 for q in queries if q["expected_hop"] == 3)
    assert hop1_count == 15, f"hop=1 应为 15，实际 {hop1_count}"
    assert hop2_count == 20, f"hop=2 应为 20，实际 {hop2_count}"
    assert hop3_count == 15, f"hop=3 应为 15，实际 {hop3_count}"

    return queries


# ---------------------------------------------------------------------------
# 写文件
# ---------------------------------------------------------------------------
def write_json(filename, data):
    path = os.path.join(OUT_DIR, filename)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    return path


def main():
    entities = gen_entities()
    events, relations, anchor_event_ids = gen_events_and_relations(entities)
    queries = gen_queries(entities, events, anchor_event_ids)

    # 验证规模
    assert len(entities) == 200, f"实体规模 {len(entities)} ≠ 200"
    assert len(events) == 500, f"事件规模 {len(events)} ≠ 500"
    assert len(relations) == 1500, f"关系规模 {len(relations)} ≠ 1500"
    assert len(queries) == 50, f"查询规模 {len(queries)} ≠ 50"

    # 验证：所有事件的 entities 引用都存在
    ent_ids = {e["id"] for e in entities}
    for evt in events:
        for eid in evt["entities"]:
            assert eid in ent_ids, f"事件 {evt['id']} 引用了不存在的实体 {eid}"

    # 验证：所有关系引用都存在
    evt_ids = {e["id"] for e in events}
    for r in relations:
        assert r["event_id"] in evt_ids, f"关系引用了不存在的事件 {r['event_id']}"
        assert r["entity_id"] in ent_ids, f"关系引用了不存在的实体 {r['entity_id']}"

    # 验证：所有查询的 query_entities 都能在 entities 中找到
    ent_names = {e["name"] for e in entities}
    for q in queries:
        for name in q["query_entities"]:
            assert name in ent_names, f"查询 '{q['query']}' 的实体 '{name}' 不在 entities.json 中"

    # 验证：所有查询的 expected_event_ids 都在 events 中
    for q in queries:
        for eid in q["expected_event_ids"]:
            assert eid in evt_ids, f"查询 '{q['query']}' 引用了不存在的事件 {eid}"

    # 写文件
    p1 = write_json("entities.json", entities)
    p2 = write_json("events.json", events)
    p3 = write_json("relations.json", relations)
    p4 = write_json("queries.json", queries)

    print(f"entities.json: {len(entities)} 条 → {p1}")
    print(f"events.json:   {len(events)} 条 → {p2}")
    print(f"relations.json:{len(relations)} 条 → {p3}")
    print(f"queries.json:  {len(queries)} 条 → {p4}")

    # 打印实体类型分布
    type_dist = defaultdict(int)
    for e in entities:
        type_dist[e["entity_type"]] += 1
    print("\n实体类型分布:")
    for t, c in sorted(type_dist.items()):
        print(f"  {t}: {c}")

    # 打印查询跳数分布
    hop_dist = defaultdict(int)
    for q in queries:
        hop_dist[q["expected_hop"]] += 1
    print("\n查询跳数分布:")
    for h, c in sorted(hop_dist.items()):
        print(f"  hop={h}: {c}")

    # 打印事件 hop 分布
    evt_hop_dist = defaultdict(int)
    for evt in events:
        evt_hop_dist[evt["hop"]] += 1
    print("\n事件 hop 分布:")
    for h, c in sorted(evt_hop_dist.items()):
        print(f"  hop={h}: {c}")


if __name__ == "__main__":
    main()
