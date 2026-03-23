# Project Blaze

A lightweight hospital robot coordination core demonstrating concurrency,
synchronization, and safe coordination in Rust.

轻量级医院机器人协调核心，演示 Rust 中的并发、同步与安全协作。

---

## Architecture / 架构

This is a Rust workspace with three crates:

本项目为 Rust workspace，包含三个 crate：

| Crate | Role / 职责 |
|-------|------------|
| **blaze_core** | All core logic: task queue, zone access control, health monitoring, robot worker loop, coordinator. 核心逻辑：任务队列、区域访问控制、健康监控、机器人工作循环、协调器。 |
| **blaze_sim** | Demo scenario definitions (no core logic). 演示场景定义（不含核心逻辑）。 |
| **blaze_app** | Binary entry point that runs the demo scenarios. 可执行入口，运行演示场景。 |

---

## File Hierarchy / 文件层级

| Path | Responsibility / 职责 |
|------|----------------------|
| `Cargo.toml` | Workspace root. 工作空间根配置。 |
| `README.md` | Project documentation. 项目说明。 |
| **blaze_core/** | |
| `blaze_core/Cargo.toml` | Core crate manifest. 核心库配置。 |
| `blaze_core/src/lib.rs` | Module declarations and re-exports. 模块声明与导出。 |
| `blaze_core/src/types.rs` | RobotId, TaskId, ZoneId, TaskPriority, RobotStatus, TaskKind, constants. 基础类型、枚举与常量。 |
| `blaze_core/src/errors.rs` | BlazeError (ZoneNotOwned, RobotNotRegistered). 统一错误类型。 |
| `blaze_core/src/traits.rs` | TaskProvider, ZoneAccess, HeartbeatRegistry. 三个核心 trait。 |
| `blaze_core/src/task.rs` | Task struct and Task::new() with auto-increment ID. 任务定义与工厂。 |
| `blaze_core/src/task_queue.rs` | Two-level priority queue (urgent/normal), blocking pop, shutdown. 两级优先级任务队列。 |
| `blaze_core/src/zone_manager.rs` | Mutex+Condvar zone mutual exclusion, enter/leave with ownership check. 区域互斥控制。 |
| `blaze_core/src/health_monitor.rs` | Heartbeat registry, timeout detection, no auto-recovery. 心跳注册表与超时检测。 |
| `blaze_core/src/robot.rs` | robot_worker() loop: pop → enter zone → work → leave → heartbeat; fail_flag for timeout demo. 机器人工作循环。 |
| `blaze_core/src/coordinator.rs` | Top-level orchestrator: spawn robots, monitor thread, submit tasks, shutdown. 顶层协调器。 |
| `blaze_core/src/event_log.rs` | Structured EventKind enum, thread-safe log, dump(). 结构化事件日志。 |
| `blaze_core/tests/task_queue_tests.rs` | 5 tests: priority, no duplicates, blocking, shutdown, drain. 任务队列测试。 |
| `blaze_core/tests/zone_manager_tests.rs` | 4 tests: enter/leave, ownership, blocking, multi-zone. 区域管理测试。 |
| `blaze_core/tests/health_monitor_tests.rs` | 4 tests: heartbeat, timeout, no recovery, newly-offline only. 健康监控测试。 |
| `blaze_core/tests/integration_demo_tests.rs` | 3 tests: full lifecycle, zone conflict, timeout demo. 集成测试。 |
| **blaze_sim/** | |
| `blaze_sim/Cargo.toml` | Sim crate manifest. 场景库配置。 |
| `blaze_sim/src/lib.rs` | Re-exports scenarios. 导出 scenarios。 |
| `blaze_sim/src/scenarios.rs` | basic_delivery(), zone_conflict(), timeout_demo() → Vec&lt;Task&gt;. 三个演示场景。 |
| **blaze_app/** | |
| `blaze_app/Cargo.toml` | App crate manifest. 应用配置。 |
| `blaze_app/src/main.rs` | Runs all three demos sequentially, prints event log. 依次运行三个演示。 |

```
blaze/
├── Cargo.toml
├── README.md
├── blaze_core/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── types.rs
│   │   ├── errors.rs
│   │   ├── traits.rs
│   │   ├── task.rs
│   │   ├── task_queue.rs
│   │   ├── zone_manager.rs
│   │   ├── health_monitor.rs
│   │   ├── robot.rs
│   │   ├── coordinator.rs
│   │   └── event_log.rs
│   └── tests/
│       ├── task_queue_tests.rs
│       ├── zone_manager_tests.rs
│       ├── health_monitor_tests.rs
│       └── integration_demo_tests.rs
├── blaze_sim/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── scenarios.rs
└── blaze_app/
    ├── Cargo.toml
    └── src/
        └── main.rs
```

---

## Key Concepts / 核心概念

| Concept / 概念 | Implementation / 实现 |
|----------------|------------------------|
| Concurrency control / 并发控制 | Multiple robot worker threads safely share state via `Arc<Mutex<T>>`. 多个机器人工作线程通过 `Arc<Mutex<T>>` 安全共享状态。 |
| Synchronization / 同步 | `Condvar`-based blocking on task queue and zone access. 基于 `Condvar` 的任务队列与区域访问阻塞。 |
| Coordination / 协作 | Coordinator orchestrates robot lifecycle, task dispatch, and health monitoring. 协调器负责机器人生命周期、任务分发与健康监控。 |

---

## Implemented Optimizations So Far / 当前已实现优化

| Optimization / 优化项 | Mechanism (EN / 中文) | Main Files / 对应文件 |
|---|---|---|
| Zone waiting visibility fix / Zone 等待可视化修复 | Ensure robot is added to zone waiting list before logging waiting event, so UI never misses waiting robots. / 先加入 zone 等待队列，再记录等待事件，避免前端丢失等待中的机器人显示。 | `blaze_core/src/robot.rs`, `blaze_core/src/zone_manager.rs`, `blaze_app/src/web/app.js` |
| Heartbeat-aware blocking waits / 心跳感知阻塞等待 | While blocked on task pop or zone enter, workers keep heartbeats so normal waiting is not misclassified as offline. / 机器人在阻塞等待任务或区域时持续发心跳，避免正常等待被误判为离线。 | `blaze_core/src/task_queue.rs`, `blaze_core/src/zone_manager.rs`, `blaze_core/src/robot.rs` |
| Two-level priority scheduling / 两级优先级调度 | `Urgent` queue is always consumed before `Normal`, providing deterministic priority order. / 始终优先消费 `Urgent` 队列，再消费 `Normal`，实现确定性优先级调度。 | `blaze_core/src/task_queue.rs`, `blaze_core/src/task.rs` |
| Cooperative priority preemption / 协作式优先级抢占 | Preemptible `Normal` tasks periodically check urgent backlog and voluntarily yield; task is requeued as restartable and urgent work runs first. / 可抢占普通任务周期检查紧急队列并主动让出；任务以可重启语义回收重排，先执行紧急任务。 | `blaze_core/src/robot.rs`, `blaze_core/src/task.rs`, `blaze_core/src/task_queue.rs`, `blaze_core/src/event_log.rs`, `blaze_sim/src/scenarios.rs`, `blaze_sim/src/demo.rs` |
| Timeout reclaim and self-healing / 超时回收与自愈 | Health monitor marks timed-out robots offline; unfinished task is reclaimed and reassigned, preventing task loss. / 健康监控将超时机器人标记离线，未完成任务自动回收并重新分配，防止任务丢失。 | `blaze_core/src/health_monitor.rs`, `blaze_core/src/robot.rs`, `blaze_core/src/task_queue.rs`, `blaze_core/src/coordinator.rs` |
| Deadlock prevention by lock ordering / 基于锁顺序的死锁预防 | A documented global lock order is enforced by convention to avoid circular wait. / 通过全局锁顺序约定避免循环等待，从而预防死锁。 | `blaze_core/src/coordinator.rs`, `blaze_core/src/zone_manager.rs`, `blaze_core/src/health_monitor.rs`, `blaze_core/src/step_gate.rs` |
| Strong observability and replay / 可观测性与可回放增强 | Structured event timeline + metrics + snapshots + step gate support debugging, demos, and exports. / 结构化事件时间线、指标、快照与单步控制提升调试、演示和导出能力。 | `blaze_core/src/event_log.rs`, `blaze_core/src/metrics.rs`, `blaze_core/src/summary.rs`, `blaze_core/src/step_gate.rs`, `blaze_sim/src/demo.rs`, `blaze_app/src/server.rs` |

---

## Building / 构建

```sh
cargo build
```

---

## Running the Demo / 运行演示

```sh
cargo run -p blaze_app
```

Three scenarios run sequentially: Basic Delivery, Zone Conflict, Timeout Demo.

依次运行三个场景：基础配送、区域冲突、超时演示。

---

## Starting the Web UI / 启动 Web 仪表盘

```sh
cargo run -p blaze_app -- --web
```

Opens the local dashboard at `http://localhost:3000`. Choose a scenario, start/step through events, and view queue, zones, and robots in real time.

在本地 `http://localhost:3000` 打开仪表盘，可选择场景、单步执行，并实时查看任务队列、区域与机器人状态。

Optional: specify a custom port:

```sh
cargo run -p blaze_app -- --web --port 8080
```

---

## Testing / 测试

```sh
cargo test -p blaze_core
```

---

## License

MIT or as specified by your course.
