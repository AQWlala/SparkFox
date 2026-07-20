# sparkfox-ipc

> SparkFox Tauri IPC 桥 — commands + events + tick 心跳后端

## 功能

- **Tauri commands**：10 个 `#[tauri::command]` 占位命令，覆盖 6 个 store（agentStore / memoryStore / monitorStore / hotspotStore / sceneStore + ChatView）
  - `knowledge_search` / `memory_put` / `memory_get` / `memory_list`
  - `agent_list` / `agent_create`
  - `monitor_stats` / `monitor_ack`
  - `hotspot_track` / `hotspot_list`
- **Tauri events**：6 个事件载荷（`serde::Serialize`），后端 `app_handle.emit()` 推送，前端 `listen()` 订阅
  - `thought_pushed` / `citation_added` / `hotspot_updated`
  - `monitor_updated` / `memory_changed` / `agent_status`
- **Tick 心跳后端**（Task 8.7）：`Heartbeat` 周期性检查组件存活，连续 N 次失败触发重连回调；`TickResult` 通过 `tick` event 推送前端

## 架构

```
┌──────────────────────┐       commands       ┌──────────────────────┐
│  前端 React (Tauri)  │ ───────────────────► │   sparkfox-ipc       │
│                      │                      │  ┌────────────────┐  │
│  agentStore          │ ◄──── events ─────── │  │  commands.rs   │  │
│  memoryStore         │                      │  ├────────────────┤  │
│  monitorStore        │ ◄──── tick ───────── │  │  events.rs     │  │
│  hotspotStore        │                      │  ├────────────────┤  │
│  sceneStore          │                      │  │  tick.rs       │  │
│  ChatView/MemoryView │                      │  │  (Heartbeat)   │  │
└──────────────────────┘                      │  └────────────────┘  │
                                              └──────────┬───────────┘
                                                         │ 占位（Phase 1 接入）
                                                         ▼
                              ┌──────────────────────────────────┐
                              │ sparkfox-knowledge/memory/...    │
                              └──────────────────────────────────┘
```

**设计原则**：commands 为占位实现，不直接依赖业务 crate（避免循环依赖）。Phase 1 阶段由调用方注入真实业务逻辑。

**依赖**：`sparkfox-core` / `tauri` / `serde` / `serde_json` / `tokio` / `chrono` / `log` / `env_logger`

## 使用

### Command 注册（Tauri 入口）

```rust
use sparkfox_ipc::commands::*;

tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        knowledge_search,
        memory_put, memory_get, memory_list,
        agent_list, agent_create,
        monitor_stats, monitor_ack,
        hotspot_track, hotspot_list,
    ])
    .run(tauri::generate_context!())?;
```

### Event 监听（前端）

```typescript
import { listen } from '@tauri-apps/api/event';

await listen<{ metric: string; value: number }>('monitor_updated', (e) => {
  console.log('监控指标:', e.payload.metric, e.payload.value);
});
```

### Tick 心跳

```rust
use std::sync::Arc;
use sparkfox_ipc::Heartbeat;

let hb = Arc::new(Heartbeat::new(30).with_max_failures(3));
let hb_clone = Arc::clone(&hb);
tokio::spawn(async move {
    hb_clone.run(
        || Ok::<(), std::io::Error>(()),  // 真实场景：ping store/llm
        || log::warn!("触发重连"),
    ).await;
});
```

## 测试

```bash
# 单元测试（lib）— 25 个用例：commands 占位返回值 + events 序列化往返 + tick 心跳
cargo test -p sparkfox-ipc --lib
```

## 安全约束

- `#![forbid(unsafe_code)]` — 全 crate 禁用 unsafe（lib.rs / commands.rs / events.rs / tick.rs 均显式声明）
- 所有 command 返回 `Result<T, String>`，错误信息不泄露内部路径
- Event 载荷字段使用 `snake_case`，前端通过 `camelCase` 转换消费

## 许可证

AGPL-3.0-only，详见工作区根 `LICENSE`。

## 致谢

- [Tauri 2](https://tauri.app)（Apache-2.0/MIT）— desktop application framework
