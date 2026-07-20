/**
 * SparkFox Apple 主题预设 — macOS 暗色
 *
 * 来源：SparkFox 全新设计（Apple 系统风格）
 * 功能：CSS 变量映射，暗色模式
 *
 * 占位文件：实际实现见步骤 0.4
 */

export const macosDarkTheme = {
  name: 'macosDark',
  displayName: 'macOS Dark',
  colors: {
    primary: '#0A84FF',       // Apple 系统蓝（暗色）
    primaryHover: '#0066CC',
    primaryActive: '#0055AA',
    success: '#30D158',
    warning: '#FF9F0A',
    danger: '#FF453A',
    info: '#64D2FF',
    text: '#F5F5F7',
    textSecondary: '#AEAEB2',
    textTertiary: '#6E6E73',
    bg: '#1C1C1E',
    bgSecondary: '#2C2C2E',
    bgTertiary: '#3A3A3C',
    border: '#48484A',
  },
  radius: {
    small: '6px',
    medium: '10px',
    large: '16px',
  },
  font: {
    family: '-apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display", "Segoe UI", Roboto, sans-serif',
    size: '14px',
  },
} as const;
