/**
 * SparkFox Sider — SparkFox 6 大路由侧边栏
 *
 * 来源：SparkFox 全新设计（基于 NomiFun Sider 风格 + Apple 系统风格）
 * 功能：6 大路由导航（对话/Agent/监视/热点/记忆/设置）+ 快捷键提示
 *
 * 设计：
 * - Apple 系统蓝 #007AFF 主色
 * - macOS 圆角
 * - SF Pro 字体
 * - 6 个导航项 + 快捷键 Cmd/Ctrl+1~5 + Cmd/Ctrl+,
 *
 * 占位文件：实际实现见 Phase 1（当前仅用于步骤 0.5 验证骨架）
 */

import React from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { sparkfoxRoutes } from '@renderer/router/routes';
import { registerSparkFoxShortcuts } from '@renderer/router/shortcuts';
import { useSettingsStore } from '@renderer/store/settingsStore';

// SparkFox 路由前缀（与 Router.tsx 中的 /sparkfox/* 对应）
const SPARKFOX_PREFIX = '/sparkfox';
const Icon: React.FC<{ name: string; size?: number }> = ({ name, size = 18 }) => {
  const icons: Record<string, string> = {
    Message: '💬',
    Robot: '🤖',
    BarChart: '📊',
    Newspaper: '📰',
    Brain: '🧠',
    Settings: '⚙️',
    // 知识库入口（F1）
    Book: '📚',
  };
  return (
    <span style={{ fontSize: size, lineHeight: 1, display: 'inline-block' }} role="img" aria-label={name}>
      {icons[name] || '•'}
    </span>
  );
};

const SparkFoxSider: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const primaryColor = useSettingsStore((s) => s.primaryColor);

  // 注册快捷键
  React.useEffect(() => {
    const unregister = registerSparkFoxShortcuts((path) => navigate(path));
    return unregister;
  }, [navigate]);

  // Agent 子菜单展开状态（持久化到 localStorage）
  const [agentMenuExpanded, setAgentMenuExpanded] = React.useState<boolean>(() => {
    try {
      return localStorage.getItem('sparkfox-sider-agent-expanded') === '1';
    } catch {
      return false;
    }
  });

  const toggleAgentMenu = () => {
    setAgentMenuExpanded((prev) => {
      const next = !prev;
      try {
        localStorage.setItem('sparkfox-sider-agent-expanded', next ? '1' : '0');
      } catch {
        // ignore
      }
      return next;
    });
  };

  // Agent 子菜单项（对应 OpenAkita Sidebar 的 Agent 分组）
  const agentSubMenu: Array<{ path: string; label: string; icon: string }> = [
    { path: '/agents', label: 'Agent 管理', icon: '📋' },
    { path: '/agents/store', label: 'Agent 商店', icon: '🏪' },
    { path: '/agents/dashboard', label: 'Agent 仪表盘', icon: '📊' },
    { path: '/agents/system', label: 'Agent 系统', icon: '⚙️' },
  ];

  return (
    <nav
      style={{
        width: 184,
        height: '100%',
        background: 'var(--sf-color-bg, #FFFFFF)',
        borderRight: '1px solid var(--sf-color-border, #D2D2D7)',
        padding: '16px 12px',
        boxSizing: 'border-box',
        fontFamily: 'var(--sf-font-family, -apple-system, sans-serif)',
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
      }}
    >
      {/* SparkFox Logo */}
      <div
        style={{
          padding: '8px 12px 16px',
          fontWeight: 600,
          fontSize: 15,
          color: 'var(--sf-color-text, #1D1D1F)',
          letterSpacing: '-0.2px',
        }}
      >
        🦊 SparkFox
      </div>

      {/* 6 大路由导航 */}
      {sparkfoxRoutes.map((route, idx) => {
        const fullPath = `${SPARKFOX_PREFIX}${route.path === '/' ? '' : route.path}`;
        // 兼容末尾斜杠：/sparkfox 和 /sparkfox/ 都算激活
        const isActive =
          location.pathname === fullPath ||
          location.pathname === `${fullPath}/` ||
          (route.path === '/' && (location.pathname === SPARKFOX_PREFIX || location.pathname === `${SPARKFOX_PREFIX}/`));
        const isShortcut = idx < 5 || route.path === '/settings';
        const shortcutKey = route.path === '/settings' ? ',' : String(idx + 1);

        // Agent 项特殊处理：显示折叠箭头 + 子菜单
        if (route.path === '/agents') {
          // 判断 Agent 父菜单是否激活（任意 /agents* 路径）
          const isAgentActive = location.pathname.startsWith(`${SPARKFOX_PREFIX}/agents`);
          return (
            <React.Fragment key={route.path}>
              <button
                onClick={() => {
                  toggleAgentMenu();
                  if (!agentMenuExpanded) {
                    navigate(fullPath);
                  }
                }}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  width: '100%',
                  padding: '8px 12px',
                  border: 'none',
                  borderRadius: 'var(--sf-radius-small, 6px)',
                  background: isAgentActive ? `${primaryColor}1A` : 'transparent',
                  color: isAgentActive ? primaryColor : 'var(--sf-color-text, #1D1D1F)',
                  cursor: 'pointer',
                  fontSize: 14,
                  fontWeight: isAgentActive ? 500 : 400,
                  textAlign: 'left',
                  transition: 'background 0.15s ease',
                  fontFamily: 'inherit',
                }}
                onMouseEnter={(e) => {
                  if (!isAgentActive) e.currentTarget.style.background = 'var(--sf-color-bg-secondary, #F5F5F7)';
                }}
                onMouseLeave={(e) => {
                  if (!isAgentActive) e.currentTarget.style.background = 'transparent';
                }}
              >
                <Icon name={route.icon} size={18} />
                <span style={{ flex: 1 }}>{route.label}</span>
                <span style={{ fontSize: 10, color: 'var(--sf-color-text-tertiary, #AEAEB2)' }}>
                  {agentMenuExpanded ? '▾' : '▸'}
                </span>
              </button>

              {/* Agent 子菜单（折叠展开） */}
              {agentMenuExpanded && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 2, paddingLeft: 16 }}>
                  {agentSubMenu.map((sub) => {
                    const subFullPath = `${SPARKFOX_PREFIX}${sub.path}`;
                    const isSubActive =
                      location.pathname === subFullPath ||
                      (sub.path === '/agents' && (location.pathname === subFullPath || location.pathname === `${subFullPath}/`));
                    return (
                      <button
                        key={sub.path}
                        onClick={() => navigate(subFullPath)}
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          gap: 8,
                          width: '100%',
                          padding: '6px 10px',
                          border: 'none',
                          borderRadius: 'var(--sf-radius-small, 6px)',
                          background: isSubActive ? `${primaryColor}1A` : 'transparent',
                          color: isSubActive ? primaryColor : 'var(--sf-color-text-secondary, #515154)',
                          cursor: 'pointer',
                          fontSize: 13,
                          fontWeight: isSubActive ? 500 : 400,
                          textAlign: 'left',
                          fontFamily: 'inherit',
                        }}
                      >
                        <span style={{ fontSize: 14 }}>{sub.icon}</span>
                        <span>{sub.label}</span>
                      </button>
                    );
                  })}
                </div>
              )}
            </React.Fragment>
          );
        }

        return (
          <button
            key={route.path}
            onClick={() => navigate(fullPath)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 10,
              width: '100%',
              padding: '8px 12px',
              border: 'none',
              borderRadius: 'var(--sf-radius-small, 6px)',
              background: isActive ? `${primaryColor}1A` : 'transparent',
              color: isActive ? primaryColor : 'var(--sf-color-text, #1D1D1F)',
              cursor: 'pointer',
              fontSize: 14,
              fontWeight: isActive ? 500 : 400,
              textAlign: 'left',
              transition: 'background 0.15s ease',
              fontFamily: 'inherit',
            }}
            onMouseEnter={(e) => {
              if (!isActive) e.currentTarget.style.background = 'var(--sf-color-bg-secondary, #F5F5F7)';
            }}
            onMouseLeave={(e) => {
              if (!isActive) e.currentTarget.style.background = 'transparent';
            }}
          >
            <Icon name={route.icon} size={18} />
            <span style={{ flex: 1 }}>{route.label}</span>
            {isShortcut && (
              <kbd
                style={{
                  fontSize: 11,
                  color: 'var(--sf-color-text-tertiary, #AEAEB2)',
                  background: 'var(--sf-color-bg-secondary, #F5F5F7)',
                  padding: '2px 6px',
                  borderRadius: 4,
                  fontFamily: 'inherit',
                }}
              >
                ⌘{shortcutKey}
              </kbd>
            )}
          </button>
        );
      })}

      {/* 知识库入口（F1）—— 路径为 /knowledge，独立于 /sparkfox/* 路由 */}
      {(() => {
        const knowledgePath = '/knowledge';
        const isKnowledgeActive =
          location.pathname === knowledgePath || location.pathname.startsWith(`${knowledgePath}/`);
        return (
          <button
            onClick={() => navigate(knowledgePath)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 10,
              width: '100%',
              padding: '8px 12px',
              border: 'none',
              borderRadius: 'var(--sf-radius-small, 6px)',
              background: isKnowledgeActive ? `${primaryColor}1A` : 'transparent',
              color: isKnowledgeActive ? primaryColor : 'var(--sf-color-text, #1D1D1F)',
              cursor: 'pointer',
              fontSize: 14,
              fontWeight: isKnowledgeActive ? 500 : 400,
              textAlign: 'left',
              transition: 'background 0.15s ease',
              fontFamily: 'inherit',
            }}
            onMouseEnter={(e) => {
              if (!isKnowledgeActive) e.currentTarget.style.background = 'var(--sf-color-bg-secondary, #F5F5F7)';
            }}
            onMouseLeave={(e) => {
              if (!isKnowledgeActive) e.currentTarget.style.background = 'transparent';
            }}
          >
            <Icon name='Book' size={18} />
            <span style={{ flex: 1 }}>知识库</span>
          </button>
        );
      })()}

      {/* 底部占位 */}
      <div style={{ flex: 1 }} />
      <div
        style={{
          padding: '8px 12px',
          fontSize: 11,
          color: 'var(--sf-color-text-tertiary, #AEAEB2)',
          borderTop: '1px solid var(--sf-color-border, #D2D2D7)',
        }}
      >
        SparkFox v0.2.28 · Phase 0 骨架
      </div>
    </nav>
  );
};

export default SparkFoxSider;
