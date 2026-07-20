/**
 * SparkFox Apple 主题预设 — macOS 跟随系统
 *
 * 来源：SparkFox 全新设计（Apple 系统风格）
 * 功能：根据 prefers-color-scheme 自动切换亮/暗色
 *
 * 占位文件：实际实现见步骤 0.4
 */

import { macosLightTheme } from './macosLight';
import { macosDarkTheme } from './macosDark';

export type ThemePreset = 'macosLight' | 'macosDark';

// 通用主题类型（放宽字面量约束，允许亮/暗色互相赋值）
export type MacosTheme = {
  name: string;
  displayName: string;
  colors: Record<string, string>;
  radius: Record<string, string>;
  font: { family: string; size: string };
};

export function getMacosAutoTheme(): MacosTheme {
  if (typeof window === 'undefined') {
    return macosLightTheme;
  }
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
  return prefersDark ? macosDarkTheme : macosLightTheme;
}

export const macosAutoTheme = {
  name: 'macosAuto',
  displayName: 'macOS Auto',
  getTheme: getMacosAutoTheme,
} as const;
