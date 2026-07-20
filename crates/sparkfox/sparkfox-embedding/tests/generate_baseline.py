#!/usr/bin/env python3
"""
PoC-3 Python baseline 生成脚本

生成 bge-small-zh-v1.5 的 Python sentence-transformers 嵌入基线，
用于与 Rust candle-transformers 实现做一致性比对（cosine > 0.99）。

运行方式：
    pip install sentence-transformers
    python tests/generate_baseline.py

输出：
    tests/baseline_small.json — 100 条 (text, embedding) 对
"""

import json
import os
import sys

from sentence_transformers import SentenceTransformer


def main():
    # 优先用本地预下载路径，避免依赖 HF 网络访问
    local_path = os.path.join(
        os.path.dirname(__file__), "..", "..", "..", "..", ".models", "BAAI_bge-small-zh-v1.5"
    )
    local_path = os.path.abspath(local_path)
    model_name = local_path if os.path.isdir(local_path) else "BAAI/bge-small-zh-v1.5"

    output_path = os.path.join(os.path.dirname(__file__), "baseline_small.json")

    print(f"加载模型: {model_name}")
    model = SentenceTransformer(model_name)

    # 100 条中文测试文本（覆盖短/长/技术/文学场景）
    texts = [
        f"测试文本 {i}" for i in range(50)
    ] + [
        "SparkFox 是一个桌面端 AI Agent 应用",
        "知识库 RAG 引擎支持向量检索和关键词检索",
        "6 层记忆架构 L0-L5 是 Pangu Nebula 的核心设计",
        "Tauri 2 进程内后端模式让 IPC 延迟低于 1ms",
        "automerge-rs 0.10 提供 CRDT 同步能力",
        "Double Ratchet 算法实现端到端加密",
        "sqlite-vec 是嵌入式向量库，无需外部服务",
        "candle-transformers 是 Hugging Face 官方 Rust 栈",
        "bge-small-zh-v1.5 模型大小 120MB，输出 512 维",
        "RRF 融合公式：score(d) = Σ 1/(k + rank_i(d))，k=60",
        "知识图谱使用 petgraph + SQLite 存储",
        "MDRM 5 维多跳遍历是 OpenAkita 的核心算法",
        "AGPL 清洁室流程确保合规性",
        "数据主权是 SparkFox 的核心理念",
        "前端采用 React 19.1 + Zustand + Arco Design",
        "NomiFun 的文件系统真源设计契合数据主权",
        "OpenAkita 的 Agent 菜单系统支持 22 字段配置",
        "BaiLongma 的对话展示采用流式气泡",
        "信息热点追踪覆盖微博/知乎/抖音/B站四平台",
        "3D 地球使用 Three.js + @react-three/fiber",
        "Tick 心跳机制监控 Agent 存活状态",
        "Scene Protocol 实现场景可序列化",
        "ThoughtStream 三区联动展示思考过程",
        "AgentDashboard 力导向图可视化",
        "TokenStats 6 周期 5 维度统计",
        "11 层安全栈覆盖数据主权全链路",
        "嵌入缓存策略：仅缓存查询嵌入，文档嵌入每次重建",
        "PoC-3 验收门槛：单条 < 50ms，1000 条 < 30s，cosine > 0.99",
        "知识库默认不同步，用户显式开启",
        "MCP Broker audit log 记录所有 search 调用",
        "PDF 解析使用 lopdf 纯 Rust 实现",
        "Word 解析使用 docx-rs",
        "Excel 解析使用 calamine",
        "OCR 使用 tesseract-rs（用户可选安装）",
        "CLIP 图片嵌入支持图文检索",
        "Rerank 使用 bge-reranker-v2-m3（560MB）",
        "文档分块策略：256 tokens + 50 重叠",
        "FTS5 关键词召回与向量召回通过 RRF 融合",
        "引用协议格式：[citation:kdoc_xxx:chunk_5:0:128]",
        "CitationChip 组件渲染三色标签",
        "知识图谱实体抽取使用 LLM function calling",
        "14 个 Rust crate 构成 SparkFox 后端",
        "8 个前端 Zustand store 管理状态",
        "6 个 View 提供用户界面",
        "v1.0.0 是单一版本发布，包含 50+ 任务",
        "10 周工期估算 101 人天",
        "6 阶段执行顺序 + 5 里程碑",
        "PoC-3 NO-GO 时退回 Python sidecar",
        "Kill Switch 机制保护项目进度",
        "sparkfox-embedding crate 是 PoC-3 载体",
        "sparkfox-knowledge crate 封装 RAG 引擎",
    ]

    print(f"生成 {len(texts)} 条嵌入...")
    vecs = model.encode(texts).tolist()
    data = list(zip(texts, vecs))

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False)

    print(f"✅ baseline 已生成: {output_path}")
    print(f"   样本数: {len(data)}")
    print(f"   维度: {len(vecs[0])}")
    print(f"   文件大小: {os.path.getsize(output_path)} bytes")


if __name__ == "__main__":
    try:
        main()
    except ImportError as e:
        print(f"❌ 缺少依赖: {e}", file=sys.stderr)
        print("请运行: pip install sentence-transformers", file=sys.stderr)
        sys.exit(1)
