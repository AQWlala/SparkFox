/**
 * SparkFox sceneStore — Scene Protocol 状态管理
 *
 * 来源：SparkFox 全新设计（基于 BaiLongma Scene Protocol）
 * 功能：场景切换、场景上下文、Tick 心跳
 *
 * 占位文件：实际实现见 Phase 2
 *
 * ─── Tauri IPC 对接说明（v1.0.0 Task 7.8） ───
 * 本 store 暂无直接对应的 Tauri command（sparkfox-ipc 未提供 scene_* 接口）。
 * v1.1.0+ 计划新增：
 *   - scene_switch(name) → 切换当前场景
 *   - scene_get_context(name) → 拉取场景上下文
 *   - scene_tick_emit(name, payload) → 发出 Tick 心跳
 * 当前 PoC 阶段保持占位，不影响其他 store 的 IPC 对接。
 */

import { create } from 'zustand';

interface SceneState {
  initialized: boolean;
  currentScene: string;
  // 占位：后续添加场景协议完整字段
}

export const useSceneStore = create<SceneState>(() => ({
  initialized: false,
  currentScene: 'default',
}));
