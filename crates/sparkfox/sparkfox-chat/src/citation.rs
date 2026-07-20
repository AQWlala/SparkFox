//! 引用追踪 — BaiLongma 引用清洁室重写
//!
//! 设计思路（仅借鉴功能，未拷贝代码）：
//! - 每条 AI 回复可附带一组 `Citation`，指向知识库的具体文档 / 分块。
//! - `CitationSet` 聚合一组引用，支持按来源过滤、按相关度排序。
//! - `inject_marks` 在原文指定偏移处插入 `[1] [2]` 编号标记，便于前端高亮。

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// 消息中的引用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// 引用唯一 ID
    pub id: String,
    /// 知识库文档 ID
    pub source_id: String,
    /// 文档分块 ID
    pub chunk_id: String,
    /// 页码（可选）
    pub page: Option<usize>,
    /// 在消息中的起始字节偏移
    pub start_offset: usize,
    /// 在消息中的结束字节偏移
    pub end_offset: usize,
    /// 相关度得分（0.0 - 1.0，越高越相关）
    pub score: f32,
    /// 引用文本预览
    pub preview: String,
}

/// 引用集合
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CitationSet {
    /// 引用列表（按添加顺序）
    pub citations: Vec<Citation>,
}

impl CitationSet {
    /// 创建空集合
    pub fn new() -> Self {
        Self {
            citations: Vec::new(),
        }
    }

    /// 添加一条引用
    pub fn add(&mut self, citation: Citation) {
        self.citations.push(citation);
    }

    /// 按来源文档 ID 过滤
    pub fn by_source(&self, source_id: &str) -> Vec<&Citation> {
        self.citations
            .iter()
            .filter(|c| c.source_id == source_id)
            .collect()
    }

    /// 按相关度降序取前 N
    pub fn top(&self, n: usize) -> Vec<&Citation> {
        let mut all: Vec<&Citation> = self.citations.iter().collect();
        all.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        all.truncate(n);
        all
    }

    /// 在消息内容中注入 `[1] [2]` 编号标记
    ///
    /// 规则（清洁室自定）：
    /// - 引用按 `start_offset` 升序编号 1, 2, 3 ...
    /// - 在每条引用的 `end_offset` 处插入 `[N]` 标记
    /// - 偏移按降序插入，避免后插入影响前面的位置
    /// - 越界偏移会被自动夹紧到内容长度
    pub fn inject_marks(&self, content: &str) -> String {
        if self.citations.is_empty() {
            return content.to_string();
        }

        // 按 start_offset 升序分配编号
        let mut indexed: Vec<(usize, &Citation)> =
            self.citations.iter().map(|c| (c.start_offset, c)).collect();
        indexed.sort_by_key(|(start, _)| *start);

        let mut marks: Vec<(usize, String)> = Vec::new();
        for (i, (_, c)) in indexed.iter().enumerate() {
            let n = i + 1;
            let pos = c.end_offset.min(content.len());
            marks.push((pos, format!("[{}]", n)));
        }

        // 按偏移降序插入
        marks.sort_by(|a, b| b.0.cmp(&a.0));
        let mut out = content.to_string();
        for (pos, mark) in marks {
            let pos = pos.min(out.len());
            out.insert_str(pos, &mark);
        }
        out
    }

    /// 当前引用数量
    pub fn len(&self) -> usize {
        self.citations.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.citations.is_empty()
    }
}

impl Citation {
    /// 创建一个最小字段的引用（便于测试 / 快速构造）
    pub fn new(source_id: impl Into<String>, chunk_id: impl Into<String>, score: f32) -> Self {
        Self {
            id: format!("cit_{}", uuid::Uuid::new_v4().simple()),
            source_id: source_id.into(),
            chunk_id: chunk_id.into(),
            page: None,
            start_offset: 0,
            end_offset: 0,
            score,
            preview: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_citation(source: &str, start: usize, end: usize, score: f32) -> Citation {
        Citation {
            id: format!("cit_{}", source),
            source_id: source.into(),
            chunk_id: format!("chunk_{}", source),
            page: None,
            start_offset: start,
            end_offset: end,
            score,
            preview: "预览".into(),
        }
    }

    #[test]
    fn add_and_count() {
        let mut set = CitationSet::new();
        assert!(set.is_empty());
        set.add(make_citation("doc1", 0, 5, 0.9));
        set.add(make_citation("doc2", 6, 10, 0.7));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn by_source_filters() {
        let mut set = CitationSet::new();
        set.add(make_citation("doc1", 0, 5, 0.9));
        set.add(make_citation("doc2", 6, 10, 0.7));
        set.add(make_citation("doc1", 11, 15, 0.5));
        let doc1 = set.by_source("doc1");
        assert_eq!(doc1.len(), 2);
        let doc2 = set.by_source("doc2");
        assert_eq!(doc2.len(), 1);
        assert_eq!(set.by_source("doc3").len(), 0);
    }

    #[test]
    fn top_sorts_by_score_desc() {
        let mut set = CitationSet::new();
        set.add(make_citation("a", 0, 1, 0.3));
        set.add(make_citation("b", 2, 3, 0.9));
        set.add(make_citation("c", 4, 5, 0.6));
        let top = set.top(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].source_id, "b");
        assert!((top[0].score - 0.9).abs() < f32::EPSILON);
        assert_eq!(top[1].source_id, "c");
        assert!((top[1].score - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn inject_marks_inserts_in_order() {
        let mut set = CitationSet::new();
        // 引用按 start_offset 升序编号
        set.add(Citation {
            id: "c2".into(),
            source_id: "s2".into(),
            chunk_id: "k2".into(),
            page: None,
            start_offset: 20,
            end_offset: 25,
            score: 0.5,
            preview: "".into(),
        });
        set.add(Citation {
            id: "c1".into(),
            source_id: "s1".into(),
            chunk_id: "k1".into(),
            page: None,
            start_offset: 0,
            end_offset: 5,
            score: 0.9,
            preview: "".into(),
        });
        let content = "Hello world, this is a test message.";
        // 字节索引：
        //   H=0 e=1 l=2 l=3 o=4 ' '=5 w=6 ... t=23 e=24 s=25 t=26 ' '=27
        // 引用 1（start=0,end=5）→ 在字节 5 处插入 "[1]"（"Hello" 之后）
        // 引用 2（start=20,end=25）→ 在字节 25 处插入 "[2]"（"te" 之后、"st" 之前）
        let marked = set.inject_marks(content);
        assert_eq!(marked, "Hello[1] world, this is a te[2]st message.");
    }

    #[test]
    fn inject_marks_empty_set_returns_original() {
        let set = CitationSet::new();
        let content = "hello";
        assert_eq!(set.inject_marks(content), content);
    }

    #[test]
    fn inject_marks_clamps_out_of_range_offset() {
        let mut set = CitationSet::new();
        set.add(Citation {
            id: "c1".into(),
            source_id: "s1".into(),
            chunk_id: "k1".into(),
            page: None,
            start_offset: 0,
            end_offset: 9999,
            score: 1.0,
            preview: "".into(),
        });
        let marked = set.inject_marks("hi");
        assert_eq!(marked, "hi[1]");
    }

    #[test]
    fn citation_set_serialization() {
        let mut set = CitationSet::new();
        set.add(make_citation("doc1", 0, 5, 0.9));
        let json = serde_json::to_string(&set).unwrap();
        let back: CitationSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.citations.len(), 1);
        assert_eq!(back.citations[0].source_id, "doc1");
    }

    #[test]
    fn citation_new_helper() {
        let c = Citation::new("src", "chk", 0.42);
        assert_eq!(c.source_id, "src");
        assert_eq!(c.chunk_id, "chk");
        assert!((c.score - 0.42).abs() < f32::EPSILON);
        assert!(c.id.starts_with("cit_"));
    }
}
