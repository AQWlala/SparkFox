//! 信息热点追踪 — BaiLongma 热点跟踪清洁室重写
//!
//! 设计思路（仅借鉴功能，未拷贝代码）：
//! - 对每条消息内容做轻量提取，识别 URL / 代码 / 实体 / 关键词 / 主题。
//! - `HotspotTracker` 累积跨消息的热点信息：频次、首次出现、最近出现、关联消息。
//! - 提供 `top / by_type / related` 三种查询视角。
//!
//! 提取规则（清洁室自定）：
//! - `Url`: `https?://...` 形式的 URL
//! - `Code`: 反引号包裹的 `` `code` `` 片段
//! - `Entity`: ASCII 大写起头的多字符词（如 `SparkFox`、`Rust`）
//! - `Keyword`: 长度 >= 3 的 ASCII 词（去停用词）
//! - `Topic`: CJK 连续片段（长度 >= 2 个汉字）

#![forbid(unsafe_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 信息热点（关键词 / 实体 / 主题 等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hotspot {
    /// 唯一 ID
    pub id: String,
    /// 热点文本
    pub text: String,
    /// 热点类型
    pub hotspot_type: HotspotType,
    /// 出现频次
    pub frequency: u32,
    /// 首次出现时间
    pub first_seen: chrono::DateTime<chrono::Utc>,
    /// 最近出现时间
    pub last_seen: chrono::DateTime<chrono::Utc>,
    /// 关联消息 ID 列表（按出现顺序，去重）
    pub related_messages: Vec<String>,
    /// 附带元数据
    pub metadata: Option<serde_json::Value>,
}

/// 热点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HotspotType {
    /// 关键词
    Keyword,
    /// 命名实体
    Entity,
    /// 主题
    Topic,
    /// URL
    Url,
    /// 代码片段
    Code,
}

impl HotspotType {
    pub fn label(self) -> &'static str {
        match self {
            HotspotType::Keyword => "关键词",
            HotspotType::Entity => "实体",
            HotspotType::Topic => "主题",
            HotspotType::Url => "URL",
            HotspotType::Code => "代码",
        }
    }
}

/// 常见英文停用词（避免污染关键词统计）
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "her", "was", "one", "our",
    "out", "his", "has", "had", "how", "its", "who", "did", "yes", "let", "put", "say", "she",
    "too", "use", "this", "that", "with", "have", "from", "they", "will", "would", "there",
    "their", "what", "about", "which", "when", "your", "them", "then",
];

/// 热点追踪器
#[derive(Debug, Default)]
pub struct HotspotTracker {
    hotspots: HashMap<String, Hotspot>,
}

impl HotspotTracker {
    /// 创建空追踪器
    pub fn new() -> Self {
        Self {
            hotspots: HashMap::new(),
        }
    }

    /// 跟踪一条消息：从内容中提取所有热点并更新内部状态
    pub fn track(&mut self, message_id: &str, content: &str) {
        let now = chrono::Utc::now();
        for extracted in extract_hotspots(content) {
            let entry = self.hotspots.get_mut(&extracted.key);
            match entry {
                Some(h) => {
                    h.frequency = h.frequency.saturating_add(1);
                    h.last_seen = now;
                    if !h.related_messages.iter().any(|m| m == message_id) {
                        h.related_messages.push(message_id.to_string());
                    }
                }
                None => {
                    let hotspot = Hotspot {
                        id: format!("hot_{}", uuid::Uuid::new_v4().simple()),
                        text: extracted.text.clone(),
                        hotspot_type: extracted.hotspot_type,
                        frequency: 1,
                        first_seen: now,
                        last_seen: now,
                        related_messages: vec![message_id.to_string()],
                        metadata: None,
                    };
                    self.hotspots.insert(extracted.key, hotspot);
                }
            }
        }
    }

    /// 返回所有热点（无序）
    pub fn all(&self) -> Vec<&Hotspot> {
        self.hotspots.values().collect()
    }

    /// 按频次取前 N
    pub fn top(&self, n: usize) -> Vec<&Hotspot> {
        let mut all: Vec<&Hotspot> = self.hotspots.values().collect();
        all.sort_by(|a, b| {
            b.frequency
                .cmp(&a.frequency)
                .then_with(|| a.text.cmp(&b.text))
        });
        all.truncate(n);
        all
    }

    /// 按类型过滤
    pub fn by_type(&self, hotspot_type: HotspotType) -> Vec<&Hotspot> {
        self.hotspots
            .values()
            .filter(|h| h.hotspot_type == hotspot_type)
            .collect()
    }

    /// 返回与指定消息相关的热点
    pub fn related(&self, message_id: &str) -> Vec<&Hotspot> {
        self.hotspots
            .values()
            .filter(|h| h.related_messages.iter().any(|m| m == message_id))
            .collect()
    }

    /// 当前热点总数
    pub fn len(&self) -> usize {
        self.hotspots.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.hotspots.is_empty()
    }
}

/// 内部提取结果
struct Extracted {
    /// 用作去重键（text + type）
    key: String,
    text: String,
    hotspot_type: HotspotType,
}

/// 从内容中提取所有热点
fn extract_hotspots(content: &str) -> Vec<Extracted> {
    let mut out = Vec::new();
    let mut seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();

    let push = |out: &mut Vec<Extracted>, seen: &mut std::collections::HashSet<String>, text: String, ty: HotspotType| {
        if text.is_empty() {
            return;
        }
        let key = make_key(&text, ty);
        if seen.insert(key.clone()) {
            out.push(Extracted {
                key,
                text,
                hotspot_type: ty,
            });
        }
    };

    // 1. URL
    for m in find_urls(content) {
        push(&mut out, &mut seen_keys, m, HotspotType::Url);
    }

    // 2. 反引号代码
    for m in find_inline_code(content) {
        push(&mut out, &mut seen_keys, m, HotspotType::Code);
    }

    // 3. ASCII 词（实体 / 关键词）
    for word in find_ascii_words(content) {
        if word.len() < 3 {
            continue;
        }
        let lower = word.to_ascii_lowercase();
        if STOPWORDS.contains(&lower.as_str()) {
            continue;
        }
        let starts_upper = word
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false);
        if starts_upper && word.chars().any(|c| c.is_ascii_lowercase()) {
            push(&mut out, &mut seen_keys, word, HotspotType::Entity);
        } else {
            push(&mut out, &mut seen_keys, lower, HotspotType::Keyword);
        }
    }

    // 4. CJK 主题
    for m in find_cjk_topics(content) {
        push(&mut out, &mut seen_keys, m, HotspotType::Topic);
    }

    out
}

fn make_key(text: &str, ty: HotspotType) -> String {
    format!("{}::{}", ty.label(), text)
}

/// 简易 URL 提取：`https?://` 直到下一个空白或中文标点
fn find_urls(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(rel) = content[i..].find("http://").or_else(|| content[i..].find("https://")) {
            let start = i + rel;
            let scheme_end = content[start..]
                .find("://")
                .map(|p| start + p + 3)
                .unwrap_or(start);
            let mut end = scheme_end;
            for (idx, ch) in content[scheme_end..].char_indices() {
                if ch.is_whitespace() || "，。；：、）」』\"'<>)".contains(ch) {
                    end = scheme_end + idx;
                    break;
                }
                end = scheme_end + idx + ch.len_utf8();
            }
            if end > start {
                out.push(content[start..end].to_string());
            }
            i = end.max(start + 1);
        } else {
            break;
        }
    }
    out
}

/// 提取反引号包裹的代码片段
fn find_inline_code(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = content.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '`' {
            // 寻找匹配的反引号
            let start = i + 1;
            let mut end_opt = None;
            while let Some((j, d)) = chars.next() {
                if d == '`' {
                    end_opt = Some(j);
                    break;
                }
            }
            if let Some(end) = end_opt {
                let text = &content[start..end];
                if !text.is_empty() {
                    out.push(text.to_string());
                }
            } else {
                break;
            }
        }
    }
    out
}

/// 提取 ASCII 词（字母 + 数字 + 下划线）
fn find_ascii_words(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in content.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            cur.push(ch);
        } else {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// 提取 CJK 连续片段（>= 2 个汉字）
fn find_cjk_topics(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in content.chars() {
        if is_cjk(ch) {
            cur.push(ch);
        } else {
            if cur.chars().count() >= 2 {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    if cur.chars().count() >= 2 {
        out.push(cur);
    }
    out
}

/// 判断是否为 CJK 表意文字
fn is_cjk(ch: char) -> bool {
    matches!(ch as u32,
        0x4E00..=0x9FFF   // CJK 统一表意文字
        | 0x3400..=0x4DBF // CJK 扩展 A
        | 0x3040..=0x309F // 平假名
        | 0x30A0..=0x30FF // 片假名
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_single_message_url_and_code() {
        let mut tracker = HotspotTracker::new();
        tracker.track(
            "msg_1",
            "看这个 https://example.com/page 的 `Vec<u8>` 实现",
        );
        let urls = tracker.by_type(HotspotType::Url);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].text, "https://example.com/page");
        let codes = tracker.by_type(HotspotType::Code);
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].text, "Vec<u8>");
    }

    #[test]
    fn track_updates_frequency_across_messages() {
        let mut tracker = HotspotTracker::new();
        tracker.track("m1", "SparkFox 是个项目");
        tracker.track("m2", "SparkFox 用 Rust 写的");
        let entities = tracker.by_type(HotspotType::Entity);
        // m2 同时引入了 "Rust" 作为 Entity
        assert_eq!(entities.len(), 2);
        let sf = entities.iter().find(|h| h.text == "SparkFox").unwrap();
        assert_eq!(sf.frequency, 2);
        assert_eq!(sf.related_messages.len(), 2);
        assert!(sf.related_messages.contains(&"m1".to_string()));
        assert!(sf.related_messages.contains(&"m2".to_string()));
    }

    #[test]
    fn top_returns_sorted_by_frequency() {
        let mut tracker = HotspotTracker::new();
        // 同一条消息内重复只计一次（去重设计），故用多条消息累计频次
        tracker.track("m1", "alpha");
        tracker.track("m2", "alpha");
        tracker.track("m3", "alpha");
        tracker.track("m4", "beta");
        tracker.track("m5", "beta");
        tracker.track("m6", "gamma");
        let top = tracker.top(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].text, "alpha");
        assert_eq!(top[0].frequency, 3);
        assert_eq!(top[1].text, "beta");
        assert_eq!(top[1].frequency, 2);
    }

    #[test]
    fn by_type_filters_correctly() {
        let mut tracker = HotspotTracker::new();
        tracker.track("m1", "Rust 编程语言很有趣");
        tracker.track("m2", "Rust 编程语言很强大");
        let topics = tracker.by_type(HotspotType::Topic);
        assert!(topics.iter().any(|h| h.text.contains("编程语言")));
        let entities = tracker.by_type(HotspotType::Entity);
        assert!(entities.iter().any(|h| h.text == "Rust"));
    }

    #[test]
    fn related_returns_only_matching_messages() {
        let mut tracker = HotspotTracker::new();
        tracker.track("m1", "SparkFox 是 AI 应用");
        tracker.track("m2", "今天天气不错");
        let m1_related = tracker.related("m1");
        assert!(m1_related.iter().any(|h| h.text == "SparkFox"));
        let m2_related = tracker.related("m2");
        assert!(m2_related.iter().any(|h| h.text.contains("天气")));
    }

    #[test]
    fn dedup_within_message() {
        let mut tracker = HotspotTracker::new();
        tracker.track("m1", "SparkFox SparkFox SparkFox");
        let entities = tracker.by_type(HotspotType::Entity);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].frequency, 1, "同一条消息内重复应只计一次");
    }

    #[test]
    fn empty_content_no_hotspots() {
        let mut tracker = HotspotTracker::new();
        tracker.track("m1", "");
        assert!(tracker.is_empty());
    }

    #[test]
    fn hotspot_serialization() {
        let h = Hotspot {
            id: "hot_1".into(),
            text: "测试".into(),
            hotspot_type: HotspotType::Topic,
            frequency: 3,
            first_seen: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
            related_messages: vec!["m1".into()],
            metadata: None,
        };
        let json = serde_json::to_string(&h).unwrap();
        let back: Hotspot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.text, h.text);
        assert_eq!(back.hotspot_type, h.hotspot_type);
        assert_eq!(back.frequency, h.frequency);
    }
}
