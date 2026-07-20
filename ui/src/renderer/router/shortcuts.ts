/**
 * SparkFox 路由快捷键
 *
 * 来源：SparkFox 全新设计
 * 功能：Cmd/Ctrl+1~5 切换前 5 个路由，Cmd/Ctrl+, 打开设置
 *
 * 已落地（v0.1 落地，v0.2 验证通过）
 */

import { sparkfoxRoutes } from './routes';

export function registerSparkFoxShortcuts(navigate: (path: string) => void): () => void {
  const SPARKFOX_PREFIX = '/sparkfox';
  const handler = (e: KeyboardEvent) => {
    const isMod = e.metaKey || e.ctrlKey;
    if (!isMod) return;

    // Cmd/Ctrl + 1~5：前 5 个路由
    if (e.key >= '1' && e.key <= '5') {
      const idx = parseInt(e.key, 10) - 1;
      const route = sparkfoxRoutes[idx];
      if (route) {
        e.preventDefault();
        const fullPath = `${SPARKFOX_PREFIX}${route.path === '/' ? '' : route.path}`;
        navigate(fullPath);
      }
    }

    // Cmd/Ctrl + ,：设置
    if (e.key === ',') {
      e.preventDefault();
      navigate(`${SPARKFOX_PREFIX}/settings`);
    }
  };

  window.addEventListener('keydown', handler);
  return () => window.removeEventListener('keydown', handler);
}
