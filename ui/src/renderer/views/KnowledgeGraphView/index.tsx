/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * KnowledgeGraphView — 知识图谱视图入口（spec §三 11.3.1 / 11.3.2 / 11.3.3 / 11.4.1 / 11.4.2）
 *
 * 本文件提供「知识图谱」页面的入口：
 *   - 顶部：标题「知识图谱」+ 返回按钮（返回知识库详情页）
 *   - PoC 提示卡片：标注「图谱渲染待 11.3.2 实现」（实际 @xyflow/react 渲染推迟到 11.4.1）
 *   - 渲染模式切换：SVG 模式（GraphCanvas）/ ReactFlow 模式（GraphFlow）（11.4.1 新增）
 *   - 主体（SVG 模式）：GraphCanvas 画布组件（11 类着色 + 图例 + SVG 简单节点/边展示）
 *   - 主体（ReactFlow 模式）：GraphFlow 组件（@xyflow/react v12 完整渲染，11.4.1 新增）
 *   - 抽屉：节点点击后打开 EntityEditDrawer（合并 / 拆分 / 重命名 3 操作，spec §三 11.3.3）
 *
 * 范围说明：spec §三 11.3.2 原本包含 @xyflow/react 实际渲染，但 @xyflow/react 依赖
 * 较重且需 v12 升级，第 13 波仅实施「11 类着色常量 + 图例组件 + 简单 SVG 节点展示」，
 * 实际 @xyflow/react 渲染推迟到 11.4.1（本波）。两种模式并存以便对比与回退。
 *
 * Sub-Step 11.4.2：handleMerge / handleSplit / handleRename 三个回调接入 Tauri IPC
 * （entity_merge / entity_split / entity_rename 命令），持久化到 entity 表 +
 * event_entity_relation 表。通过 isTauriRuntime() 环境检测实现降级：
 *   - Tauri 桌面环境：invoke() 调用后端命令 + 成功后刷新图谱数据
 *   - Web/开发环境（无 Tauri 运行时）：降级为 console.log 调试日志，不阻塞 UI
 *
 * Sub-Step 12.4.2：重命名流程升级为「预览影响 + 确认执行」两步：
 *   - handlePreviewRenameImpact：invoke preview_entity_rename_impact（纯 SELECT 不修改），
 *     返回 RenameImpactPreview { affected_events, affected_relations, affected_chunks }，
 *     通过 renameImpactPreview state 传给 EntityEditDrawer 渲染受影响范围面板
 *   - handleRename 改为 invoke execute_entity_rename（事务原子性 BEGIN/COMMIT/ROLLBACK，
 *     同步更新 entity.name + knowledge_event.content/summary/title）
 *   - 原 entity_rename 命令保留向后兼容（11.4.2 实现），12.4.2 起前端改用 execute_entity_rename
 *
 * PoC 数据：使用 useState mock 5 个节点 + 4 条边，覆盖 5 种实体类型
 *   （PERSON / LOCATION / ORGANIZATION / TIME / EVENT）。
 *
 * 路由：/kb/:id/graph → KnowledgeGraphView（kbId 从 useParams 获取）
 * 入口：KnowledgeDetailPage 顶部操作栏「查看知识图谱」按钮（Link）
 */

import React, { useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { Button, Card, Radio } from '@arco-design/web-react';
import { Left } from '@icon-park/react';
// Sub-Step 11.4.2：entity_merge / entity_split / entity_rename IPC 调用
import { invoke } from '@tauri-apps/api/core';
// Sub-Step 11.4.2：环境检测（Tauri 桌面 vs Web/开发环境）— 非 Tauri 环境降级 console.log
import { isTauriRuntime } from '@/common/adapter/tauriRuntime';
import GraphCanvas from './GraphCanvas';
import GraphFlow from './GraphFlow';
// 12.4.2：EntityEditDrawer 新增 RenameImpactPreview 类型导出 + impactPreview prop
import EntityEditDrawer from './EntityEditDrawer';
import type { RenameImpactPreview } from './EntityEditDrawer';
import MultiHopPathView from './MultiHopPathView';
import type { SearchHit } from './MultiHopPathView';
// 12.2.3：导入 Hyperedge 类型 + HyperedgeLayer 集成（在 GraphFlow 内叠加渲染）
import type { Hyperedge } from './HyperedgeLayer';
import type { GraphData } from './graphContract';
import type { GraphNode, GraphEdge } from './types';
import styles from './styles.module.css';

// Arco Radio.Group 别名（与项目其他模块写法保持一致）
const RadioGroup = Radio.Group;

/**
 * 渲染模式（spec §三 11.4.1）：
 *   - 'svg'  使用 GraphCanvas（SVG 简单渲染，第 13 波实现）
 *   - 'flow' 使用 GraphFlow（@xyflow/react v12 完整渲染，11.4.1 实现）
 */
type RenderMode = 'svg' | 'flow';

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

  // ─── 12.4.2 重命名影响预览状态 ───
  // 受影响范围数据（preview_entity_rename_impact 返回；null 表示未预览或预览失败）
  // 通过 impactPreview prop 传给 EntityEditDrawer 渲染受影响事件/关系/文本块面板
  const [renameImpactPreview, setRenameImpactPreview] =
    useState<RenameImpactPreview | null>(null);

  // ─── 11.4.1 渲染模式切换状态（SVG / ReactFlow） ───
  // 默认 'svg' 保持与 11.3.2 阶段一致的初始行为，用户可手动切换到 'flow' 模式
  const [renderMode, setRenderMode] = useState<RenderMode>('svg');

  // ─── 11.5.1 多跳路径视图状态 ───
  // 当前选中的 SearchHit（GraphFlow 节点点击时由 mockHitsByNodeId 查找）
  const [selectedHit, setSelectedHit] = useState<SearchHit | null>(null);
  // MultiHopPathView 可见性（默认 true 显示，onClose 后隐藏）
  const [showPathView, setShowPathView] = useState<boolean>(true);

  // ─── 12.2.3 超边状态 ───
  // 所有超边列表（mock 数据，对应后端 detect_from_relations 全图检测）
  // 真实数据由 12.2.1/12.2.2 后端 HyperedgeDetector + detect_from_relations 提供
  const [hyperedges] = useState<Hyperedge[]>([
    {
      id: 'he_mock_001',
      member_events: ['evt-1', 'evt-2', 'evt-3'],
      member_entities: ['n1', 'n2', 'n3'],
    },
    {
      id: 'he_mock_002',
      member_events: ['evt-2', 'evt-4', 'evt-5'],
      member_entities: ['n2', 'n4', 'n5'],
    },
  ]);
  // 激活的超边 ID 列表（来自 activate_local_hyperedges；非 Tauri 环境 mock 派生）
  // 12.2.4 阶段：Tauri 环境改为 invoke('activate_local_hyperedges') 获取真实数据
  const [activatedHyperedgeIds, setActivatedHyperedgeIds] = useState<string[]>([]);

  // ─── 11.4.1 GraphFlow 数据契约：从 SVG mock 数据转换为 GraphData DTO ───
  // 两种模式共享同一份 mock 数据，仅渲染方式不同，便于对比与切换
  const graphData: GraphData = useMemo(
    () => ({
      nodes: nodes.map((n) => ({
        id: n.id,
        label: n.label,
        // GraphNode.type → GraphNodeDTO.entity_type（字段名对齐后端契约）
        entity_type: n.type,
      })),
      edges: edges.map((e, idx) => ({
        // SVG GraphEdge 没有 id 字段，用 index 生成稳定 id
        id: `e-${idx}`,
        source: e.source,
        target: e.target,
        label: e.label,
      })),
      meta: {
        total_nodes: nodes.length,
        total_edges: edges.length,
        truncated: false,
      },
    }),
    [nodes, edges]
  );

  // ─── 11.5.1 PoC mock 数据：每个节点对应的多跳检索命中（SearchHit） ───
  // 真实数据由 11.4.2 IPC + 11.6.1 hnswlib-rs 后端检索填充；PoC 阶段使用 mock
  // 数据驱动 MultiHopPathView 渲染，验证 hop1 → hop2 → hop3 路径可视化效果
  const mockHitsByNodeId: Record<string, SearchHit> = useMemo(
    () => ({
      // n1 张三（PERSON）：单跳命中（hop1 蓝色）
      n1: {
        event_id: 'evt-1',
        score: 0.92,
        hop: 1,
        via_entities: [
          { entity_id: 'n1', entity_type: 'PERSON', text: '张三' },
        ],
        chunk_id: null,
      },
      // n2 北京（LOCATION）：二跳命中（hop2 黄色），路径 n1 → n2
      n2: {
        event_id: 'evt-2',
        score: 0.85,
        hop: 2,
        via_entities: [
          { entity_id: 'n1', entity_type: 'PERSON', text: '张三' },
          { entity_id: 'n2', entity_type: 'LOCATION', text: '北京' },
        ],
        chunk_id: null,
      },
      // n3 阿里（ORGANIZATION）：三跳命中（hop3 灰色），路径 n1 → n2 → n3
      n3: {
        event_id: 'evt-3',
        score: 0.78,
        hop: 3,
        via_entities: [
          { entity_id: 'n1', entity_type: 'PERSON', text: '张三' },
          { entity_id: 'n2', entity_type: 'LOCATION', text: '北京' },
          { entity_id: 'n3', entity_type: 'ORGANIZATION', text: '阿里' },
        ],
        chunk_id: null,
      },
      // n4 2026-07-20（TIME）：单跳命中（hop1 蓝色）
      n4: {
        event_id: 'evt-4',
        score: 0.66,
        hop: 1,
        via_entities: [
          { entity_id: 'n4', entity_type: 'TIME', text: '2026-07-20' },
        ],
        chunk_id: null,
      },
      // n5 发布会（EVENT）：二跳命中（hop2 黄色），路径 n4 → n5
      n5: {
        event_id: 'evt-5',
        score: 0.71,
        hop: 2,
        via_entities: [
          { entity_id: 'n4', entity_type: 'TIME', text: '2026-07-20' },
          { entity_id: 'n5', entity_type: 'EVENT', text: '发布会' },
        ],
        chunk_id: null,
      },
    }),
    []
  );

  // 节点点击回调（spec §三 11.3.3：打开 EntityEditDrawer 抽屉）
  // 两种渲染模式共享此回调：SVG 模式由 GraphCanvas 触发，ReactFlow 模式由 GraphFlow 触发
  // 11.5.1 扩展：同时联动 selectedHit，刷新 MultiHopPathView 显示
  // 12.2.3 扩展：同时联动 activate_local_hyperedges，高亮与该节点相关的超边
  const handleNodeClick = (nodeId: string) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] node clicked:', nodeId);
    // 根据 nodeId 在 nodes 中查找对应的 GraphNode
    const target = nodes.find((n) => n.id === nodeId) ?? null;
    setEditingEntity(target);
    setDrawerVisible(true);
    // 11.5.1：根据 nodeId 在 mockHitsByNodeId 中查找对应的 SearchHit
    // 找到则更新 selectedHit 并显示 MultiHopPathView
    const hit = mockHitsByNodeId[nodeId] ?? null;
    setSelectedHit(hit);
    if (hit) setShowPathView(true);
    // 12.2.3：激活与该节点相关的超边（queryEntities = [nodeId]）
    // Tauri 环境：invoke('activate_local_hyperedges', { queryEntities: [nodeId] })
    // 非 Tauri 环境：mock 派生（hyperedges 中含该 nodeId 的全部高亮）
    void activateHyperedgesForQuery([nodeId]);
  };

  // 12.2.3：超边激活逻辑（query 命中时高亮激活的超边）
  // - Tauri 桌面环境：invoke activate_local_hyperedges 调用后端 12.2.2 实现
  // - Web/开发环境：从 mock hyperedges 中过滤 member_entities ∩ queryEntities 非空的超边
  //   （mock 实现与后端 activate_local_hyperedges 算法一致，便于前端独立验证可视化）
  const activateHyperedgesForQuery = async (queryEntities: string[]) => {
    if (queryEntities.length === 0) {
      setActivatedHyperedgeIds([]);
      return;
    }
    if (isTauriRuntime()) {
      try {
        // invoke 返回 unknown，需 cast 为 Hyperedge[]
        // 字段名 id/member_events/member_entities 保持 snake_case（Rust serde 默认行为）
        const activated = (await invoke('activate_local_hyperedges', {
          queryEntities,
        })) as Hyperedge[];
        setActivatedHyperedgeIds(activated.map((he) => he.id));
      } catch (err) {
        // eslint-disable-next-line no-console
        console.error(
          '[KnowledgeGraphView] activate_local_hyperedges IPC failed:',
          err
        );
        setActivatedHyperedgeIds([]);
      }
    } else {
      // 非 Tauri 环境：mock 派生（与后端 activate_local_hyperedges 算法一致）
      const querySet = new Set(queryEntities);
      const activated = hyperedges
        .filter((he) => he.member_entities.some((ent) => querySet.has(ent)))
        .map((he) => he.id);
      setActivatedHyperedgeIds(activated);
    }
  };

  // 12.2.3：超边点击回调（PoC：仅 console.log，12.2.4+ 阶段可打开详情面板）
  const handleHyperedgeClick = (hyperedge: Hyperedge) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] hyperedge clicked:', hyperedge.id);
  };

  // 12.2.3：派生 queryEntities（从 selectedHit 提取，传给 GraphFlow → HyperedgeLayer）
  // 用于在 HyperedgeLayer 中保留查询上下文，便于未来扩展高亮逻辑
  const queryEntities = useMemo(
    () =>
      selectedHit
        ? selectedHit.via_entities.map((e) => e.entity_id)
        : [],
    [selectedHit]
  );

  // 11.5.1：via_entities Tag 点击回调，跳转到对应实体节点
  // PoC 阶段：复用 handleNodeClick 实现「点击 via_entity Tag 联动图谱」效果
  const handleEntityClick = (entityId: string) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] via_entity clicked:', entityId);
    handleNodeClick(entityId);
  };

  // 11.5.1：MultiHopPathView 关闭回调
  const handlePathViewClose = () => {
    setShowPathView(false);
  };

  // 边点击回调（PoC：仅 console.log）
  // SVG 模式直接传入 edge 对象；ReactFlow 模式仅传入 edgeId，需在 edges 中查找
  const handleEdgeClick = (edgeOrId: GraphEdge | string) => {
    const edge =
      typeof edgeOrId === 'string'
        ? edges.find((e, idx) => `e-${idx}` === edgeOrId)
        : edgeOrId;
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] edge clicked:', edge);
  };

  // 关闭抽屉回调：清空 editingEntity + 隐藏抽屉 + 清空重命名影响预览
  // 12.4.2：新增 setRenameImpactPreview(null) 避免下次打开抽屉时残留上次预览数据
  const handleDrawerClose = () => {
    setDrawerVisible(false);
    setEditingEntity(null);
    setRenameImpactPreview(null);
  };

  // 合并操作回调（11.4.2：Tauri 环境 invoke entity_merge；非 Tauri 环境降级 console.log）
  // Rust 命令参数 source_entity_id/target_entity_id 在 JS 侧自动映射为 camelCase
  const handleMerge = async (sourceId: string, targetId: string) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] merge entities:', sourceId, '->', targetId);
    if (isTauriRuntime()) {
      try {
        await invoke('entity_merge', {
          sourceEntityId: sourceId,
          targetEntityId: targetId,
        });
        // 持久化成功后刷新图谱数据（PoC 阶段为 mock useState 不可变；
        // 生产环境此处应触发 useQuery / SWR 重新拉取图谱数据）
      } catch (err) {
        // eslint-disable-next-line no-console
        console.error('[KnowledgeGraphView] entity_merge IPC failed:', err);
      }
    }
    setDrawerVisible(false);
    setEditingEntity(null);
  };

  // 拆分操作回调（11.4.2：Tauri 环境 invoke entity_split；非 Tauri 环境降级 console.log）
  // Rust 命令参数 source_entity_id/new_names 在 JS 侧自动映射为 camelCase
  const handleSplit = async (sourceId: string, newNames: string[]) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] split entity:', sourceId, '->', newNames);
    if (isTauriRuntime()) {
      try {
        // 注：invoke 返回 unknown，需 cast 为 string[]。
        // 12.4.2 整理：原 invoke<string[]>('entity_split', ...) 写法因含泛型参数，
        // 导致测试字符串断言 invoke('entity_split' 不匹配，统一改为 as cast 模式。
        const newIds = (await invoke('entity_split', {
          sourceEntityId: sourceId,
          newNames,
        })) as string[];
        // 持久化成功后刷新图谱数据（PoC 阶段为 mock useState 不可变）
        // eslint-disable-next-line no-console
        console.log('[KnowledgeGraphView] entity_split created new ids:', newIds);
      } catch (err) {
        // eslint-disable-next-line no-console
        console.error('[KnowledgeGraphView] entity_split IPC failed:', err);
      }
    }
    setDrawerVisible(false);
    setEditingEntity(null);
  };

  // 重命名影响预览回调（12.4.2：Tauri 环境 invoke preview_entity_rename_impact；
  // 非 Tauri 环境降级 console.log）。
  //
  // 纯 SELECT 查询，不修改任何数据。返回 RenameImpactPreview
  // （affected_events / affected_relations / affected_chunks），存入 renameImpactPreview state。
  //
  // Rust 命令参数 entity_id/new_name 在 JS 侧自动映射为 camelCase。
  // Rust 返回值字段 affected_events/affected_relations/affected_chunks 保持 snake_case
  // （Rust serde 默认行为），与前端 RenameImpactPreview 接口字段名一致。
  const handlePreviewRenameImpact = async (
    entityId: string,
    newName: string
  ) => {
    // eslint-disable-next-line no-console
    console.log(
      '[KnowledgeGraphView] preview rename impact:',
      entityId,
      '->',
      newName
    );
    if (isTauriRuntime()) {
      try {
        // 注：invoke 返回 unknown，需 cast 为 RenameImpactPreview。
        // 字段名 affected_events/affected_relations/affected_chunks 保持 snake_case
        // （Rust serde 默认行为，与前端 RenameImpactPreview 接口字段名一致）。
        const preview = (await invoke('preview_entity_rename_impact', {
          entityId,
          newName,
        })) as RenameImpactPreview;
        setRenameImpactPreview(preview);
      } catch (err) {
        // eslint-disable-next-line no-console
        console.error(
          '[KnowledgeGraphView] preview_entity_rename_impact IPC failed:',
          err
        );
        // 预览失败时清空面板，避免显示陈旧数据
        setRenameImpactPreview(null);
      }
    } else {
      // 非 Tauri 环境：清空预览（无真实数据，不展示 mock 防止误导用户）
      setRenameImpactPreview(null);
    }
  };

  // 重命名操作回调（12.4.2：Tauri 环境 invoke execute_entity_rename；非 Tauri 环境降级 console.log）
  //
  // 12.4.2 升级：原 11.4.2 的 entity_rename 命令仅更新 entity.name，
  // 12.4.2 改用 execute_entity_rename 命令在单个 SQLite 事务中同步更新：
  //   1. UPDATE entity SET name = new_name, normalized_name = ...
  //   2. UPDATE knowledge_event.content = REPLACE(content, old_name, new_name)
  //   3. UPDATE knowledge_event.summary = REPLACE(summary, old_name, new_name)
  //   4. UPDATE knowledge_event.title = REPLACE(title, old_name, new_name)
  // 任一步失败则 ROLLBACK，保证 entity.name 与 chunk_text 同步更新。
  //
  // Rust 命令参数 entity_id/new_name 在 JS 侧自动映射为 camelCase
  const handleRename = async (entityId: string, newName: string) => {
    // eslint-disable-next-line no-console
    console.log('[KnowledgeGraphView] rename entity:', entityId, '->', newName);
    if (isTauriRuntime()) {
      try {
        await invoke('execute_entity_rename', {
          entityId,
          newName,
        });
        // 持久化成功后刷新图谱数据（PoC 阶段为 mock useState 不可变；
        // 生产环境此处应触发 useQuery / SWR 重新拉取图谱数据）
      } catch (err) {
        // eslint-disable-next-line no-console
        console.error(
          '[KnowledgeGraphView] execute_entity_rename IPC failed:',
          err
        );
      }
    }
    setDrawerVisible(false);
    setEditingEntity(null);
    setRenameImpactPreview(null);
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

      {/* ─── PoC 提示卡片：标注当前实现范围（11.3.2 / 11.4.1 / 12.2.3） ─── */}
      <Card className={styles.pocHint} bordered>
        <span>图谱渲染 11.3.2（SVG 11 类着色）+ 11.4.1（@xyflow/react v12）+ 12.2.3（SAG 超边可视化：虚线 + 渐变色 + 查询高亮）</span>
      </Card>

      {/* ─── 11.4.1 渲染模式切换：SVG 模式 / ReactFlow 模式 ─── */}
      <div className={styles.modeSwitch}>
        <span className={styles.modeLabel}>渲染模式</span>
        <RadioGroup
          type='button'
          size='small'
          value={renderMode}
          onChange={(val: RenderMode) => setRenderMode(val)}
        >
          <Radio value='svg'>SVG 模式</Radio>
          <Radio value='flow'>ReactFlow 模式</Radio>
        </RadioGroup>
      </div>

      {/* ─── 主体：根据 renderMode 渲染 GraphCanvas 或 GraphFlow ─── */}
      {renderMode === 'flow' ? (
        <GraphFlow
          data={graphData}
          onNodeClick={handleNodeClick}
          onEdgeClick={(edgeId) => handleEdgeClick(edgeId)}
          // 12.2.3：传入超边数据 + 激活状态 + 查询上下文（仅在 ReactFlow 模式下渲染 HyperedgeLayer）
          hyperedges={hyperedges}
          activatedHyperedgeIds={activatedHyperedgeIds}
          queryEntities={queryEntities}
          onHyperedgeClick={handleHyperedgeClick}
        />
      ) : (
        <GraphCanvas
          nodes={nodes}
          edges={edges}
          onNodeClick={handleNodeClick}
          onEdgeClick={handleEdgeClick}
        />
      )}

      {/* ─── 11.5.1 MultiHopPathView：图谱下方显示当前选中 hit 的多跳检索路径 ─── */}
      {/* showPathView 控制可见性（默认 true 显示，onClose 后隐藏直到下次节点点击） */}
      {showPathView && (
        <div className={styles.pathViewWrap}>
          <MultiHopPathView
            hit={selectedHit}
            onClose={handlePathViewClose}
            onEntityClick={handleEntityClick}
          />
        </div>
      )}

      {/* ─── 11.3.3 EntityEditDrawer：节点点击打开抽屉（合并 / 拆分 / 重命名 3 操作） ─── */}
      {/* 12.4.2：新增 onPreviewRenameImpact + impactPreview 两个 prop（重命名影响预览） */}
      <EntityEditDrawer
        visible={drawerVisible}
        entity={editingEntity}
        onClose={handleDrawerClose}
        onMerge={handleMerge}
        onSplit={handleSplit}
        onRename={handleRename}
        onPreviewRenameImpact={handlePreviewRenameImpact}
        impactPreview={renameImpactPreview}
      />
    </div>
  );
};

export default KnowledgeGraphView;
