//! Sub-Step 10.6.1 — jieba 降级 NER（R-06 决策）
//!
//! ## 背景与定位
//! 当 LLM structured output 全部重试失败时（R-06 决策），系统需要一条不依赖 LLM 的
//! 离线降级路径，仍能从中文文本中抽取出基本实体（人名 / 地名 / 机构 / 时间 / 数字）。
//! 本模块基于 [`jieba_rs`]（纯 Rust 中文分词）+ 自定义词典 + 正则规则实现该降级路径。
//!
//! ## 4 类实体识别策略
//! | 类型        | 来源                   | 示例                       |
//! |-------------|------------------------|----------------------------|
//! | PERSON      | 默认 + 自定义人名词典  | 张三 / 陈八                |
//! | ORGANIZATION| 默认 + 自定义机构词典  | 腾讯 / 阿里巴巴            |
//! | LOCATION    | 默认 + 自定义地名词典  | 北京 / 上海                |
//! | TIME        | 正则匹配（中文/ISO/相对词） | 2026年7月20日 / 2026-07-20 / 今天 |
//! | NUMBER      | 正则匹配（带量词优先 + 纯数字兜底） | 999 元 / 50% / 42 |
//!
//! ## 去重策略
//! 同一 `[start, end)` 区间不重复识别；正则匹配优先于词典匹配（避免「2026」被
//! 同时识别为 TIME 与 NUMBER）。
//!
//! ## 不在本模块范围
//! - LLM 调用与重试：在 `processor.rs`（Sub-Step 10.2.2）
//! - JSON 修复与多级降级链：在 `parser.rs`（Sub-Step 10.2.3）
//! - 本模块仅提供 `JiebaNer::extract` 原子能力，由 `parser.rs` 调用

use std::collections::HashSet;

use jieba_rs::Jieba;
use regex::Regex;

// ---------------------------------------------------------------------------
// 默认词典与正则规则（REFACTOR 阶段提取到 patterns / default_dict 模块）
// ---------------------------------------------------------------------------

/// 默认词典初始化（人名 / 机构 / 地名 / 相对时间词）
///
/// 所有默认词典集中在此模块，便于 10.6.2 Sub-Step 调优时统一扩展。
mod default_dict {
    /// 默认人名词典（5 个示例，覆盖常见测试场景）
    pub const PERSON: &[&str] = &["张三", "李四", "王五", "赵六", "钱七"];

    /// 默认机构词典（5 个示例）
    pub const ORGANIZATION: &[&str] = &["腾讯", "阿里巴巴", "字节跳动", "百度", "华为"];

    /// 默认地名词典（5 个示例）
    pub const LOCATION: &[&str] = &["北京", "上海", "广州", "深圳", "杭州"];

    /// 相对时间词（jieba 默认词典已含，但需作为 TIME 实体识别）
    ///
    /// 这些词会同时被 `patterns::TIME` 中的正则匹配，因此也注册到 jieba
    /// 以避免被合并到更长 token（如「今天天气」）。
    pub const RELATIVE_TIME: &[&str] = &[
        "今天", "明天", "昨天", "后天", "前天", "去年", "今年", "明年", "现在", "刚才",
    ];
}

/// 正则规则定义（时间 / 数字）
///
/// REFACTOR 阶段从 `extract` 方法中提取为独立模块，便于 10.6.2 Sub-Step 调优。
mod patterns {
    /// 时间正则（中文日期 / ISO 日期 / 相对时间词）
    ///
    /// 注意：相对时间词（今天 / 明天 等）走正则而非 jieba token 匹配，
    /// 因为 jieba 可能把「今天天气」整体切分为一个 token，导致「今天」无法独立识别。
    pub const TIME: &[&str] = &[
        // 中文日期：2026年7月20日 / 2026 年 7 月 20 日（允许任意空白）
        r"\d{4}\s*年\s*\d{1,2}\s*月\s*\d{1,2}\s*日",
        // ISO 日期：2026-07-20
        r"\d{4}-\d{1,2}-\d{1,2}",
        // 相对时间词（中文无 \b，直接匹配子串；「今天」作为前缀出现即应识别为 TIME）
        r"今天|明天|昨天|后天|前天|去年|今年|明年|现在|刚才",
    ];

    /// 数字正则（带量词优先，纯数字兜底）
    ///
    /// 顺序即优先级：带量词 → 百分比 → 纯数字。`extract` 方法按此顺序匹配，
    /// 一旦某区间命中即跳过后续正则，避免「999 元」与「999」重叠。
    pub const NUMBER: &[&str] = &[
        // 带量词：999 元 / 3 个 / 50% / 3.14 次
        r"\d+(?:\.\d+)?\s*(?:元|个|名|位|次|%|岁|年|月|天|件|台|套|本|页|字|斤|公斤|吨|米|公里|秒|分|时)",
        // 百分比（独立列出，因 % 已包含在上面但允许紧贴）
        r"\d+(?:\.\d+)?%",
        // 纯数字兜底
        r"\d+",
    ];
}

/// 识别出的实体提及
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityMention {
    /// 实体类型（PERSON / LOCATION / ORGANIZATION / TIME / NUMBER）
    pub entity_type: String,
    /// 实体文本
    pub text: String,
    /// 起始字节偏移（含）
    pub start: usize,
    /// 结束字节偏移（不含）
    pub end: usize,
}

/// jieba + 规则降级 NER（R-06 决策）
///
/// 当 LLM structured output 全部重试失败时，由 `parser.rs` 调用本结构体的
/// [`extract`](Self::extract) 方法获取降级实体识别结果。
///
/// 用法：
/// ```ignore
/// use sparkfox_knowledge::jieba_ner::JiebaNer;
/// let ner = JiebaNer::new();
/// let entities = ner.extract("张三昨天去了腾讯，花费 999 元");
/// ```
pub struct JiebaNer {
    /// jieba 分词器（含默认词典 + add_word 添加的自定义词）
    jieba: Jieba,
    /// 人名词典（用于 token → PERSON 实体）
    person_dict: HashSet<String>,
    /// 机构词典
    org_dict: HashSet<String>,
    /// 地名词典
    location_dict: HashSet<String>,
    /// 时间正则
    time_patterns: Vec<Regex>,
    /// 数字正则
    number_patterns: Vec<Regex>,
}

impl JiebaNer {
    /// 用默认词典构造（含 5 个人名 / 5 个机构 / 5 个地名 + 时间数字正则）
    pub fn new() -> Self {
        let empty: [&str; 0] = [];
        Self::with_custom_dict(empty, empty, empty)
    }

    /// 用自定义词典扩展默认词典构造
    ///
    /// 参数均为 `&str` 切片，便于测试用字面量传入。
    /// 自定义词会被 `jieba.add_word` 注册，确保 jieba 不会将其切碎。
    pub fn with_custom_dict(
        extra_persons: impl IntoIterator<Item = impl AsRef<str>>,
        extra_orgs: impl IntoIterator<Item = impl AsRef<str>>,
        extra_locations: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        let mut jieba = Jieba::new();

        let mut person_dict: HashSet<String> =
            default_dict::PERSON.iter().map(|s| s.to_string()).collect();
        let mut org_dict: HashSet<String> =
            default_dict::ORGANIZATION.iter().map(|s| s.to_string()).collect();
        let mut location_dict: HashSet<String> = default_dict::LOCATION
            .iter()
            .map(|s| s.to_string())
            .collect();

        // 注册自定义词到 jieba（确保分词不会切碎），同时加入对应 HashSet
        for w in extra_persons.into_iter() {
            let s = w.as_ref().to_string();
            jieba.add_word(&s, None, None);
            person_dict.insert(s);
        }
        for w in extra_orgs.into_iter() {
            let s = w.as_ref().to_string();
            jieba.add_word(&s, None, None);
            org_dict.insert(s);
        }
        for w in extra_locations.into_iter() {
            let s = w.as_ref().to_string();
            jieba.add_word(&s, None, None);
            location_dict.insert(s);
        }

        // 默认词典中的词也注册到 jieba，避免被默认词典切散
        // （含 RELATIVE_TIME，确保「今天 / 明天」不被合并到「今天天气」等长词）
        for w in default_dict::PERSON
            .iter()
            .chain(default_dict::ORGANIZATION.iter())
            .chain(default_dict::LOCATION.iter())
            .chain(default_dict::RELATIVE_TIME.iter())
        {
            jieba.add_word(w, None, None);
        }

        let time_patterns = patterns::TIME
            .iter()
            .map(|s| Regex::new(s).expect("时间正则编译失败"))
            .collect();
        let number_patterns = patterns::NUMBER
            .iter()
            .map(|s| Regex::new(s).expect("数字正则编译失败"))
            .collect();

        Self {
            jieba,
            person_dict,
            org_dict,
            location_dict,
            time_patterns,
            number_patterns,
        }
    }

    /// 仅分词，不做实体识别（用于 sanity check）
    pub fn segment<'a>(&self, text: &'a str) -> Vec<&'a str> {
        self.jieba.cut(text, false).into_iter().collect()
    }

    /// 从文本中抽取实体
    ///
    /// 流程：
    /// 1. 跑时间正则，记录命中的 `[start, end)` 区间
    /// 2. 跑数字正则（带量词优先），跳过与 TIME 重叠的区间
    /// 3. jieba 分词，对每个 token 查三类词典（PERSON / ORGANIZATION / LOCATION）
    /// 4. 跳过与正则命中区间重叠的 token（避免「2026」被识别为 NUMBER 而非 TIME）
    /// 5. 按 start 升序返回
    pub fn extract(&self, text: &str) -> Vec<EntityMention> {
        let mut mentions: Vec<EntityMention> = Vec::new();
        // 正则命中区间（字节偏移），用于去重
        let mut regex_spans: Vec<(usize, usize)> = Vec::new();

        // 1. 时间正则
        for pat in &self.time_patterns {
            for m in pat.find_iter(text) {
                let span = (m.start(), m.end());
                // 跳过与已识别 TIME 重叠的区间（中文日期优先于相对时间词）
                if overlaps_any(span, &regex_spans) {
                    continue;
                }
                mentions.push(EntityMention {
                    entity_type: "TIME".to_string(),
                    text: m.as_str().to_string(),
                    start: m.start(),
                    end: m.end(),
                });
                regex_spans.push(span);
            }
        }

        // 2. 数字正则（带量词优先，已在 patterns::NUMBER 中按优先级排序）
        //    对每个字符位置只允许一个 NUMBER 命中（避免「999 元」与「999」重叠）
        let mut number_spans: Vec<(usize, usize)> = Vec::new();
        for pat in &self.number_patterns {
            for m in pat.find_iter(text) {
                let span = (m.start(), m.end());
                // 跳过与已识别 NUMBER 或 TIME 重叠的区间
                if overlaps_any(span, &number_spans) || overlaps_any(span, &regex_spans) {
                    continue;
                }
                mentions.push(EntityMention {
                    entity_type: "NUMBER".to_string(),
                    text: m.as_str().to_string(),
                    start: m.start(),
                    end: m.end(),
                });
                number_spans.push(span);
                regex_spans.push(span);
            }
        }

        // 3. jieba 分词 + 词典匹配
        let words = self.jieba.cut(text, false);
        // jieba-rs 的 cut 返回 Vec<&str>，需要根据 &str 在原文中的位置重建字节偏移
        let mut cursor: usize = 0;
        for w in words {
            // 在原文 text 中从 cursor 开始找 w 的下一个出现位置
            let rel = match text[cursor..].find(w) {
                Some(r) => r,
                None => {
                    // 极端情况：jieba 切出的词在原文找不到（不应发生），跳过
                    cursor = cursor.min(text.len());
                    continue;
                }
            };
            let start = cursor + rel;
            let end = start + w.len();
            cursor = end;

            // 跳过空白 token
            if w.chars().all(char::is_whitespace) {
                continue;
            }

            // 跳过与正则命中区间重叠的 token
            if overlaps_any((start, end), &regex_spans) {
                continue;
            }

            // 词典匹配（优先级：PERSON > ORGANIZATION > LOCATION）
            // 相对时间词（今天 / 明天 等）已由正则识别，无需在此重复匹配
            if self.person_dict.contains(w) {
                mentions.push(EntityMention {
                    entity_type: "PERSON".to_string(),
                    text: w.to_string(),
                    start,
                    end,
                });
            } else if self.org_dict.contains(w) {
                mentions.push(EntityMention {
                    entity_type: "ORGANIZATION".to_string(),
                    text: w.to_string(),
                    start,
                    end,
                });
            } else if self.location_dict.contains(w) {
                mentions.push(EntityMention {
                    entity_type: "LOCATION".to_string(),
                    text: w.to_string(),
                    start,
                    end,
                });
            }
        }

        // 4. 按 start 升序稳定排序
        mentions.sort_by_key(|m| (m.start, m.end));
        mentions
    }
}

impl Default for JiebaNer {
    fn default() -> Self {
        Self::new()
    }
}

/// 判断 `(s, e)` 是否与 `spans` 中任一区间重叠
fn overlaps_any((s, e): (usize, usize), spans: &[(usize, usize)]) -> bool {
    spans.iter().any(|&(ps, pe)| s < pe && ps < e)
}
