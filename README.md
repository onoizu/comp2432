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
| **blaze_core** | Core library: task queue, zone access, health monitor, robot worker loop, coordinator, event log, metrics, step gate, summaries. 核心库：任务队列、区域访问、健康监控、机器人循环、协调器、事件日志、指标、单步门控与汇总。 |
| **blaze_sim** | Demo scenarios (`scenarios`) and runnable demo harness / exports (`demo`). 演示场景与可运行演示、导出逻辑。 |
| **blaze_app** | Binary (`main`) and optional local web dashboard (`server` + `web/`). 可执行入口与可选本地 Web 仪表盘。 |

---

## File Hierarchy / 文件层级

| Path | Responsibility / 职责 |
|------|----------------------|
| `Cargo.toml` | Workspace root (`members`: blaze_core, blaze_sim, blaze_app; `edition = "2024"`). 工作空间根配置。 |
| `README.md` | Project documentation. 项目说明。 |
| **blaze_core/** | |
| `blaze_core/Cargo.toml` | Core crate manifest. 核心库配置。 |
| `blaze_core/src/lib.rs` | Module declarations. 模块声明。 |
| `blaze_core/src/types.rs` | RobotId, TaskId, ZoneId, TaskPriority, TaskKind, RobotStatus, timing constants. 基础类型与常量。 |
| `blaze_core/src/errors.rs` | `BlazeError` and related errors. 统一错误类型。 |
| `blaze_core/src/traits.rs` | `TaskProvider`, `ZoneAccess`, `HeartbeatRegistry`. 三个核心 trait。 |
| `blaze_core/src/task.rs` | `Task` definition and constructors (IDs, preemption flags). 任务类型与构造。 |
| `blaze_core/src/task_queue.rs` | Two-level priority queue, blocking pop with heartbeat hooks, shutdown, snapshots. 两级优先级队列与快照。 |
| `blaze_core/src/zone_manager.rs` | Zone mutual exclusion (`Mutex` + `Condvar`), FIFO waiters, timeout/heartbeat-aware enter. 区域互斥与等待队列。 |
| `blaze_core/src/health_monitor.rs` | Heartbeat registry, timeout scan, offline handling. 心跳与超时。 |
| `blaze_core/src/robot.rs` | `robot_worker` loop: tasks, zones, heartbeats, cooperative yield, reclaim paths. 机器人工作循环。 |
| `blaze_core/src/coordinator.rs` | Spawns robots and monitor, submits tasks, ties subsystems together. 顶层协调器。 |
| `blaze_core/src/event_log.rs` | `EventKind`, thread-safe log, dump / query helpers. 结构化事件日志。 |
| `blaze_core/src/metrics.rs` | Counters and runtime metrics for demos and summaries. 运行时指标。 |
| `blaze_core/src/step_gate.rs` | Optional per-event stepping for demos and UI. 单步门控。 |
| `blaze_core/src/summary.rs` | Aggregated views / reporting helpers. 汇总与报告。 |
| `blaze_core/tests/task_queue_tests.rs` | Task queue behavior tests. 任务队列测试。 |
| `blaze_core/tests/zone_manager_tests.rs` | Zone enter/leave and blocking tests. 区域测试。 |
| `blaze_core/tests/health_monitor_tests.rs` | Heartbeat and timeout tests. 健康监控测试。 |
| `blaze_core/tests/integration_demo_tests.rs` | End-to-end lifecycle tests. 集成演示测试。 |
| `blaze_core/tests/step_gate_tests.rs` | Step gate tests. 单步门控测试。 |
| `blaze_core/tests/observability_tests.rs` | Observability / log-related tests. 可观测性测试。 |
| **blaze_sim/** | |
| `blaze_sim/Cargo.toml` | Sim crate manifest. 场景库配置。 |
| `blaze_sim/src/lib.rs` | `pub mod scenarios; pub mod demo;`. 导出子模块。 |
| `blaze_sim/src/scenarios.rs` | `basic_delivery`, `zone_conflict`, `timeout_demo`, `cooperative_preemption_demo`. 任务向量工厂。 |
| `blaze_sim/src/demo.rs` | `DemoScenario`, `default_scenarios`, run + export to `output/`. 演示运行与导出。 |
| `blaze_sim/tests/demo_export_tests.rs` | Demo export integration tests. 演示导出测试。 |
| **blaze_app/** | |
| `blaze_app/Cargo.toml` | App crate manifest (depends on core + sim). 应用配置。 |
| `blaze_app/src/lib.rs` | Library root; exposes `server` for tests and reuse. 库根，暴露 `server`。 |
| `blaze_app/src/main.rs` | CLI: default terminal demo via `demo::default_scenarios`, or `--web` for dashboard. 命令行入口。 |
| `blaze_app/src/server.rs` | Local HTTP server (`std::net::TcpListener`), dashboard API + embedded static assets. 本地 HTTP 服务与仪表盘 API。 |
| `blaze_app/web/index.html` | Dashboard shell page. 仪表盘页面。 |
| `blaze_app/web/app.js` | Client-side dashboard logic. 前端脚本。 |
| `blaze_app/web/styles.css` | Dashboard styles. 样式。 |
| `blaze_app/tests/server_api_tests.rs` | HTTP / server integration tests. 服务端集成测试。 |

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
│   │   ├── event_log.rs
│   │   ├── metrics.rs
│   │   ├── step_gate.rs
│   │   └── summary.rs
│   └── tests/
│       ├── task_queue_tests.rs
│       ├── zone_manager_tests.rs
│       ├── health_monitor_tests.rs
│       ├── integration_demo_tests.rs
│       ├── step_gate_tests.rs
│       └── observability_tests.rs
├── blaze_sim/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── scenarios.rs
│   │   └── demo.rs
│   └── tests/
│       └── demo_export_tests.rs
└── blaze_app/
    ├── Cargo.toml
    ├── src/
    │   ├── lib.rs
    │   ├── main.rs
    │   └── server.rs
    ├── web/
    │   ├── index.html
    │   ├── app.js
    │   └── styles.css
    └── tests/
        └── server_api_tests.rs
```

---

## Key Concepts / 核心概念

| Concept / 概念 | Implementation / 实现 |
|----------------|------------------------|
| Concurrency control / 并发控制 | Multiple robot worker threads safely share state via `Arc` and interior mutability (`Mutex`, atomics where appropriate). 多个工作线程通过 `Arc` 与内部可变性安全共享状态。 |
| Synchronization / 同步 | `Condvar`-based blocking on task queue and zone access. 基于 `Condvar` 的任务队列与区域等待。 |
| Coordination / 协作 | Coordinator orchestrates robots, monitor thread, task dispatch, and health-driven reclaim. 协调器负责机器人、监控线程、任务分发与健康驱动的回收。 |

---

## Implemented Optimizations So Far / 当前已实现优化

| Optimization / 优化项 | Mechanism (EN / 中文) | Main Files / 对应文件 |
|---|---|---|
| Zone waiting visibility fix / Zone 等待可视化修复 | Ensure robot is added to zone waiting list before logging waiting event, so UI never misses waiting robots. / 先加入 zone 等待队列，再记录等待事件，避免前端丢失等待中的机器人显示。 | `blaze_core/src/robot.rs`, `blaze_core/src/zone_manager.rs`, `blaze_app/web/app.js` |
| Heartbeat-aware blocking waits / 心跳感知阻塞等待 | While blocked on task pop or zone enter, workers keep heartbeats so normal waiting is not misclassified as offline. / 机器人在阻塞等待任务或区域时持续发心跳，避免正常等待被误判为离线。 | `blaze_core/src/task_queue.rs`, `blaze_core/src/zone_manager.rs`, `blaze_core/src/robot.rs` |
| Two-level priority scheduling / 两级优先级调度 | `Urgent` queue is always consumed before `Normal`, providing deterministic priority order. / 始终优先消费 `Urgent` 队列，再消费 `Normal`，实现确定性优先级调度。 | `blaze_core/src/task_queue.rs`, `blaze_core/src/task.rs` |
| Cooperative priority preemption / 协作式优先级抢占 | Preemptible `Normal` tasks periodically check urgent backlog and voluntarily yield; task is requeued as restartable and urgent work runs first. / 可抢占普通任务周期检查紧急队列并主动让出；任务以可重启语义回收重排，先执行紧急任务。 | `blaze_core/src/robot.rs`, `blaze_core/src/task.rs`, `blaze_core/src/task_queue.rs`, `blaze_core/src/event_log.rs`, `blaze_sim/src/scenarios.rs`, `blaze_sim/src/demo.rs` |
| Timeout reclaim and self-healing / 超时回收与自愈 | Health monitor marks timed-out robots offline; unfinished task is reclaimed and reassigned, preventing task loss. / 健康监控将超时机器人标记离线，未完成任务自动回收并重新分配，防止任务丢失。 | `blaze_core/src/health_monitor.rs`, `blaze_core/src/robot.rs`, `blaze_core/src/task_queue.rs`, `blaze_core/src/coordinator.rs` |
| Deadlock prevention by lock ordering / 基于锁顺序的死锁预防 | A documented global lock order is enforced by convention to avoid circular wait. / 通过全局锁顺序约定避免循环等待，从而预防死锁。 | `blaze_core/src/coordinator.rs`, `blaze_core/src/zone_manager.rs`, `blaze_core/src/health_monitor.rs`, `blaze_core/src/step_gate.rs` |
| No blocking under unrelated mutex / 持锁不做阻塞与嵌套加锁 | Heartbeat `on_wait` runs only after releasing `TaskQueue`, `ZoneManager`, and `StepGate` mutexes; only `Condvar` wait on the same lock is used during waits. / 心跳回调在释放任务队列、区域与 StepGate 互斥锁之后执行；等待路径上仅使用对应 `Condvar` 的等待。 | `blaze_core/src/task_queue.rs`, `blaze_core/src/zone_manager.rs`, `blaze_core/src/step_gate.rs`, `blaze_core/src/coordinator.rs` |
| Strong observability and replay / 可观测性与可回放增强 | Structured event timeline + metrics + snapshots + step gate support debugging, demos, and exports. / 结构化事件时间线、指标、快照与单步控制提升调试、演示和导出能力。 | `blaze_core/src/event_log.rs`, `blaze_core/src/metrics.rs`, `blaze_core/src/summary.rs`, `blaze_core/src/step_gate.rs`, `blaze_sim/src/demo.rs`, `blaze_app/src/server.rs` |

---

## Building / 构建

Requires a Rust toolchain that supports **Edition 2024** (recent stable or nightly as per your `rustup`).

需要支持 **Edition 2024** 的 Rust 工具链（与 `rustup` 当前 stable 版本一致即可）。

```sh
cargo build
```

---

## Running the Demo / 运行演示

```sh
cargo run -p blaze_app
```

Runs `blaze_sim::demo::default_scenarios()` in order (e.g. Basic Parallel Scheduling, Zone Conflict, Timeout & Reclaim), prints a short summary, and writes exports under `./output`.

按顺序运行默认演示场景，打印摘要，并在 `./output` 下写入导出结果。

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
cargo test --workspace
```

Or per crate, for example:

```sh
cargo test -p blaze_core
cargo test -p blaze_app
cargo test -p blaze_sim
```
