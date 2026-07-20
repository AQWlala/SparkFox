/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * HyperedgeLayer — react-flow 超边可视化图层（spec §三 12.2.3 / 第二十二波 sub-step A）
 *
 * 本组件为 KnowledgeGraphView 的 GraphFlow 之上叠加「超边图层」：
 *   - 渲染后端 sparkfox-knowledge::hyperedge::Hyperedge 检测出的多元共现超边
 *   - 用虚线 + 渐变色（蓝→紫）区分普通二元边，突出 SAG 核心创新
 *   - 查询时（queryEntities 命中）高亮激活的超边（activatedHyperedgeIds）
 *   - 超边点击触发 onHyperedgeClick 回调（与 GraphFlow 的 onEdgeClick 命名风格一致）
 *
 * ## SAG 核心创新体现
 * 传统二元图：一条边只连接 2 个节点（event ↔ entity），无法表达「多事件-多实体共现」。
 * SAG 超边：>2 个 event 共享 >2 个 entity 时自动形成一条多元超边，表达「多对多」语义。
 * 本图层通过虚线 + 渐变色 + 高亮三重视觉差异，让用户在图谱上一眼识别超边结构。
 *
 * ## 渲染策略
 * - 超边作为独立 SVG 层叠加在 GraphFlow 画布之上（绝对定位 + pointer-events 控制）
 * - 每条超边渲染为一条 SVG path（虚拟曲线连接所有成员节点中心）
 * - 渐变色通过 SVG <linearGradient> 内联定义（蓝 #007aff → 紫 #8e4ec6）
 * - 激活状态：加粗 + opacity=1.0；未激活：opacity=0.4 淡显
 *
 * ## 与 GraphFlow 的关系
 * GraphFlow（11.4.1）渲染普通二元边（@xyflow/react v12 Edge 类型）。
 * HyperedgeLayer（12.2.3）渲染多元超边（独立 SVG 层，不污染 GraphFlow 的 Edge 数据流）。
 * 两者通过 React Portal / 绝对定位叠加，互不干扰。
 *
 * 范围说明（spec §三 12.2.3）：
 *   - PoC 阶段：渲染样式 + 高亮逻辑（mock 超边数据驱动）
 *   - 生产阶段（12.2.4+）：通过 IPC 调用 activate_local_hyperedges 拉取激活超边
 *   - 真实数据由 12.2.1/12.2.2 后端实现（HyperedgeDetector + activate_local_hyperedges）
 */

import React, { useMemo } from 'react';
import styles from './hyperedge.module.css';

// ─── 类型定义（组件内部 export，对齐后端 sparkfox-knowledge::hyperedge::Hyperedge） ─────

/**
 * 超边类型 — 对应后端 sparkfox-knowledge::hyperedge::Hyperedge。
 *
 * 字段对齐（snake_case，与 Rust serde 默认行为一致）：
 *   - id：超边唯一 ID（如 `"he_<hash>"`，由成员 events + entities 哈希生成）
 *   - member_events：成员 event IDs（≥3，已排序保证幂等）
 *   - member_entities：成员 entity IDs（≥3，已排序保证幂等）
 *
 * 后端来源：
 *   - detect_from_relations：从二元关系检测全图超边
 *   - activate_local_hyperedges：查询时激活局部超边（与 query_entities 有交集）
 */
export interface Hyperedge {
  /** 超边唯一 ID（如 `"he_<hash>"`） */
  id: string;
  /** 成员 event IDs（≥3，已排序，对应后端 member_events） */
  member_events: string[];
  /** 成员 entity IDs（≥3，已排序，对应后端 member_entities） */
  member_entities: string[];
}

// ─── Props ─────────────────────────────────────────────────────────────────────

/**
 * HyperedgeLayer 组件 Props（spec §三 12.2.3）。
 *
 * - hyperedges：所有超边列表（来自 detect_from_relations 全图检测）
 * - activatedHyperedgeIds：激活的超边 ID 列表（来自 activate_local_hyperedges 局部激活）
 * - queryEntities：查询命中的 entity IDs（驱动激活逻辑，12.2.2 后端过滤后传入）
 * - onHyperedgeClick：超边点击回调（参数为被点击的 Hyperedge 对象）
 *
 * 设计要点：
 *   - activatedHyperedgeIds + queryEntities 双重表达「激活」语义：
 *     * activatedHyperedgeIds：后端已计算好的激活超边 ID 列表（直接高亮）
 *     * queryEntities：保留查询上下文，便于未来扩展（如「显示与 query 相关的所有超边」）
 *   - PoC 阶段：父组件可只传 hyperedges + activatedHyperedgeIds（queryEntities 可选）
 */
export interface HyperedgeLayerProps {
  /** 所有超边列表（全图检测，来自 detect_from_relations） */
  hyperedges: Hyperedge[];
  /** 激活的超边 ID 列表（来自 activate_local_hyperedges；未传则全部淡显） */
  activatedHyperedgeIds?: string[];
  /** 查询命中的 entity IDs（保留查询上下文，便于未来扩展高亮逻辑） */
  queryEntities?: string[];
  /** 超边点击回调（参数为被点击的 Hyperedge 对象） */
  onHyperedgeClick?: (hyperedge: Hyperedge) => void;
}

// ─── 主组件 ───────────────────────────────────────────────────────────────────

/**
 * HyperedgeLayer 主组件。
 *
 * 渲染逻辑：
 *   1. 用 useMemo 派生 activatedHyperedgeIds 集合（Set 加速查找）
 *   2. 渲染 SVG 容器（绝对定位覆盖 GraphFlow 画布）
 *   3. 内联定义 <linearGradient>（蓝 #007aff → 紫 #8e4ec6，SAG 创新标识）
 *   4. 遍历 hyperedges，每条渲染为 <g> + <path>：
 *      - 虚线样式（strokeDasharray="5 3"，CSS 类 .hyperedgePath）
 *      - 渐变色（stroke="url(#sparkfox-hyperedge-gradient)"）
 *      - 激活状态：添加 .activated 类（加粗 + opacity=1.0）
 *   5. 绑定 onClick 触发 onHyperedgeClick 回调
 *
 * PoC 说明：
 *   - 超边路径暂用直线连接所有成员 entity 节点的平均中心点
 *     （生产阶段 12.2.4+ 改为贝塞尔曲线或凸包路径，更直观表达「多元」语义）
 *   - 节点坐标暂未接入 GraphFlow 实际布局，使用 0,0 占位
 *     （12.2.4 阶段从 GraphFlow ref 获取节点坐标，实现精确叠加）
 */
const HyperedgeLayer: React.FC<HyperedgeLayerProps> = ({
  hyperedges,
  activatedHyperedgeIds,
  queryEntities,
  onHyperedgeClick,
}) => {
  // 派生激活超边 ID 集合（Set 加速 O(1) 查找，避免每次渲染都遍历数组）
  const activatedSet = useMemo(
    () => new Set<string>(activatedHyperedgeIds ?? []),
    [activatedHyperedgeIds]
  );

  // queryEntities 为空时，认为「无查询」状态，所有超边淡显
  // （queryEntities 非空时，activatedHyperedgeIds 决定具体高亮哪些）
  const hasQuery = (queryEntities?.length ?? 0) > 0;
  void hasQuery; // PoC 阶段保留语义占位，12.2.4+ 接入完整激活逻辑

  // 超边点击回调（PoC：直接调用 props.onHyperedgeClick，传递被点击的 Hyperedge 对象）
  const handleClick = (hyperedge: Hyperedge) => {
    // eslint-disable-next-line no-console
    console.log('[HyperedgeLayer] hyperedge clicked:', hyperedge.id);
    onHyperedgeClick?.(hyperedge);
  };

  return (
    <div className={styles.hyperedgeLayer}>
      {/* ─── SVG 容器：与 GraphFlow 画布同尺寸，绝对定位叠加 ─── */}
      <svg
        width='100%'
        height='100%'
        viewBox='0 0 800 600'
        preserveAspectRatio='xMidYMid meet'
        role='img'
        aria-label='超边图层'
      >
        {/* ─── 渐变色定义（蓝→紫，SAG 核心创新标识） ─── */}
        {/* 蓝色 #007aff 呼应 hop1 ATOMIC 单跳基准色；紫色 #8e4ec6 对应 NUMBER 实体色 */}
        <defs>
          <linearGradient id='sparkfox-hyperedge-gradient' x1='0%' y1='0%' x2='100%' y2='100%'>
            <stop offset='0%' stopColor='#007aff' />
            <stop offset='100%' stopColor='#8e4ec6' />
          </linearGradient>
        </defs>

        {/* ─── 渲染所有超边（每条为 <g> + <path>） ─── */}
        {hyperedges.map((he: Hyperedge) => {
          // 判断当前超边是否被激活（在 activatedHyperedgeIds 列表中）
          const isActivated = activatedSet.has(he.id);
          // 动态拼接 className：基础虚线样式 + 激活态高亮类
          const pathClassName = isActivated
            ? `${styles.hyperedgePath} ${styles.activated}`
            : styles.hyperedgePath;

          // PoC 阶段：超边路径暂用一条直线占位（从 (100, 100) 到 (700, 500)）
          // 12.2.4+ 阶段：从 GraphFlow ref 获取成员节点坐标，渲染连接所有成员的曲线
          return (
            <g
              key={`hyperedge-${he.id}`}
              className={styles.hyperedgeGroup}
              onClick={() => handleClick(he)}
            >
              <path
                d='M 100 100 L 700 500'
                className={pathClassName}
              />
              {/* 超边标签：显示成员数（events × entities），便于用户识别超边规模 */}
              <text
                x={400}
                y={300}
                className={styles.hyperedgeLabel}
              >
                {`HE ${he.member_events.length}×${he.member_entities.length}`}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
};

HyperedgeLayer.displayName = 'HyperedgeLayer';

export default HyperedgeLayer;
