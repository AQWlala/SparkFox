/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * KnowledgeGraphView — 知识图谱视图入口（spec §三 11.3.1 / 11.3.2 / 第 13 波并行 sub-step C）
 *
 * 本文件提供「知识图谱」页面的入口：
 *   - 顶部：标题「知识图谱」+ 返回按钮（返回知识库详情页）
 *   - PoC 提示卡片：标注「图谱渲染待 11.3.2 实现」（实际 @xyflow/react 渲染推迟到 11.4.1）
 *   - 主体：GraphCanvas 画布组件（11 类着色 + 图例 + SVG 简单节点/边展示）
 *
 * 范围说明：spec §三 11.3.2 原本包含 @xyflow/react 实际渲染，但 @xyflow/react 依赖
 * 较重且需 v12 升级，本波仅实施「11 类着色常量 + 图例组件 + 简单 SVG 节点展示」，
 * 实际 @xyflow/react 渲染推迟到 11.4.1。这样本波可独立完成无外部依赖冲突。
 *
 * PoC 数据：使用 useState mock 5 个节点 + 4 条边，覆盖 5 种实体类型
 *   （PERSON / LOCATION / ORGANIZATION / TIME / EVENT）。
 *
 * 路由：/kb/:id/graph → KnowledgeGraphView（kbId 从 useParams 获取）
 * 入口：KnowledgeDetailPage 顶部操作栏「查看知识图谱」按钮（Link）
 */

import React, { useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { Button, Card } from '@arco-design/web-react';
import { Left } from '@icon-park/react';
import GraphCanvas from './GraphCanvas';
import type { GraphEdge, GraphNode } from './types';
import styles from './styles.module.css';

/**
 * KnowledgeGraphView 主组件。
 *
 * 11.3.2 PoC 阶段：使用 useState 维护 mock 数据（5 节点 + 4 边），
 * 后续 11.4.1 阶段接入 @xyflow/react 后改用 useQuery / SWR 从后端拉取真实图谱数据。
 */
const KnowledgeGraphView: React.FC = () => {
  const navigate = useNavigate();
  // 从路由参数 /kb/:id/graph 提取知识库 ID
  const { id: kbId } = useParams<{ id: string }>();

  // ─── PoC mock 数据：5 个节点（覆盖 PERSON / LOCATION / ORGANIZATION / TIME / EVENT） ───
  const [nodes] = useState<GraphNode[]>([
    { id: 'n1', label: '张三', type: 'PERSON', x: 180, y: 160 },
    { id: 'n2', label: '北京', type: 'LOCATION', x: 420, y: 130 },
    { id: 'n3', label: '阿里', type: 'ORGANIZATION', x: 620, y: 200 },
    { id: 'n4', label: '2026-07-20', type: 'TIME', x: 280, y: 380 },
    { id: 'n5', label: '发布会', type: 'EVENT', x: 520, y: 420 },
  ]);

  // ─── PoC mock 数据：4 条边（event_entity_relation） ───
  const [edges] = useState<GraphEdge[]>([
    { source: 'n1', target: 'n2', label: '居住于' },
    { source: 'n1', target: 'n3', label: '就职于' },
    { source: 'n4', target: 'n5', label: '发生于' },
    { source: 'n3', target: 'n5', label: '主办' },
  ]);

  // 节点点击回调（PoC：仅 console.log，后续 11.4.1 接入抽屉/详情面板）
  const handleNodeClick = (nodeId: string) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] node clicked:', nodeId);
  };

  // 边点击回调（PoC：仅 console.log）
  const handleEdgeClick = (edge: GraphEdge) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] edge clicked:', edge);
  };

  return (
    <div className={styles.container}>
      {/* ─── 顶部栏：标题 + 返回按钮 ─── */}
      <div className={styles.header}>
        <h1 className={styles.title}>知识图谱</h1>
        <Button
          shape='round'
          icon={<Left theme='outline' size='14' />}
          onClick={() => navigate(`/knowledge/${kbId ?? ''}`)}
        >
          返回知识库
        </Button>
      </div>

      {/* ─── PoC 提示卡片：标注当前 11.3.2 阶段实现范围 ─── */}
      <Card className={styles.pocHint} bordered>
        <span>图谱渲染待 11.3.2 实现（PoC：11 类着色 + 图例 + SVG 简单展示；@xyflow/react 渲染待 11.4.1）</span>
      </Card>

      {/* ─── 主体：GraphCanvas 画布（11 类着色 + 图例） ─── */}
      <GraphCanvas
        nodes={nodes}
        edges={edges}
        onNodeClick={handleNodeClick}
        onEdgeClick={handleEdgeClick}
      />
    </div>
  );
};

export default KnowledgeGraphView;
