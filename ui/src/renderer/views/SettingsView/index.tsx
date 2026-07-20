/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * SparkFox SettingsView — 设置页主视图
 *
 * 来源：SparkFox 全新设计（参考 macOS 系统偏好设置）
 *
 * 功能：
 * - 外观（主题预设 / 主色 / 字体大小 / 密度 / 减少动画）
 * - 快捷键映射（启用 / 查看）
 * - 数据管理（导出 / 导入 / 重置 / 清除全部数据）
 * - 关于（版本 / 协议 / 致谢）
 */

import React, { useState } from 'react';
import { Button, Message, Switch, Select, Modal } from '@arco-design/web-react';
import { useSettingsStore, type ThemePreset, type FontSize, type Density } from '@renderer/store/settingsStore';
import '@renderer/components/agent/agent.css';

const { Option } = Select;

const PRIMARY_COLOR_PRESETS = [
  { label: '系统蓝', value: '#007AFF' },
  { label: '系统紫', value: '#5856D6' },
  { label: '系统橙', value: '#FF9500' },
  { label: '系统绿', value: '#34C759' },
  { label: '系统红', value: '#FF3B30' },
  { label: '系统粉', value: '#FF2D55' },
];

const SettingsView: React.FC = () => {
  const initialize = useSettingsStore((s) => s.initialize);
  const themePreset = useSettingsStore((s) => s.themePreset);
  const setThemePreset = useSettingsStore((s) => s.setThemePreset);
  const primaryColor = useSettingsStore((s) => s.primaryColor);
  const setPrimaryColor = useSettingsStore((s) => s.setPrimaryColor);
  const fontSize = useSettingsStore((s) => s.fontSize);
  const setFontSize = useSettingsStore((s) => s.setFontSize);
  const density = useSettingsStore((s) => s.density);
  const setDensity = useSettingsStore((s) => s.setDensity);
  const reduceMotion = useSettingsStore((s) => s.reduceMotion);
  const setReduceMotion = useSettingsStore((s) => s.setReduceMotion);
  const shortcuts = useSettingsStore((s) => s.shortcuts);
  const updateShortcut = useSettingsStore((s) => s.updateShortcut);
  const exportSettings = useSettingsStore((s) => s.exportSettings);
  const importSettings = useSettingsStore((s) => s.importSettings);
  const resetToDefaults = useSettingsStore((s) => s.resetToDefaults);
  const clearAllData = useSettingsStore((s) => s.clearAllData);

  const [importText, setImportText] = useState('');

  React.useEffect(() => {
    initialize();
  }, [initialize]);

  const handleExport = () => {
    const json = exportSettings();
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `sparkfox-settings-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    Message.success('已导出设置');
  };

  const handleImport = () => {
    if (!importText.trim()) {
      Message.warning('请粘贴设置 JSON');
      return;
    }
    const ok = importSettings(importText);
    if (ok) {
      Message.success('已导入设置');
      setImportText('');
    } else {
      Message.error('JSON 格式错误');
    }
  };

  const handleReset = () => {
    Modal.confirm({
      title: '重置为默认设置',
      content: '所有自定义设置将丢失，确定继续？',
      okText: '重置',
      cancelText: '取消',
      onOk: () => {
        resetToDefaults();
        Message.success('已重置为默认设置');
      },
    });
  };

  const handleClearAll = () => {
    Modal.confirm({
      title: '清除全部数据',
      content: '将清除所有对话、记忆、Agent、热点数据与设置，此操作不可恢复。',
      okText: '清除',
      okButtonProps: { status: 'danger' },
      cancelText: '取消',
      onOk: async () => {
        await clearAllData();
        Message.success('已清除全部数据');
      },
    });
  };

  return (
    <div className='sf-agent-manager-view'>
      {/* 顶部标题栏 */}
      <header className='sf-agent-manager-header'>
        <div className='sf-agent-manager-title'>
          <h1>设置</h1>
          <span className='sf-agent-manager-count'>SparkFox v0.1 PoC</span>
        </div>
        <div className='sf-agent-manager-actions'>
          <Button type='secondary' size='small' onClick={handleExport}>
            ⬇ 导出
          </Button>
        </div>
      </header>

      {/* 外观 */}
      <section className='sf-agent-system-section'>
        <h2 className='sf-agent-system-section-title'>外观</h2>
        <p className='sf-agent-system-section-desc'>
          主题、色彩与排版
        </p>

        <div className='sf-agent-system-grid'>
          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>主题预设</label>
            <Select
              value={themePreset}
              onChange={(v) => setThemePreset(v as ThemePreset)}
              style={{ width: 200 }}
              size='small'
            >
              <Option value='macosLight'>macOS 浅色</Option>
              <Option value='macosDark'>macOS 深色</Option>
              <Option value='macosAuto'>macOS 自动（跟随系统）</Option>
              <Option value='custom'>自定义</Option>
            </Select>
          </div>

          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>主色</label>
            <div className='sf-agent-system-color-picker'>
              {PRIMARY_COLOR_PRESETS.map((c) => (
                <button
                  key={c.value}
                  type='button'
                  className={`sf-agent-system-color-btn ${primaryColor === c.value ? 'active' : ''}`}
                  style={{ background: c.value }}
                  onClick={() => setPrimaryColor(c.value)}
                  aria-label={c.label}
                  title={c.label}
                />
              ))}
            </div>
          </div>

          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>字体大小</label>
            <Select
              value={fontSize}
              onChange={(v) => setFontSize(v as FontSize)}
              style={{ width: 200 }}
              size='small'
            >
              <Option value='small'>小</Option>
              <Option value='medium'>中（默认）</Option>
              <Option value='large'>大</Option>
            </Select>
          </div>

          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>密度</label>
            <Select
              value={density}
              onChange={(v) => setDensity(v as Density)}
              style={{ width: 200 }}
              size='small'
            >
              <Option value='compact'>紧凑</Option>
              <Option value='comfortable'>舒适（默认）</Option>
              <Option value='spacious'>宽松</Option>
            </Select>
          </div>

          <div className='sf-agent-system-row sf-agent-system-row-switch'>
            <label className='sf-agent-system-label'>减少动画</label>
            <Switch
              checked={reduceMotion}
              onChange={setReduceMotion}
              size='small'
            />
            <span className='sf-agent-system-hint'>减少界面过渡动画</span>
          </div>
        </div>
      </section>

      {/* 快捷键 */}
      <section className='sf-agent-system-section'>
        <h2 className='sf-agent-system-section-title'>快捷键</h2>
        <p className='sf-agent-system-section-desc'>
          全局快捷键映射
        </p>

        <div className='sf-settings-shortcut-list'>
          {shortcuts.map((s) => (
            <div key={s.id} className='sf-settings-shortcut-row'>
              <div className='sf-settings-shortcut-info'>
                <span className='sf-settings-shortcut-label'>{s.label}</span>
                <kbd className='sf-settings-shortcut-keys'>{s.keys}</kbd>
              </div>
              <Switch
                checked={s.enabled}
                onChange={(v) => updateShortcut(s.id, { enabled: v })}
                size='small'
              />
            </div>
          ))}
        </div>
      </section>

      {/* 数据管理 */}
      <section className='sf-agent-system-section'>
        <h2 className='sf-agent-system-section-title'>数据管理</h2>
        <p className='sf-agent-system-section-desc'>
          导入、导出、重置或清除数据
        </p>

        <div className='sf-agent-system-grid'>
          <div className='sf-agent-system-row'>
            <label className='sf-agent-system-label'>导入设置</label>
            <textarea
              className='sf-settings-import-textarea'
              value={importText}
              onChange={(e) => setImportText(e.target.value)}
              placeholder='粘贴之前导出的设置 JSON…'
              rows={3}
            />
          </div>

          <div className='sf-settings-data-actions'>
            <Button type='secondary' size='small' onClick={handleImport}>
              ⬆ 导入
            </Button>
            <Button type='secondary' size='small' onClick={handleReset}>
              ↺ 重置为默认
            </Button>
            <Button status='danger' size='small' onClick={handleClearAll}>
              ✕ 清除全部数据
            </Button>
          </div>
        </div>
      </section>

      {/* 关于 */}
      <section className='sf-agent-system-section'>
        <h2 className='sf-agent-system-section-title'>关于</h2>
        <p className='sf-agent-system-section-desc'>
          SparkFox 版本与协议信息
        </p>

        <div className='sf-settings-about'>
          <div className='sf-settings-about-row'>
            <span className='sf-settings-about-label'>版本</span>
            <span className='sf-settings-about-value'>v0.1.0 PoC</span>
          </div>
          <div className='sf-settings-about-row'>
            <span className='sf-settings-about-label'>协议</span>
            <span className='sf-settings-about-value'>AGPL-3.0-only</span>
          </div>
          <div className='sf-settings-about-row'>
            <span className='sf-settings-about-label'>构建</span>
            <span className='sf-settings-about-value'>
              {new Date().getFullYear()}.{String(new Date().getMonth() + 1).padStart(2, '0')}
            </span>
          </div>
          <div className='sf-settings-about-row'>
            <span className='sf-settings-about-label'>致谢</span>
            <span className='sf-settings-about-value'>
              BaiLongma · NomiFun · OpenAkita · Pangu Nebula
            </span>
          </div>
        </div>
      </section>
    </div>
  );
};

export default SettingsView;
