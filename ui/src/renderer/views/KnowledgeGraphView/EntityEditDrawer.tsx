/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * EntityEditDrawer — 实体编辑抽屉（spec §三 11.3.3 / 第 14 波并行 sub-step C）
 *
 * 本组件实现「实体编辑」抽屉，集成 Arco Design Drawer + Tabs：
 *   - 合并：将当前实体合并到目标实体（输入目标 entity_id）
 *   - 拆分：将当前实体拆分为多个新实体（输入新名称列表，逗号分隔）
 *   - 重命名：修改当前实体的 name（输入新名称）
 *
 * 范围说明：spec §三 11.3.3 原本包含「IPC 调用 + 持久化到 entity 表」，但本波
 * 仅实施前端 UI 部分（Drawer 组件 + 3 操作 tabs + PoC mock 回调），IPC 调用
 * 推迟到 11.4.x（与后端 entity 编辑命令同步实施）。这样本波可独立完成无后端依赖。
 *
 * 调用契约：
 *   - 父组件传入 visible / entity / onClose / onMerge / onSplit / onRename
 *   - 提交后调用对应回调（PoC：console.log mock + onClose 关闭抽屉）
 *   - 实际持久化由 11.4.x 阶段的 IPC 命令实现（entity 表更新）
 */

import React, { useState } from 'react';
import { Drawer, Tabs, Input, Button } from '@arco-design/web-react';
import type { GraphNode } from './types';
import styles from './EntityEditDrawer.module.css';

/**
 * EntityEditDrawer 组件 Props。
 *
 * - visible：抽屉是否可见（受控）
 * - entity：当前选中的节点（GraphNode | null）
 * - onClose：关闭抽屉回调
 * - onMerge：合并回调（sourceId → targetId），PoC 阶段父组件 console.log + 关闭抽屉
 * - onSplit：拆分回调（sourceId → newNames[]），PoC 阶段同上
 * - onRename：重命名回调（entityId → newName），PoC 阶段同上
 */
export interface EntityEditDrawerProps {
  visible: boolean;
  entity: GraphNode | null;
  onClose: () => void;
  onMerge?: (sourceId: string, targetId: string) => void;
  onSplit?: (sourceId: string, newNames: string[]) => void;
  onRename?: (entityId: string, newName: string) => void;
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
 *   - 调用对应的 onMerge / onSplit / onRename 回调
 *   - 通过 console.log 打印 PoC 操作日志（11.4.x 阶段替换为 IPC 调用）
 *   - 重置表单 + 关闭抽屉
 */
const EntityEditDrawer: React.FC<EntityEditDrawerProps> = ({
  visible,
  entity,
  onClose,
  onMerge,
  onSplit,
  onRename,
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
   * 重命名操作提交。
   * 修改当前实体（entityId = entity.id）的 name 为 newName。
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

            {/* ─── Tab 3：重命名 ─── */}
            <Tabs.TabPane key='rename' title='重命名'>
              <div className={styles.tabBody}>
                <p className={styles.tabHint}>
                  重命名当前实体（输入新名称替换当前实体 name）
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
                  <Button type='primary' onClick={handleRename}>
                    执行重命名
                  </Button>
                </div>
              </div>
            </Tabs.TabPane>
          </Tabs>
        </div>
      )}
    </Drawer>
  );
};

export default EntityEditDrawer;
