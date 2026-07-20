/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * KnowledgeGraphView — 知识图谱视图入口（spec §三 11.3.1 / 11.3.2 / 11.3.3）
 *
 * 本文件提供「知识图谱」页面的入口：
 *   - 顶部：标题「知识图谱」+ 返回按钮（返回知识库详情页）
 *   - PoC 提示卡片：标注「图谱渲染待 11.3.2 实现」（实际 @xyflow/react 渲染推迟到 11.4.1）
 *   - 主体：GraphCanvas 画布组件（11 类着色 + 图例 + SVG 简单节点/边展示）
 *   - 抽屉：节点点击后打开 EntityEditDrawer（合并 / 拆分 / 重命名 3 操作，spec §三 11.3.3）
 *
 * 范围说明：spec §三 11.3.2 原本包含 @xyflow/react 实际渲染，但 @xyflow/react 依赖
 * 较重且需 v12 升级，本波仅实施「11 类着色常量 + 图例组件 + 简单 SVG 节点展示」，
 * 实际 @xyflow/react 渲染推迟到 11.4.1。这样本波可独立完成无外部依赖冲突。
 *
 * 范围说明：spec §三 11.3.3 原本包含「IPC 调用 + 持久化到 entity 表」，但本波仅实施
 * 前端 UI 部分（Drawer 组件 + 3 操作 tabs + PoC mock 回调），IPC 调用 + 持久化
 * 推迟到 11.4.x（与后端 entity 编辑命令同步实施）。
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
import EntityEditDrawer from './EntityEditDrawer';
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

  // ─── 11.3.3 EntityEditDrawer 抽屉状态 ───
  // 当前选中的实体（节点点击时设置；null 表示未选中）
  const [editingEntity, setEditingEntity] = useState<GraphNode | null>(null);
  // 抽屉可见性（节点点击时打开，onClose 时关闭）
  const [drawerVisible, setDrawerVisible] = useState<boolean>(false);

  // 节点点击回调（spec §三 11.3.3：打开 EntityEditDrawer 抽屉）
  const handleNodeClick = (nodeId: string) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] node clicked:', nodeId);
    // 根据 nodeId 在 nodes 中查找对应的 GraphNode
    const target = nodes.find((n) => n.id === nodeId) ?? null;
    setEditingEntity(target);
    setDrawerVisible(true);
  };

  // 边点击回调（PoC：仅 console.log）
  const handleEdgeClick = (edge: GraphEdge) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] edge clicked:', edge);
  };

  // 关闭抽屉回调：清空 editingEntity + 隐藏抽屉
  const handleDrawerClose = () => {
    setDrawerVisible(false);
    setEditingEntity(null);
  };

  // 合并操作回调（PoC：console.log + 关闭抽屉；11.4.x 接入 IPC 持久化到 entity 表）
  const handleMerge = (sourceId: string, targetId: string) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] merge entities:', sourceId, '->', targetId);
    setDrawerVisible(false);
    setEditingEntity(null);
  };

  // 拆分操作回调（PoC：console.log + 关闭抽屉；11.4.x 接入 IPC 持久化到 entity 表）
  const handleSplit = (sourceId: string, newNames: string[]) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] split entity:', sourceId, '->', newNames);
    setDrawerVisible(false);
    setEditingEntity(null);
  };

  // 重命名操作回调（PoC：console.log + 关闭抽屉；11.4.x 接入 IPC 持久化到 entity 表）
  const handleRename = (entityId: string, newName: string) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] rename entity:', entityId, '->', newName);
    setDrawerVisible(false);
    setEditingEntity(null);
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

      {/* ─── 11.3.3 EntityEditDrawer：节点点击打开抽屉（合并 / 拆分 / 重命名 3 操作） ─── */}
      <EntityEditDrawer
        visible={drawerVisible}
        entity={editingEntity}
        onClose={handleDrawerClose}
        onMerge={handleMerge}
        onSplit={handleSplit}
        onRename={handleRename}
      />
    </div>
  );
};

export default KnowledgeGraphView;
