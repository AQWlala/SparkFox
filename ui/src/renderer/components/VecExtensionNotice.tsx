/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * VecExtensionNotice —— sqlite-vec 缺失提示（F6）
 *
 * 当后端检测到 sqlite-vec 扩展未加载时，前端展示此降级提示：
 * - 警告样式（Alert type='warning'）
 * - 提供"下载"按钮跳转到 sqlite-vec 官方 releases 页面
 *
 * 与 spec 2161-2172 行一致。
 */

import { Alert, Button } from '@arco-design/web-react';

export function VecExtensionNotice() {
  return (
    <Alert
      type='warning'
      title='向量检索不可用'
      content='sqlite-vec 扩展未加载，语义检索功能降级。请下载 sqlite_vec.dll 放置到 %APPDATA%\\sparkfox\\sqlite-vec\\'
      action={
        <Button
          size='mini'
          onClick={() => window.open('https://github.com/asg017/sqlite-vec/releases')}
        >
          下载
        </Button>
      }
    />
  );
}

export default VecExtensionNotice;
