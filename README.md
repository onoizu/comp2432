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

## Key Concepts / 核心概念

| Concept / 概念 | Implementation / 实现 |
|----------------|------------------------|
| Concurrency control / 并发控制 | Multiple robot worker threads safely share state via `Arc<Mutex<T>>`. 多个机器人工作线程通过 `Arc<Mutex<T>>` 安全共享状态。 |
| Synchronization / 同步 | `Condvar`-based blocking on task queue and zone access. 基于 `Condvar` 的任务队列与区域访问阻塞。 |
| Coordination / 协作 | Coordinator orchestrates robot lifecycle, task dispatch, and health monitoring. 协调器负责机器人生命周期、任务分发与健康监控。 |

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

## Testing / 测试

```sh
cargo test -p blaze_core
```

---

## License

MIT or as specified by your course.
