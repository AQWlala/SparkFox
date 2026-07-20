/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * EntityEditDrawer — 实体编辑抽屉（spec §三 11.3.3 / 11.4.2 / 12.4.2 IPC 持久化）
 *
 * 本组件实现「实体编辑」抽屉，集成 Arco Design Drawer + Tabs：
 *   - 合并：将当前实体合并到目标实体（输入目标 entity_id）
 *   - 拆分：将当前实体拆分为多个新实体（输入新名称列表，逗号分隔）
 *   - 重命名：修改当前实体的 name（输入新名称）— 12.4.2 增强为「预览影响 + 确认执行」两步流程
 *
 * 范围说明：spec §三 11.3.3 第 14 波仅实施前端 UI 部分（Drawer + 3 Tabs +
 * PoC mock 回调）；Sub-Step 11.4.2 将 onMerge/onSplit/onRename 回调接入
 * Tauri IPC（entity_merge / entity_split / entity_rename 命令），持久化到
 * entity 表 + event_entity_relation 表。本组件仅触发回调 + console.log
 * 调试日志，实际 IPC 调用 + 环境检测降级由父组件 index.tsx 实现。
 *
 * Sub-Step 12.4.2 增强：重命名 tab 新增「重命名影响预览」面板，
 * 用户输入新名称后点击「预览影响」按钮，父组件 invoke preview_entity_rename_impact
 * 查询受影响 events / relations / chunks 数量（纯 SELECT 不修改数据），
 * 预览数据通过 impactPreview prop 传回本组件渲染。用户确认后点击「执行重命名」按钮，
 * 父组件 invoke execute_entity_rename（事务原子性：BEGIN/COMMIT/ROLLBACK）。
 *
 * 调用契约：
 *   - 父组件传入 visible / entity / onClose / onMerge / onSplit / onRename
 *   - 12.4.2 新增：父组件传入 onPreviewRenameImpact + impactPreview
 *   - 提交后调用对应回调（含 console.log 调试日志 + onClose 关闭抽屉）
 *   - 实际持久化由 11.4.2 阶段的 IPC 命令实现（entity 表更新，父组件负责 invoke）
 */

import React, { useState } from 'react';
import { Drawer, Tabs, Input, Button } from '@arco-design/web-react';
import type { GraphNode } from './types';
import styles from './EntityEditDrawer.module.css';

/**
 * 重命名影响预览 DTO（spec §三 12.4.2）
 *
 * 字段命名与后端 Rust 结构体 `sparkfox_knowledge::entity_ops::RenameImpactPreview`
 * 的 serde JSON 输出保持一致（snake_case），便于 Tauri IPC 反序列化时字段名直接对应。
 *
 * - affected_events：受影响的 event 数量（DISTINCT event_id）
 * - affected_relations：受影响的 event_entity_relation 行数
 * - affected_chunks：受影响的 knowledge_event 行数（content/summary/title 含旧 name）
 */
export interface RenameImpactPreview {
  affected_events: number;
  affected_relations: number;
  affected_chunks: number;
}

/**
 * EntityEditDrawer 组件 Props。
 *
 * - visible：抽屉是否可见（受控）
 * - entity：当前选中的节点（GraphNode | null）
 * - onClose：关闭抽屉回调
 * - onMerge：合并回调（sourceId → targetId），11.4.2 父组件 invoke entity_merge + 刷新图谱
 * - onSplit：拆分回调（sourceId → newNames[]），11.4.2 父组件 invoke entity_split + 刷新图谱
 * - onRename：重命名回调（entityId → newName），12.4.2 父组件 invoke execute_entity_rename + 刷新图谱
 * - onPreviewRenameImpact：重命名影响预览回调（entityId → newName），
 *   12.4.2 父组件 invoke preview_entity_rename_impact（纯 SELECT 不修改数据）
 * - impactPreview：影响预览数据（父组件控制，本组件仅展示；null 表示未预览）
 */
export interface EntityEditDrawerProps {
  visible: boolean;
  entity: GraphNode | null;
  onClose: () => void;
  onMerge?: (sourceId: string, targetId: string) => void;
  onSplit?: (sourceId: string, newNames: string[]) => void;
  onRename?: (entityId: string, newName: string) => void;
  onPreviewRenameImpact?: (entityId: string, newName: string) => void;
  impactPreview?: RenameImpactPreview | null;
}

/**
 * EntityEditDrawer 主组件。
 *
 * 内部状态：
 *   - targetId：合并 tab 中输入的目标实体 ID
 *   - splitNames：拆分 tab 中输入的新实体名称列表（逗号分隔字符串）
 *   - newName：重命名 tab 中输入的新名称
 *
 * 提交逻辑（每个 tab）：
 *   - 校验 entity 非空
 *   - 调用对应的 onMerge / onSplit / onRename 回调（父组件 11.4.2 实现 IPC invoke）
 *   - 通过 console.log 打印调试日志（保留用于追溯 + 测试断言）
 *   - 重置表单 + 关闭抽屉
 */
const EntityEditDrawer: React.FC<EntityEditDrawerProps> = ({
  visible,
  entity,
  onClose,
  onMerge,
  onSplit,
  onRename,
  onPreviewRenameImpact,
  impactPreview,
}) => {
  // 合并 tab：目标实体 ID
  const [targetId, setTargetId] = useState<string>('');
  // 拆分 tab：新实体名称列表（逗号分隔）
  const [splitNames, setSplitNames] = useState<string>('');
  // 重命名 tab：新名称
  const [newName, setNewName] = useState<string>('');

  /**
   * 合并操作提交。
   * 将当前实体（sourceId = entity.id）合并到目标实体（targetId）。
   */
  const handleMerge = () => {
    if (!entity || !targetId.trim()) return;
    // eslint-disable-next-line no-console
    console.log('[EntityEditDrawer] merge:', entity.id, '->', targetId);
    onMerge?.(entity.id, targetId.trim());
    setTargetId('');
    onClose();
  };

  /**
   * 拆分操作提交。
   * 将当前实体（sourceId = entity.id）拆分为多个新实体（newNames）。
   * 输入字符串按中英文逗号分隔后去空白、去空串。
   */
  const handleSplit = () => {
    if (!entity || !splitNames.trim()) return;
    const newNames = splitNames
      .split(/[,，]/)
      .map((n) => n.trim())
      .filter((n) => n.length > 0);
    if (newNames.length === 0) return;
    // eslint-disable-next-line no-console
    console.log('[EntityEditDrawer] split:', entity.id, '->', newNames);
    onSplit?.(entity.id, newNames);
    setSplitNames('');
    onClose();
  };

  /**
   * 重命名影响预览（spec §三 12.4.2）。
   *
   * 用户输入新名称后点击「预览影响」按钮触发，父组件 invoke preview_entity_rename_impact
   * 查询受影响 events / relations / chunks 数量（纯 SELECT 不修改数据）。
   * 预览结果通过 impactPreview prop 传回本组件渲染。
   */
  const handlePreviewRenameImpact = () => {
    if (!entity || !newName.trim()) return;
    // eslint-disable-next-line no-console
    console.log(
      '[EntityEditDrawer] preview rename impact:',
      entity.id,
      '->',
      newName
    );
    onPreviewRenameImpact?.(entity.id, newName.trim());
  };

  /**
   * 重命名操作提交（spec §三 12.4.2：执行重命名，事务原子性）。
   *
   * 修改当前实体（entityId = entity.id）的 name 为 newName。
   * 父组件 invoke execute_entity_rename（BEGIN/COMMIT/ROLLBACK），
   * 同步更新 entity.name + knowledge_event.content/summary/title。
   */
  const handleRename = () => {
    if (!entity || !newName.trim()) return;
    // eslint-disable-next-line no-console
    console.log('[EntityEditDrawer] rename:', entity.id, '->', newName);
    onRename?.(entity.id, newName.trim());
    setNewName('');
    onClose();
  };

  return (
    <Drawer
      title='实体编辑'
      visible={visible}
      onCancel={onClose}
      placement='right'
      width={520}
      okButtonProps={{ style: { display: 'none' } }}
      cancelText='关闭'
    >
      {!entity ? (
        <div className={styles.placeholder}>暂无选中实体</div>
      ) : (
        <div className={styles.drawerBody}>
          {/* ─── 当前实体信息卡片 ─── */}
          <div className={styles.entityInfo}>
            <div className={styles.entityInfoRow}>
              <span className={styles.entityInfoLabel}>实体 ID</span>
              <span className={styles.entityInfoValue}>{entity.id}</span>
            </div>
            <div className={styles.entityInfoRow}>
              <span className={styles.entityInfoLabel}>名称</span>
              <span className={styles.entityInfoValue}>{entity.label}</span>
            </div>
            <div className={styles.entityInfoRow}>
              <span className={styles.entityInfoLabel}>类型</span>
              <span className={styles.entityInfoValue}>{entity.type}</span>
            </div>
          </div>

          {/* ─── 3 操作 Tabs：合并 / 拆分 / 重命名 ─── */}
          <Tabs defaultActiveTab='merge' className={styles.tabs}>
            {/* ─── Tab 1：合并 ─── */}
            <Tabs.TabPane key='merge' title='合并'>
              <div className={styles.tabBody}>
                <p className={styles.tabHint}>
                  合并到目标实体（将当前实体合并到目标实体，目标实体保留）
                </p>
                <div className={styles.formItem}>
                  <label className={styles.formLabel}>目标实体 ID</label>
                  <Input
                    placeholder='输入目标 entity_id（如 n2）'
                    value={targetId}
                    onChange={setTargetId}
                    allowClear
                  />
                </div>
                <div className={styles.formActions}>
                  <Button type='primary' onClick={handleMerge}>
                    执行合并
                  </Button>
                </div>
              </div>
            </Tabs.TabPane>

            {/* ─── Tab 2：拆分 ─── */}
            <Tabs.TabPane key='split' title='拆分'>
              <div className={styles.tabBody}>
                <p className={styles.tabHint}>
                  拆分为多个实体（按逗号分隔多个新实体名称）
                </p>
                <div className={styles.formItem}>
                  <label className={styles.formLabel}>新实体名称列表（逗号分隔）</label>
                  <Input.TextArea
                    placeholder='如：实体A，实体B，实体C'
                    value={splitNames}
                    onChange={setSplitNames}
                    autoSize={{ minRows: 3, maxRows: 6 }}
                  />
                </div>
                <div className={styles.formActions}>
                  <Button type='primary' onClick={handleSplit}>
                    执行拆分
                  </Button>
                </div>
              </div>
            </Tabs.TabPane>

            {/* ─── Tab 3：重命名（12.4.2 增强：预览影响 + 确认执行两步流程） ─── */}
            <Tabs.TabPane key='rename' title='重命名'>
              <div className={styles.tabBody}>
                <p className={styles.tabHint}>
                  重命名当前实体（输入新名称替换当前实体 name）。
                  12.4.2 新增「预览影响」按钮：先查询受影响 events / relations / chunks 数量，
                  确认后再点击「执行重命名」事务执行（BEGIN/COMMIT/ROLLBACK）。
                </p>
                <div className={styles.formItem}>
                  <label className={styles.formLabel}>新名称</label>
                  <Input
                    placeholder='输入新的实体名称'
                    value={newName}
                    onChange={setNewName}
                    allowClear
                  />
                </div>
                <div className={styles.formActions}>
                  {/* 12.4.2：预览影响按钮（纯查询不修改数据），与「执行重命名」并列 */}
                  <Button onClick={handlePreviewRenameImpact}>
                    预览影响
                  </Button>
                  <Button type='primary' onClick={handleRename}>
                    执行重命名
                  </Button>
                </div>
                {/* 12.4.2：重命名影响预览面板（impactPreview 非空时显示） */}
                {impactPreview ? (
                  <div className={styles.impactPanel}>
                    <p className={styles.impactTitle}>受影响范围预览</p>
                    <div className={styles.impactRow}>
                      <span className={styles.impactLabel}>受影响事件</span>
                      <span className={styles.impactValue}>
                        {impactPreview.affected_events}
                      </span>
                    </div>
                    <div className={styles.impactRow}>
                      <span className={styles.impactLabel}>受影响关系</span>
                      <span className={styles.impactValue}>
                        {impactPreview.affected_relations}
                      </span>
                    </div>
                    <div className={styles.impactRow}>
                      <span className={styles.impactLabel}>受影响文本块</span>
                      <span className={styles.impactValue}>
                        {impactPreview.affected_chunks}
                      </span>
                    </div>
                  </div>
                ) : null}
              </div>
            </Tabs.TabPane>
          </Tabs>
        </div>
      )}
    </Drawer>
  );
};

export default EntityEditDrawer;
