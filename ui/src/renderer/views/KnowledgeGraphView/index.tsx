/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * KnowledgeGraphView — 知识图谱视图入口（spec §三 11.3.1 / 第 12 波并行 sub-step B）
 *
 * 本文件提供「知识图谱」页面的最小可用骨架：
 *   - 顶部：标题「知识图谱」+ 返回按钮（返回知识库详情页）
 *   - 主体：占位卡片，提示「图谱渲染待 11.3.2 实现」
 *
 * 实际的图谱节点渲染、力导布局、实体/事件筛选器将在 spec §三 11.3.2 阶段
 * 接入 d3 / @xyflow/react 实现并提供真实数据钩子。
 *
 * 路由：/kb/:id/graph → KnowledgeGraphView（kbId 从 useParams 获取）
 * 入口：KnowledgeDetailPage 顶部操作栏「查看知识图谱」按钮（Link）
 */

import React from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { Button, Card } from '@arco-design/web-react';
import { Left } from '@icon-park/react';
import styles from './styles.module.css';

/**
 * KnowledgeGraphView 主组件。
 *
 * Props 暂未定义——kbId 从路由参数 useParams 获取，与 KnowledgeDetailPage 保持一致。
 * 后续 11.3.2 阶段如需追加筛选器 / 实体类型 Props，再扩展此接口。
 */
const KnowledgeGraphView: React.FC = () => {
  const navigate = useNavigate();
  // 从路由参数 /kb/:id/graph 提取知识库 ID
  const { id: kbId } = useParams<{ id: string }>();

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

      {/* ─── 主体：占位卡片（11.3.2 阶段替换为真实图谱渲染） ─── */}
      <Card className={styles.placeholder} bordered>
        <span>图谱渲染待 11.3.2 实现</span>
      </Card>
    </div>
  );
};

export default KnowledgeGraphView;
