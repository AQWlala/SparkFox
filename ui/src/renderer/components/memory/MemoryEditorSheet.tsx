/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * MemoryEditorSheet — 记忆编辑抽屉
 *
 * 来源：OpenAkita MemoryView 编辑模式（清洁室重写为 Arco Design Drawer）
 * 功能：编辑 content / layer / type / priority / scope / importance / confidence / tags / 实体三元组
 */

import React from 'react';
import { Drawer, Form, Input, Select, Slider, Tag, Space, Button, Message } from '@arco-design/web-react';
const { TextArea } = Input;
const { useForm } = Form;
import {
  useMemoryStore,
  LAYER_LABELS,
  TYPE_LABELS,
  PRIORITY_LABELS,
  SCOPE_LABELS,
  type MemoryLayer,
  type MemoryType,
  type MemoryPriority,
  type MemoryScope,
} from '@renderer/store/memoryStore';

const LAYER_VALUES: MemoryLayer[] = ['L0', 'L1', 'L2', 'L3', 'L4', 'L5'];
const TYPE_VALUES: MemoryType[] = Object.keys(TYPE_LABELS) as MemoryType[];
const PRIORITY_VALUES: MemoryPriority[] = ['transient', 'short_term', 'long_term', 'permanent'];
const SCOPE_VALUES: MemoryScope[] = ['global', 'agent', 'session'];

const MemoryEditorSheet: React.FC = () => {
  const editingId = useMemoryStore((s) => s.editingId);
  const editDraft = useMemoryStore((s) => s.editDraft);
  const cancelEdit = useMemoryStore((s) => s.cancelEdit);
  const saveEdit = useMemoryStore((s) => s.saveEdit);

  const [form] = useForm();
  const [tagInput, setTagInput] = React.useState('');

  // 同步 draft 到 form
  React.useEffect(() => {
    if (editDraft) {
      form.setFieldsValue({
        content: editDraft.content || '',
        layer: editDraft.layer,
        type: editDraft.type,
        priority: editDraft.priority,
        scope: editDraft.scope,
        importance_score: editDraft.importance_score ?? 0.5,
        confidence: editDraft.confidence ?? 0.8,
        source: editDraft.source || '',
        subject: editDraft.subject || '',
        predicate: editDraft.predicate || '',
        object: editDraft.object || '',
      });
    }
  }, [editDraft, form]);

  const visible = editingId !== null && editDraft !== null;

  const handleClose = () => cancelEdit();

  const handleSave = async () => {
    try {
      const values = await form.validate();
      // 直接通过 set 把表单值写回 editDraft，再调用 saveEdit
      useMemoryStore.setState((state) => ({
        editDraft: state.editDraft
          ? {
              ...state.editDraft,
              ...values,
              importance_score: Number(values.importance_score),
              confidence: Number(values.confidence),
              subject: values.subject || undefined,
              predicate: values.predicate || undefined,
              object: values.object || undefined,
            }
          : null,
      }));
      await saveEdit();
      Message.success('记忆已保存');
    } catch (e: any) {
      Message.error(e?.message || '保存失败');
    }
  };

  const handleAddTag = () => {
    const t = tagInput.trim();
    if (!t) return;
    useMemoryStore.setState((state) => ({
      editDraft: state.editDraft
        ? { ...state.editDraft, tags: [...(state.editDraft.tags || []), t] }
        : null,
    }));
    setTagInput('');
  };

  const handleRemoveTag = (idx: number) => {
    useMemoryStore.setState((state) => ({
      editDraft: state.editDraft
        ? { ...state.editDraft, tags: (state.editDraft.tags || []).filter((_, i) => i !== idx) }
        : null,
    }));
  };

  return (
    <Drawer
      title='编辑记忆'
      visible={visible}
      onCancel={handleClose}
      width={520}
      footer={
        <Space>
          <Button onClick={handleClose}>取消</Button>
          <Button type='primary' onClick={handleSave}>
            保存
          </Button>
        </Space>
      }
    >
      {editDraft && (
        <Form form={form} layout='vertical'>
          <Form.Item label='内容' field='content' rules={[{ required: true, message: '请输入内容' }]}>
            <TextArea rows={4} placeholder='记忆内容...' />
          </Form.Item>
          <Form.Item label='层级' field='layer'>
            <Select>
              {LAYER_VALUES.map((l) => (
                <Select.Option key={l} value={l}>
                  {LAYER_LABELS[l]}
                </Select.Option>
              ))}
            </Select>
          </Form.Item>
          <Form.Item label='类型' field='type'>
            <Select>
              {TYPE_VALUES.map((t) => (
                <Select.Option key={t} value={t}>
                  {TYPE_LABELS[t]}
                </Select.Option>
              ))}
            </Select>
          </Form.Item>
          <Form.Item label='优先级' field='priority'>
            <Select>
              {PRIORITY_VALUES.map((p) => (
                <Select.Option key={p} value={p}>
                  {PRIORITY_LABELS[p]}
                </Select.Option>
              ))}
            </Select>
          </Form.Item>
          <Form.Item label='作用域' field='scope'>
            <Select>
              {SCOPE_VALUES.map((s) => (
                <Select.Option key={s} value={s}>
                  {SCOPE_LABELS[s]}
                </Select.Option>
              ))}
            </Select>
          </Form.Item>
          <Form.Item label={`重要性 ${(form.getFieldValue('importance_score') ?? 0.5) * 100 | 0}%`} field='importance_score'>
            <Slider min={0} max={1} step={0.01} />
          </Form.Item>
          <Form.Item label={`置信度 ${(form.getFieldValue('confidence') ?? 0.8) * 100 | 0}%`} field='confidence'>
            <Slider min={0} max={1} step={0.01} />
          </Form.Item>
          <Form.Item label='来源' field='source'>
            <Input placeholder='对话 / 工具 / 系统 / 手动' />
          </Form.Item>
 <Form.Item label='主体 (subject)' field='subject'>
            <Input placeholder='主体 (subject)' />
          </Form.Item>
          <Form.Item label='谓词 (predicate)' field='predicate'>
            <Input placeholder='谓词 (predicate)' />
          </Form.Item>
          <Form.Item label='客体 (object)' field='object'>
            <Input placeholder='客体 (object)' />
          </Form.Item>
          <Form.Item label='标签'>
            <div className='sf-memory-editor-tags'>
              {(editDraft.tags || []).map((t, idx) => (
                <Tag
                  key={`${t}-${idx}`}
                  closable
                  onClose={() => handleRemoveTag(idx)}
                  color='arcoblue'
                >
                  {t}
                </Tag>
              ))}
              <Input
                placeholder='输入标签后回车'
                value={tagInput}
                onChange={setTagInput}
                onPressEnter={handleAddTag}
                style={{ width: 180 }}
                size='small'
              />
            </div>
          </Form.Item>
        </Form>
      )}
    </Drawer>
  );
};

export default MemoryEditorSheet;
