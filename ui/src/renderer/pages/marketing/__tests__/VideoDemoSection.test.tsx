/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * VideoDemoSection 测试 — spec §三 12.5.2 / 第二十二波 sub-agent B
 *
 * 项目约定：使用 bun:test + readFileSync 字符串断言（参考 12.5.1 MarketingPage.test.tsx）。
 * 本项目未引入 React Testing Library / jsdom，因此通过断言源码字符串 + existsSync / statSync
 * 双重契约保证 GIF 演示区块的行为。
 *
 * 测试用例对应 spec §三 12.5.2 验收点：
 *   1. test_video_demo_section_contains_reasoning_chain_gif — 演示区块含 reasoning_chain_demo.gif
 *   2. test_video_demo_section_contains_multihop_demo_gif     — 演示区块含 multihop_demo.gif
 *   3. test_gif_files_exist                                   — 两个 GIF 文件存在于 assets 目录
 *   4. test_gif_files_size_under_5mb                          — 每个 GIF < 5MB
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync, existsSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

// 读取 VideoDemoSection 源码（源码扫描模式，不渲染 React 组件）
const videoDemoSource = readFileSync(
  new URL('../sections/VideoDemoSection.tsx', import.meta.url),
  'utf8'
);

// 计算 assets 目录的绝对路径（用于 existsSync / statSync 文件存在性 + 大小检查）
const __dirname = dirname(fileURLToPath(import.meta.url));
const assetsDir = join(__dirname, '..', 'assets');
const reasoningChainGifPath = join(assetsDir, 'reasoning_chain_demo.gif');
const multihopGifPath = join(assetsDir, 'multihop_demo.gif');

describe('VideoDemoSection — 推理链可视化 GIF 演示区块（spec §三 12.5.2）', () => {
  test('test_video_demo_section_contains_reasoning_chain_gif: 演示区块含 reasoning_chain_demo.gif', () => {
    // VideoDemoSection.tsx 源码含 reasoning_chain_demo.gif 字符串
    // 直接扫描源码字符串而非通过常量引用，保证 GIF 路径契约
    const hasGifReference =
      videoDemoSource.includes('reasoning_chain_demo.gif') ||
      videoDemoSource.includes('REASONING_CHAIN_GIF');
    expect(hasGifReference).toBe(true);
  });

  test('test_video_demo_section_contains_multihop_demo_gif: 演示区块含 multihop_demo.gif', () => {
    // VideoDemoSection.tsx 源码含 multihop_demo.gif 字符串
    const hasGifReference =
      videoDemoSource.includes('multihop_demo.gif') ||
      videoDemoSource.includes('MULTIHOP_GIF');
    expect(hasGifReference).toBe(true);
  });

  test('test_gif_files_exist: 两个 GIF 文件存在于 assets 目录', () => {
    // reasoning_chain_demo.gif 与 multihop_demo.gif 必须存在于 assets 目录
    expect(existsSync(reasoningChainGifPath)).toBe(true);
    expect(existsSync(multihopGifPath)).toBe(true);
  });

  test('test_gif_files_size_under_5mb: 每个 GIF < 5MB', () => {
    // 5MB = 5 * 1024 * 1024 = 5242880 字节
    const MAX_GIF_SIZE = 5 * 1024 * 1024;

    const reasoningChainSize = statSync(reasoningChainGifPath).size;
    const multihopSize = statSync(multihopGifPath).size;

    expect(reasoningChainSize).toBeLessThan(MAX_GIF_SIZE);
    expect(multihopSize).toBeLessThan(MAX_GIF_SIZE);
  });
});
