/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * MemoryListPanel — 记忆列表（搜索 + 过滤 + 排序 + 选择 + 增删）
 *
 * 来源：OpenAkita MemoryView.tsx 列表部分（清洁室重写）
 * 功能：
 * - 搜索框（content / tags / subject / object）
 * - 类型过滤 Select
 * - 排序 Select + 升降序切换
 * - 全选 / 批量删除
 * - 单条记忆卡片（layer 色 / type 色 / 标签 / 重要性 / 访问次数 / 编辑/删除）
 * - 分页
 */

import React from 'react';
import {
  Input,
  Select,
  Button,
  Tag,
  Pagination,
  Checkbox,
} from '@arco-design/web-react';
import {
  useMemoryStore,
  LAYER_LABELS,
  LAYER_COLORS,
  TYPE_LABELS,
  TYPE_COLORS,
  PRIORITY_LABELS,
  SCOPE_LABELS,
  type MemoryLayer,
  type MemoryType,
  type SortField,
  type MemoryItem,
} from '@renderer/store/memoryStore';

const { Search: InputSearch } = Input;

const LAYER_OPTIONS: { label: string; value: MemoryLayer | 'all' }[] = [
  { label: '全部层', value: 'all' },
  { label: LAYER_LABELS.L0, value: 'L0' },
  { label: LAYER_LABELS.L1, value: 'L1' },
  { label: LAYER_LABELS.L2, value: 'L2' },
  { label: LAYER_LABELS.L3, value: 'L3' },
  { label: LAYER_LABELS.L4, value: 'L4' },
  { label: LAYER_LABELS.L5, value: 'L5' },
];

const TYPE_OPTIONS: { label: string; value: MemoryType | 'all' }[] = [
  { label: '全部类型', value: 'all' },
  ...(Object.keys(TYPE_LABELS) as MemoryType[]).map((t) => ({
    label: TYPE_LABELS[t],
    value: t,
  })),
];

const SORT_OPTIONS: { label: string; value: SortField }[] = [
  { label: '重要性', value: 'importance_score' },
  { label: '创建时间', value: 'created_at' },
  { label: '更新时间', value: 'updated_at' },
  { label: '最近访问', value: 'last_accessed_at' },
  { label: '访问次数', value: 'access_count' },
];

function fmtDate(iso: string | null): string {
  if (!iso) return '-';
  try {
    const d = new Date(iso);
    return `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
  } catch {
    return iso;
  }
}

function fmtRelative(iso: string | null): string {
  if (!iso) return '';
  try {
    const diff = Date.now() - new Date(iso).getTime();
    if (diff < 60000) return '刚刚';
    if (diff < 3600000) return `${Math.floor(diff / 60000)} 分钟前`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)} 小时前`;
    return `${Math.floor(diff / 86400000)} 天前`;
  } catch {
    return '';
  }
}

const MemoryListPanel: React.FC = () => {
  const allMemories = useMemoryStore((s) => s.memories);
  const searchQuery = useMemoryStore((s) => s.searchQuery);
  const filterLayer = useMemoryStore((s) => s.filterLayer);
  const filterType = useMemoryStore((s) => s.filterType);
  const sortBy = useMemoryStore((s) => s.sortBy);
  const sortOrder = useMemoryStore((s) => s.sortOrder);
  const page = useMemoryStore((s) => s.page);
  const pageSize = useMemoryStore((s) => s.pageSize);
  const selectedIds = useMemoryStore((s) => s.selectedIds);
  const loading = useMemoryStore((s) => s.loading);

  // 计算过滤 + 排序后的列表（useMemo 避免无限渲染）
  const filtered = React.useMemo(() => {
    let list = allMemories;
    if (filterLayer !== 'all') {
      list = list.filter((m) => m.layer === filterLayer);
    }
    if (filterType !== 'all') {
      list = list.filter((m) => m.type === filterType);
    }
    const q = searchQuery.trim().toLowerCase();
    if (q) {
      list = list.filter(
        (m) =>
          m.content.toLowerCase().includes(q) ||
          m.tags.some((t) => t.toLowerCase().includes(q)) ||
          (m.subject || '').toLowerCase().includes(q) ||
          (m.object || '').toLowerCase().includes(q),
      );
    }
    const sorted = [...list].sort((a, b) => {
      let av: number | string = '';
      let bv: number | string = '';
      if (sortBy === 'importance_score' || sortBy === 'access_count') {
        av = a[sortBy];
        bv = b[sortBy];
      } else {
        av = a[sortBy] || '';
        bv = b[sortBy] || '';
      }
      if (typeof av === 'number' && typeof bv === 'number') {
        return av - bv;
      }
      const as = String(av);
      const bs = String(bv);
      if (as < bs) return -1;
      if (as > bs) return 1;
      return 0;
    });
    return sortOrder === 'desc' ? sorted.reverse() : sorted;
  }, [allMemories, filterLayer, filterType, searchQuery, sortBy, sortOrder]);

  const total = filtered.length;
  const memories: MemoryItem[] = React.useMemo(() => {
    const start = page * pageSize;
    return filtered.slice(start, start + pageSize);
  }, [filtered, page, pageSize]);

  const setSearch = useMemoryStore((s) => s.setSearch);
  const setFilterLayer = useMemoryStore((s) => s.setFilterLayer);
  const setFilterType = useMemoryStore((s) => s.setFilterType);
  const setSort = useMemoryStore((s) => s.setSort);
  const setPage = useMemoryStore((s) => s.setPage);
  const toggleSelect = useMemoryStore((s) => s.toggleSelect);
  const selectAll = useMemoryStore((s) => s.selectAll);
  const clearSelection = useMemoryStore((s) => s.clearSelection);
  const startEdit = useMemoryStore((s) => s.startEdit);
  const deleteOne = useMemoryStore((s) => s.deleteOne);
  const batchDelete = useMemoryStore((s) => s.batchDelete);
  const accessMemory = useMemoryStore((s) => s.accessMemory);

  const handleSearch = (value: string) => setSearch(value);
  const handleSortOrderToggle = () => setSort(sortBy, sortOrder === 'asc' ? 'desc' : 'asc');
  const handleBatchDelete = async () => {
    if (selectedIds.size === 0) return;
    if (!window.confirm(`确认删除选中的 ${selectedIds.size} 条记忆？此操作不可撤销。`)) return;
    await batchDelete();
  };

  return (
    <div className='sf-memory-list-panel'>
      {/* 工具栏 */}
      <div className='sf-memory-toolbar'>
        <InputSearch
          placeholder='搜索内容 / 标签 / 实体...'
          value={searchQuery}
          onChange={handleSearch}
          allowClear
          className='sf-memory-search'
        />
        <Select
          value={filterLayer}
          onChange={(v) => setFilterLayer(v as MemoryLayer | 'all')}
          className='sf-memory-filter'
        >
          {LAYER_OPTIONS.map((o) => (
            <Select.Option key={o.value} value={o.value}>
              {o.label}
            </Select.Option>
          ))}
        </Select>
        <Select
          value={filterType}
          onChange={(v) => setFilterType(v as MemoryType | 'all')}
          className='sf-memory-filter'
        >
          {TYPE_OPTIONS.map((o) => (
            <Select.Option key={o.value} value={o.value}>
              {o.label}
            </Select.Option>
          ))}
        </Select>
        <Select
          value={sortBy}
          onChange={(v) => setSort(v as SortField, sortOrder)}
          className='sf-memory-sort'
        >
          {SORT_OPTIONS.map((o) => (
            <Select.Option key={o.value} value={o.value}>
              {o.label}
            </Select.Option>
          ))}
        </Select>
        <Button
          type='secondary'
          size='small'
          onClick={handleSortOrderToggle}
          title='切换升序/降序'
        >
          {sortOrder === 'desc' ? '↓ 降序' : '↑ 升序'}
        </Button>
        <div className='sf-memory-toolbar-spacer' />
        {selectedIds.size > 0 ? (
          <>
            <span className='sf-memory-selected-count'>已选 {selectedIds.size} 条</span>
            <Button type='secondary' size='small' onClick={clearSelection}>
              取消选择
            </Button>
            <Button status='danger' size='small' onClick={handleBatchDelete}>
              批量删除
            </Button>
          </>
        ) : (
          <Button type='secondary' size='small' onClick={selectAll} disabled={total === 0}>
            全选
          </Button>
        )}
      </div>

      {/* 列表 */}
      <div className='sf-memory-list'>
        {memories.length === 0 ? (
          <div className='sf-memory-empty'>
            {loading ? '加载中...' : '暂无符合条件的记忆'}
          </div>
        ) : (
          memories.map((m) => {
            const layerColor = LAYER_COLORS[m.layer];
            const typeColor = TYPE_COLORS[m.type];
            const isSelected = selectedIds.has(m.id);
            return (
              <div
                key={m.id}
                className={`sf-memory-row ${isSelected ? 'is-selected' : ''}`}
                style={{ '--sf-row-layer-color': layerColor } as React.CSSProperties}
                onClick={() => accessMemory(m.id)}
              >
                <div className='sf-memory-row-checkbox'>
                  <Checkbox
                    checked={isSelected}
                    onChange={() => toggleSelect(m.id)}
                    onClick={(e) => e.stopPropagation()}
                  />
                </div>
                <div className='sf-memory-row-main'>
                  <div className='sf-memory-row-header'>
                    <span
                      className='sf-memory-row-layer'
                      style={{ background: layerColor }}
                      title={LAYER_LABELS[m.layer]}
                    >
                      {m.layer}
                    </span>
                    <span
                      className='sf-memory-row-type'
                      style={{ color: typeColor, borderColor: typeColor }}
                    >
                      {TYPE_LABELS[m.type]}
                    </span>
                    {m.subject && m.predicate && m.object && (
                      <span className='sf-memory-row-triple'>
                        <strong>{m.subject}</strong>
                        <span className='sf-memory-row-predicate'>{m.predicate}</span>
                        <strong>{m.object}</strong>
                      </span>
                    )}
                    <span className='sf-memory-row-priority'>
                      {PRIORITY_LABELS[m.priority]}
                    </span>
                    <span className='sf-memory-row-scope'>
                      {SCOPE_LABELS[m.scope]}
                    </span>
                  </div>
                  <div className='sf-memory-row-content'>{m.content}</div>
                  <div className='sf-memory-row-tags'>
                    {m.tags.map((t) => (
                      <Tag key={t} size='small' color='arcoblue'>
                        {t}
                      </Tag>
                    ))}
                  </div>
                  <div className='sf-memory-row-meta'>
                    <span title='重要性'>
                      ★ {(m.importance_score * 100).toFixed(0)}%
                    </span>
                    <span title='置信度'>
                      ✓ {(m.confidence * 100).toFixed(0)}%
                    </span>
                    <span title='访问次数'>↻ {m.access_count}</span>
                    <span title='来源'>📎 {m.source}</span>
                    <span title='创建时间'>+ {fmtDate(m.created_at)}</span>
                    <span title='最近访问'>↻ {fmtRelative(m.last_accessed_at)}</span>
                    {m.expires_at && (
                      <span title='过期时间' className='sf-memory-row-expires'>
                        ⏰ {fmtDate(m.expires_at)}
                      </span>
                    )}
                    {m.reflected && (
                      <span className='sf-memory-row-reflected' title='已被反思修改'>
                        ✎ 已反思
                      </span>
                    )}
                  </div>
                </div>
                <div className='sf-memory-row-actions'>
                  <Button
                    type='text'
                    size='mini'
                    onClick={(e) => {
                      e.stopPropagation();
                      startEdit(m.id);
                    }}
                  >
                    编辑
                  </Button>
                  <Button
                    type='text'
                    size='mini'
                    status='danger'
                    onClick={(e) => {
                      e.stopPropagation();
                      if (window.confirm('确认删除此条记忆？')) deleteOne(m.id);
                    }}
                  >
                    删除
                  </Button>
                </div>
              </div>
            );
          })
        )}
      </div>

      {/* 分页 */}
      {total > pageSize && (
        <div className='sf-memory-pagination'>
          <Pagination
            total={total}
            current={page + 1}
            pageSize={pageSize}
            onChange={(p) => setPage(p - 1)}
            showTotal
            size='mini'
          />
        </div>
      )}
    </div>
  );
};

export default MemoryListPanel;
