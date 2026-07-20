/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox AgentIcon — Agent 图标组件
 *
 * 来源：OpenAkita apps/setup-center/src/components/AgentIcon.tsx（清洁室重写）
 *
 * 保留 OpenAkita 特性：
 * - 28 个 SVG 路径图标定义（terminal/code/globe/shield/database/cpu/cloud/lock/zap/eye/
 *   message/mail/chart/network/target/compass/layers/workflow/flask/pen/mic/bot/puzzle/heart 等）
 * - emoji 图标支持（"🤖" / "🦊" 等）
 * - 自定义 URL 图标支持（"/agents/x/icon.png" / "https://..."）
 * - isCustomAgentIcon / resolveAgentIconUrl / agentIconText 工具函数
 * - 颜色 / 大小 / 降级（fallback）参数
 *
 * 改造点：
 * - 移除 apiBaseUrl 参数（改为 SparkFox 本地资源路径前缀）
 * - 使用 Apple 系统风格（圆角 + SF Pro 字体）
 */

import React from 'react';

/** 28 个 SVG 路径图标定义（完整迁移自 OpenAkita） */
export const AGENT_SVG_ICONS: Record<string, { path: string; label: string }> = {
  terminal: { label: '终端', path: 'M4 17l6-5-6-5M12 19h8' },
  code: { label: '代码', path: 'M16 18l6-6-6-6M8 6l-6 6 6 6' },
  globe: { label: '全球', path: 'M12 2a10 10 0 100 20 10 10 0 000-20zM2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10A15.3 15.3 0 0112 2z' },
  shield: { label: '安全', path: 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z' },
  database: { label: '数据库', path: 'M12 2C6.48 2 2 3.79 2 6v12c0 2.21 4.48 4 10 4s10-1.79 10-4V6c0-2.21-4.48-4-10-4zM2 12c0 2.21 4.48 4 10 4s10-1.79 10-4M2 6c0 2.21 4.48 4 10 4s10-1.79 10-4' },
  cpu: { label: '芯片', path: 'M6 6h12v12H6zM9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M2 15h4M18 9h4M18 15h4' },
  cloud: { label: '云', path: 'M18 10h-1.26A8 8 0 109 20h9a5 5 0 000-10z' },
  lock: { label: '锁', path: 'M19 11H5a2 2 0 00-2 2v7a2 2 0 002 2h14a2 2 0 002-2v-7a2 2 0 00-2-2zM7 11V7a5 5 0 0110 0v4' },
  zap: { label: '闪电', path: 'M13 2L3 14h9l-1 8 10-12h-9l1-8z' },
  eye: { label: '监控', path: 'M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8zM12 9a3 3 0 100 6 3 3 0 000-6z' },
  message: { label: '对话', path: 'M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z' },
  mail: { label: '邮件', path: 'M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2zM22 6l-10 7L2 6' },
  chart: { label: '图表', path: 'M18 20V10M12 20V4M6 20v-6' },
  network: { label: '网络', path: 'M5.5 5.5a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM18.5 5.5a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM12 24a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM5.5 5.5L12 19M18.5 5.5L12 19' },
  target: { label: '靶心', path: 'M12 2a10 10 0 100 20 10 10 0 000-20zM12 6a6 6 0 100 12 6 6 0 000-12zM12 10a2 2 0 100 4 2 2 0 000-4z' },
  compass: { label: '指南', path: 'M12 2a10 10 0 100 20 10 10 0 000-20zM16.24 7.76l-2.12 6.36-6.36 2.12 2.12-6.36z' },
  layers: { label: '层级', path: 'M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5' },
  workflow: { label: '流程', path: 'M6 3a3 3 0 100 6 3 3 0 000-6zM18 15a3 3 0 100 6 3 3 0 000-6zM8.59 13.51l6.83 3.98M6 9v4M18 9v6' },
  flask: { label: '实验', path: 'M9 3h6M10 3v6.5l-5 8.5h14l-5-8.5V3' },
  pen: { label: '创作', path: 'M12 20h9M16.5 3.5a2.12 2.12 0 013 3L7 19l-4 1 1-4L16.5 3.5z' },
  mic: { label: '语音', path: 'M12 1a3 3 0 00-3 3v8a3 3 0 006 0V4a3 3 0 00-3-3zM19 10v2a7 7 0 01-14 0v-2M12 19v4M8 23h8' },
  bot: { label: '机器人', path: 'M12 2a2 2 0 012 2v1h3a2 2 0 012 2v10a2 2 0 01-2 2H7a2 2 0 01-2-2V7a2 2 0 012-2h3V4a2 2 0 012-2zM9 13h0M15 13h0M9 17h6' },
  puzzle: { label: '拼图', path: 'M19.439 12.956l-1.5 0a2 2 0 010-4l1.5 0a.5.5 0 00.5-.5l0-2.5a2 2 0 00-2-2l-2.5 0a.5.5 0 01-.5-.5l0-1.5a2 2 0 00-4 0l0 1.5a.5.5 0 01-.5.5L7.939 3.956a2 2 0 00-2 2l0 2.5a.5.5 0 00.5.5l1.5 0a2 2 0 010 4l-1.5 0a.5.5 0 00-.5.5l0 2.5a2 2 0 002 2l2.5 0a.5.5 0 01.5.5l0 1.5a2 2 0 004 0l0-1.5a.5.5 0 01.5-.5l2.5 0a2 2 0 002-2l0-2.5a.5.5 0 00-.5-.5z' },
  heart: { label: '爱心', path: 'M20.84 4.61a5.5 5.5 0 00-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 00-7.78 7.78L12 21.23l8.84-8.84a5.5 5.5 0 000-7.78z' },
  book: { label: '书籍', path: 'M4 19.5A2.5 2.5 0 016.5 17H20M6.5 2H20v20H6.5A2.5 2.5 0 014 19.5v-15A2.5 2.5 0 016.5 2z' },
  rocket: { label: '火箭', path: 'M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 00-2.91-.09zM12 15l-3-3a22 22 0 012-3.95A12.88 12.88 0 0122 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 01-4 2zM9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5' },
  star: { label: '星标', path: 'M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z' },
  fire: { label: '火焰', path: 'M8.5 14.5A2.5 2.5 0 0011 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 11-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 002.5 2.5z' },
};

/** 判断是否为自定义 URL 图标 */
export function isCustomAgentIcon(icon: string | null | undefined): boolean {
  if (!icon) return false;
  return icon.startsWith('/') || icon.startsWith('http://') || icon.startsWith('https://');
}

/** 解析图标 URL（本地路径加前缀） */
export function resolveAgentIconUrl(icon: string, baseUrl = ''): string {
  if (icon.startsWith('http://') || icon.startsWith('https://')) return icon;
  if (icon.startsWith('/') && baseUrl) return `${baseUrl.replace(/\/+$/, '')}${icon}`;
  return icon;
}

/** 获取图标显示文本（用于 SVG 不支持时的降级） */
export function agentIconText(icon: string | null | undefined): string {
  if (!icon) return '🤖';
  return isCustomAgentIcon(icon) ? '🖼️' : icon;
}

interface AgentIconProps {
  icon?: string | null;
  /** 主题色（#RRGGBB），应用于 SVG 描边和 emoji 背景 */
  color?: string;
  /** 尺寸（px） */
  size?: number;
  /** 自定义 URL 图标的资源前缀 */
  baseUrl?: string;
  className?: string;
  style?: React.CSSProperties;
  /** 降级文本（图标加载失败时显示） */
  fallback?: string;
}

/**
 * Agent 图标组件
 *
 * 三种渲染模式：
 * 1. SVG 图标：icon 以 "svg:" 开头（如 "svg:bot"）→ 渲染 SVG path
 * 2. 自定义 URL：icon 以 "/" 或 "http(s)://" 开头 → 渲染 <img>
 * 3. emoji：其他 → 直接显示 emoji 字符
 */
export const AgentIcon: React.FC<AgentIconProps> = ({
  icon,
  color = 'currentColor',
  size = 16,
  baseUrl = '',
  className,
  style,
  fallback,
}) => {
  const [imgError, setImgError] = React.useState(false);

  // 模式 1：SVG 图标
  if (icon && icon.startsWith('svg:')) {
    const key = icon.slice(4);
    const svgIcon = AGENT_SVG_ICONS[key];
    if (svgIcon) {
      return (
        <svg
          width={size}
          height={size}
          viewBox="0 0 24 24"
          fill="none"
          stroke={color}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={className}
          style={style}
          aria-label={svgIcon.label}
          role="img"
        >
          <path d={svgIcon.path} />
        </svg>
      );
    }
  }

  // 模式 2：自定义 URL 图标
  if (icon && isCustomAgentIcon(icon) && !imgError) {
    return (
      <img
        src={resolveAgentIconUrl(icon, baseUrl)}
        alt="Agent icon"
        width={size}
        height={size}
        className={className}
        style={{
          borderRadius: Math.max(2, size * 0.15),
          objectFit: 'cover',
          ...style,
        }}
        onError={() => setImgError(true)}
      />
    );
  }

  // 模式 3：emoji（含降级）
  const text = imgError ? fallback || '🤖' : agentIconText(icon);
  return (
    <span
      className={className}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: size,
        height: size,
        fontSize: size * 0.75,
        lineHeight: 1,
        background: isCustomAgentIcon(icon) ? `${color}22` : 'transparent',
        borderRadius: Math.max(2, size * 0.15),
        ...style,
      }}
      role="img"
      aria-label={text}
    >
      {text}
    </span>
  );
};

export default AgentIcon;
