/**
 * @license
 * Copyright 2025-2026 NomiFun (nomifun.com) — Apache-2.0 (original)
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only (modifications)
 *
 * Modified by SparkFox Contributors, 2026:
 * - primaryColor: NomiFun gray (#4E5969) → Apple system blue (#007AFF)
 * - Load SparkFox Apple theme presets (macOS Light/Dark/Auto)
 * - Inject SparkFox global CSS variables for Apple system style
 */

// Runtime patches must be imported early
import './utils/ui/runtimePatches';

// Browser adapter setup
import '@/common/adapter/browser';

// React and core dependencies
import type { PropsWithChildren } from 'react';
import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';

// Context providers
import { AuthProvider } from './hooks/context/AuthContext';
import { FeedbackProvider } from './hooks/context/FeedbackContext';
import { ThemeProvider } from './hooks/context/ThemeContext';

// Arco Design
import { ConfigProvider } from '@arco-design/web-react';
// Configure Arco Design to use React 18's createRoot, fixing Message component's CopyReactDOM.render error
import '@arco-design/web-react/es/_util/react-19-adapter';
import '@arco-design/web-react/dist/css/arco.css';
import enUS from '@arco-design/web-react/es/locale/en-US';
import zhCN from '@arco-design/web-react/es/locale/zh-CN';
import { useTranslation } from 'react-i18next';

// Styles
import 'uno.css';
import './styles/arco-override.css';
import './styles/themes/index.css';

// SparkFox: Apple 系统风格主题注入（Step 0.4）
import './styles/sparkfox-apple.css';

// Config service — kick off initialization before i18n / theme modules load,
// so their startup paths (which await configService.whenReady()) observe the
// authoritative settings from the backend instead of the empty cache.
import { configService } from '@/common/config/configService';
import { application } from '@/common/adapter/ipcBridge';
import { isHandledAuthExpiredHttpError } from '@/common/adapter/httpBridge';
import { setBrowserStorageGeneration } from '@/common/utils/browserStorageKey';
configService.initialize().catch((err) => {
  console.error('Failed to initialize config:', err);
});

// i18n
import './services/i18n';
import { registerPwa } from './services/registerPwa';

import { mutate as swrMutate } from 'swr';
import { DETECTED_AGENTS_SWR_KEY, fetchDetectedAgents } from './utils/model/agentTypes';
import { repairAllCronJobTimeZonesOnce } from '@renderer/pages/cron/repairCronJobTimeZone';

// Components and utilities
import AppLoader from './components/layout/AppLoader';
import Layout from './components/layout/Layout';
// SparkFox: 使用 SparkFoxSider 替换 NomiFun 原 Sider（全量改造，步骤 A.2）
import SparkFoxSider from './components/layout/SparkFoxSider';
import RouteErrorBoundary from './components/layout/RouteErrorBoundary';
import Router from './components/layout/Router';
// SparkFox: NomiFun 原 Sider 已被 SparkFoxSider 替换（全量改造，步骤 A.2）
// import Sider from './components/layout/Sider';
import { useAuth } from './hooks/context/AuthContext';
import { ConversationHistoryProvider } from './hooks/context/ConversationHistoryContext';
import HOC from './utils/ui/HOC';

// SparkFox: 一次性 storage key 迁移（nomifun:* → sparkfox:*）
// 必须在 React.render 之前调用，确保所有读取 localStorage 的代码看到新 key。
import { migrateStorageKeys, migrateThemeKeys } from './utils/storageMigration';
try {
  // theme keys 优先迁移（index.html 内联脚本已读取过一次，这里同步保证后续一致）
  migrateThemeKeys();
  migrateStorageKeys();
} catch (e) {
  console.error('[storageMigration] 迁移失败:', e);
}

const arcoLocales: Record<string, typeof enUS> = {
  'zh-CN': zhCN,
  'en-US': enUS,
};

const AppProviders: React.FC<PropsWithChildren> = ({ children }) =>
  React.createElement(
    AuthProvider,
    null,
    React.createElement(ThemeProvider, null, React.createElement(FeedbackProvider, null, children))
  );

const Config: React.FC<PropsWithChildren> = ({ children }) => {
  const {
    i18n: { language },
  } = useTranslation();
  const arcoLocale = arcoLocales[language] ?? enUS;

  // SparkFox: Apple 系统蓝主色（替换 NomiFun 灰色 #4E5969）
  return React.createElement(ConfigProvider, { theme: { primaryColor: '#007AFF' }, locale: arcoLocale }, children);
};

const Main = () => {
  const { ready, status } = useAuth();
  const [configReady, setConfigReady] = useState(false);
  const [configError, setConfigError] = useState<Error | null>(null);

  useEffect(() => {
    // Browser sessions must pass the auth probe before any protected startup
    // request runs. In particular, `/api/system/info` returns 403 for an
    // expired session; starting it while unauthenticated would turn the normal
    // login transition into an application-level render failure.
    if (!ready || status !== 'authenticated') {
      setConfigReady(false);
      setConfigError(null);
      return;
    }

    let active = true;
    setConfigReady(false);
    setConfigError(null);
    // Prefetch `/api/agents` in parallel with configService.initialize() and
    // seed the shared SWR cache so the Guid page's model/mode selectors can
    // read `handshake.available_models` on the very first render — without
    // waiting for a session to be created.
    void Promise.all([
      application.systemInfo
        .invoke()
        .then((info) => setBrowserStorageGeneration(info.storageGeneration))
        .catch((err) => {
          console.error('Failed to initialize browser storage generation:', err);
          throw err;
        }),
      configService.initialize().catch((err) => {
        console.error('Failed to initialize config:', err);
      }),
      fetchDetectedAgents()
        .then((agents) => swrMutate(DETECTED_AGENTS_SWR_KEY, agents, false))
        .catch((err) => {
          console.error('Failed to prefetch agents:', err);
        }),
    ])
      .then(() => {
        if (active) setConfigReady(true);
      })
      .catch((error: unknown) => {
        // httpBridge already cleared the expired browser session and notified
        // AuthProvider. Let the auth state render `/login`; never latch this
        // expected transition into the root error boundary.
        if (!active || isHandledAuthExpiredHttpError(error)) return;
        setConfigError(error instanceof Error ? error : new Error(String(error)));
      });

    return () => {
      active = false;
    };
  }, [ready, status]);

  useEffect(() => {
    if (!ready || status !== 'authenticated') return;
    void repairAllCronJobTimeZonesOnce();
  }, [ready, status]);

  const router = (
    <Router
      layout={
        <ConversationHistoryProvider>
          <Layout sider={<SparkFoxSider />} />
        </ConversationHistoryProvider>
      }
    />
  );

  if (!ready) {
    return <AppLoader />;
  }

  // The login route is intentionally independent from authenticated startup
  // data. This also makes an in-flight session expiry recover immediately.
  if (status !== 'authenticated') {
    return router;
  }

  if (configError) {
    throw configError;
  }

  if (!configReady) {
    return <AppLoader />;
  }

  return router;
};

const App = HOC.Wrapper(Config)(Main);

void registerPwa();

// Dev only: expose stores for browser automation testing (PoC 阶段功能联调测试)
if (import.meta.env.DEV) {
  Promise.all([
    import('./store/chatStore'),
    import('./store/memoryStore'),
    import('./store/monitorStore'),
    import('./store/thinkingStore'),
    import('./store/hotspotStore'),
    import('./store/agentStore'),
  ]).then(
    ([
      chatStore,
      memoryStore,
      monitorStore,
      thinkingStore,
      hotspotStore,
      agentStore,
    ]) => {
      (window as unknown as { __sf: unknown }).__sf = {
        chatStore: chatStore.useChatStore,
        memoryStore: memoryStore.useMemoryStore,
        monitorStore: monitorStore.useMonitorStore,
        thinkingStore: thinkingStore.useThinkingStore,
        hotspotStore: hotspotStore.useHotspotStore,
        agentStore: agentStore.useAgentStore,
      };
    },
  );
}

const root = createRoot(document.getElementById('root')!);
root.render(
  <RouteErrorBoundary scope='application'>
    <AppProviders>
      <App />
    </AppProviders>
  </RouteErrorBoundary>
);
