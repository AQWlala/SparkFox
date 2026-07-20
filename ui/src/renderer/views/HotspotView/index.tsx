/**
 * SparkFox HotspotView — 信息热点追踪主视图
 *
 * 来源：BaiLongma src/ui/brain-ui/hotspot.js（清洁室重写为 React）
 *
 * 组合：
 * - 顶部工具栏（标题 + 状态 + 刷新 + 模式开关 + 时钟）
 * - 演示注入面板（输入用户消息 → 匹配热点 + 构建中性上下文）
 * - 主体：左 4 平台热榜 + 右 实时事件流
 * - 底部跑马灯
 *
 * 接口预留：
 * - buildContext(message) → 注入到对话 Agent
 * - matchUserMessage(message) → 匹配热点
 * - 注入到 memoryStore.injectFromConversation（PoC 暂不接入，仅预留）
 */

import { useEffect, useState } from 'react';
import { Button, Input, Switch, Tooltip } from '@arco-design/web-react';
import { IconRefresh, IconBulb } from '@arco-design/web-react/icon';
import HotspotPlatformLists from '@renderer/components/hotspot/HotspotPlatformLists';
import HotspotFeed from '@renderer/components/hotspot/HotspotFeed';
import HotspotTicker from '@renderer/components/hotspot/HotspotTicker';
import HotspotClock from '@renderer/components/hotspot/HotspotClock';
import { useHotspotStore } from '@renderer/store/hotspotStore';
import '@renderer/components/hotspot/hotspot.css';

function fmtFetchedAt(value: string | null): string {
  if (!value) return '未抓取';
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return '未知';
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

export default function HotspotView() {
  const initialized = useHotspotStore((s) => s.initialized);
  const loading = useHotspotStore((s) => s.loading);
  const meta = useHotspotStore((s) => s.meta);
  const panelActive = useHotspotStore((s) => s.panelActive);
  const initialize = useHotspotStore((s) => s.initialize);
  const refresh = useHotspotStore((s) => s.refresh);
  const setPanelActive = useHotspotStore((s) => s.setPanelActive);
  const matchUserMessage = useHotspotStore((s) => s.matchUserMessage);
  const buildContext = useHotspotStore((s) => s.buildContext);

  // 演示输入
  const [demoInput, setDemoInput] = useState('');
  const [demoResult, setDemoResult] = useState<{ matches: number; context: string } | null>(null);

  // 初始化（首次进入视图时）
  useEffect(() => {
    initialize();
  }, [initialize]);

  // 切换面板模式时同步上下文激活
  useEffect(() => {
    setPanelActive(panelActive, 'view');
    // 仅在 panelActive 变化时触发，避免重复
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleDemoRun = () => {
    const msg = demoInput.trim();
    if (!msg) {
      setDemoResult({ matches: 0, context: '' });
      return;
    }
    const matches = matchUserMessage(msg);
    const context = buildContext(msg);
    setDemoResult({ matches: matches.length, context });
  };

  const handleRefresh = () => {
    refresh();
  };

  const metaClass = loading ? 'loading' : meta.stale ? 'stale' : '';
  const metaText = loading
    ? '抓取中…'
    : !initialized
    ? '未初始化'
    : `${meta.source === 'mock' ? 'Mock 数据' : '已抓取'} · ${fmtFetchedAt(meta.fetchedAt)}`;

  return (
    <div className='hotspot-view'>
      {/* 顶部工具栏 */}
      <div className='hotspot-toolbar'>
        <div className='hotspot-toolbar-left'>
          <div className='hotspot-title'>信息热点追踪</div>
          <Tooltip content={metaText}>
            <span className={`hotspot-meta ${metaClass}`}>
              <span className='hotspot-meta-dot' />
              {metaText}
            </span>
          </Tooltip>
        </div>
        <div className='hotspot-toolbar-right'>
          <Tooltip content='开启后，热点上下文会注入到对话 Agent（中性上下文，不强制回复）'>
            <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6, fontSize: 12, color: '#8e8e93' }}>
              上下文注入
              <Switch
                size='small'
                checked={panelActive}
                onChange={(v) => setPanelActive(v, 'user')}
              />
            </span>
          </Tooltip>
          <Button
            size='small'
            type='primary'
            icon={<IconRefresh />}
            loading={loading}
            onClick={handleRefresh}
          >
            刷新
          </Button>
          <HotspotClock />
        </div>
      </div>

      {/* 演示注入面板 */}
      <div className='hotspot-demo' style={{ marginTop: 16 }}>
        <span className='hotspot-demo-label'>
          <IconBulb /> 演示
        </span>
        <Input
          className='hotspot-demo-input'
          placeholder='输入用户消息，例如"刚看到热搜第一是神舟十八号，你怎么看？"'
          value={demoInput}
          onChange={setDemoInput}
          onPressEnter={handleDemoRun}
          size='small'
        />
        <Button size='small' type='primary' onClick={handleDemoRun}>
          匹配
        </Button>
        {demoResult && (
          <span className={`hotspot-demo-result${demoResult.matches > 0 ? ' match' : ''}`}>
            {demoResult.matches > 0
              ? `命中 ${demoResult.matches} 条热点`
              : '无命中（仅注入上下文）'}
          </span>
        )}
      </div>

      {/* 上下文预览 */}
      {demoResult?.context && (
        <div className='hotspot-context-preview'>{demoResult.context}</div>
      )}

      {/* 主体：左 4 平台热榜 + 右 实时事件流 */}
      <div className='hotspot-body'>
        <HotspotPlatformLists />
        <HotspotFeed />
      </div>

      {/* 底部跑马灯 */}
      <HotspotTicker />
    </div>
  );
}
