/**
 * SparkFox Apple 主题预设 — macOS 亮色
 *
 * 来源：SparkFox 全新设计（Apple 系统风格）
 * 功能：CSS 变量映射，Apple 系统蓝 #007AFF 主色 + macOS 圆角 + SF Pro 字体
 *
 * 占位文件：实际实现见步骤 0.4
 */

export const macosLightTheme = {
  name: 'macosLight',
  displayName: 'macOS Light',
  colors: {
    primary: '#007AFF',       // Apple 系统蓝
    primaryHover: '#0066D6',
    primaryActive: '#0054B3',
    success: '#34C759',       // Apple 系统绿
    warning: '#FF9500',       // Apple 系统橙
    danger: '#FF3B30',        // Apple 系统红
    info: '#5AC8FA',          // Apple 系统青
    text: '#1D1D1F',
    textSecondary: '#6E6E73',
    textTertiary: '#AEAEB2',
    bg: '#FFFFFF',
    bgSecondary: '#F5F5F7',
    bgTertiary: '#E5E5EA',
    border: '#D2D2D7',
  },
  radius: {
    small: '6px',
    medium: '10px',   // macOS 圆角
    large: '16px',
  },
  font: {
    family: '-apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display", "Segoe UI", Roboto, sans-serif',
    size: '14px',
  },
} as const;
