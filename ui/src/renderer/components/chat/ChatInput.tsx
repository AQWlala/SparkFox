/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * ChatInput — 对话输入框组件
 *
 * 来源：BaiLongma src/ui/brain-ui/chat.js 的 autoGrowInput / idlePlaceholder /
 *       collectClipboardImageFiles / renderPastedImages（清洁室重写为 React + TS）
 *
 * 保留 BaiLongma 特性：
 * - 自适应输入框高度（autoGrowInput：reset → scrollHeight）
 * - 空闲占位符切换（聚焦时 DEFAULT_INPUT_PLACEHOLDER，未聚焦时 PUSH_TO_TALK_PLACEHOLDER）
 * - 粘贴图片（collectClipboardImageFiles + 去重 + 大小校验 + dataUrl 预览）
 * - 附件预览栏（粘贴图片缩略图 + 移除按钮）
 * - Enter 发送 / Shift+Enter 换行
 * - 输入锁定状态（isLocked + lockReason）
 *
 * 移除 BaiLongma 特性：
 * - 按住空格说话（PUSH_TO_TALK_PLACEHOLDER 文案保留但标注"即将支持"）
 * - 音频上下文 / 激活预热锁（已在 chatStore 移除）
 */

import React, { useCallback, useEffect, useRef } from 'react';
import {
  useChatStore,
  MAX_PASTED_IMAGE_BYTES,
  PUSH_TO_TALK_PLACEHOLDER,
  DEFAULT_INPUT_PLACEHOLDER,
  type ChatAttachment,
} from '@renderer/store/chatStore';

interface ChatInputProps {
  onSend: (text: string, attachments: ChatAttachment[]) => void;
}

const ChatInput: React.FC<ChatInputProps> = ({ onSend }) => {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // 选择性订阅 input 状态字段（避免不必要重渲染）
  const value = useChatStore((s) => s.input.value);
  const isLocked = useChatStore((s) => s.input.isLocked);
  const lockReason = useChatStore((s) => s.input.lockReason);
  const isFocused = useChatStore((s) => s.input.isFocused);
  const attachments = useChatStore((s) => s.input.attachments);

  const setInputValue = useChatStore((s) => s.setInputValue);
  const setInputFocused = useChatStore((s) => s.setInputFocused);
  const addAttachment = useChatStore((s) => s.addAttachment);
  const removeAttachment = useChatStore((s) => s.removeAttachment);
  const clearAttachments = useChatStore((s) => s.clearAttachments);

  // ─── 自适应输入框高度（autoGrowInput） ───
  const autoGrow = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = `${el.scrollHeight}px`;
  }, []);

  useEffect(() => {
    autoGrow();
  }, [value, autoGrow]);

  // ─── 占位符切换（idlePlaceholder） ───
  const placeholder = isLocked
    ? lockReason || '系统准备中…'
    : isFocused
      ? DEFAULT_INPUT_PLACEHOLDER
      : PUSH_TO_TALK_PLACEHOLDER;

  // ─── 发送 ───
  const handleSend = useCallback(() => {
    const text = value.trim();
    if (!text && attachments.length === 0) return;
    if (isLocked) return;
    onSend(text, attachments);
    setInputValue('');
    clearAttachments();
  }, [value, attachments, isLocked, onSend, setInputValue, clearAttachments]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      // Enter 发送，Shift+Enter 换行；输入法组合状态不触发
      if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend]
  );

  // ─── 粘贴图片（collectClipboardImageFiles + 去重 + 大小校验） ───
  const handlePaste = useCallback(
    async (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
      const data = e.clipboardData;
      if (!data) return;

      const files: File[] = [];
      const seen = new Set<string>();
      const pushFile = (file: File | null | undefined) => {
        if (!file || !String(file.type || '').startsWith('image/')) return;
        const key = `${file.name}:${file.type}:${file.size}:${file.lastModified}`;
        if (seen.has(key)) return;
        seen.add(key);
        files.push(file);
      };

      for (const item of Array.from(data.items || [])) {
        if (item?.kind === 'file' && String(item.type || '').startsWith('image/')) {
          pushFile(item.getAsFile());
        }
      }
      for (const file of Array.from(data.files || [])) pushFile(file);

      if (files.length === 0) return;
      e.preventDefault();

      for (const file of files) {
        if (file.size > MAX_PASTED_IMAGE_BYTES) continue;
        const dataUrl = await new Promise<string>((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => resolve(String(reader.result || ''));
          reader.onerror = () => reject(reader.error);
          reader.readAsDataURL(file);
        });
        const attachment: ChatAttachment = {
          id: `paste-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
          type: 'image',
          name: file.name || 'pasted-image',
          mime: file.type,
          size: file.size,
          dataUrl,
        };
        addAttachment(attachment);
      }
    },
    [addAttachment]
  );

  const canSend = !isLocked && (value.trim().length > 0 || attachments.length > 0);

  return (
    <div className='sparkfox-chat-input-area'>
      {attachments.length > 0 && (
        <div className='sparkfox-attachment-preview'>
          {attachments.map((a) => (
            <div key={a.id} className='sparkfox-attachment-item'>
              {a.dataUrl && <img src={a.dataUrl} alt={a.name} />}
              <button
                type='button'
                className='sparkfox-attachment-remove'
                onClick={() => removeAttachment(a.id)}
                aria-label='移除附件'
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}
      <div className='sparkfox-chat-input-wrapper'>
        <textarea
          ref={textareaRef}
          className='sparkfox-chat-input'
          rows={1}
          placeholder={placeholder}
          value={value}
          disabled={isLocked}
          onChange={(e) => setInputValue(e.target.value)}
          onFocus={() => setInputFocused(true)}
          onBlur={() => setInputFocused(false)}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          autoComplete='off'
        />
        <button
          type='button'
          className='sparkfox-send-btn'
          onClick={handleSend}
          disabled={!canSend}
          aria-label='发送'
        >
          ↑
        </button>
      </div>
    </div>
  );
};

ChatInput.displayName = 'ChatInput';

export default ChatInput;
