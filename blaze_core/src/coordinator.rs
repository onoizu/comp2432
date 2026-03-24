//! Top-level coordinator that wires all subsystems together.
//!
//! The [`Coordinator`] owns the shared task queue, zone manager, health
//! monitor, and event log.  It spawns robot worker threads and a
//! background health-monitor thread, and provides a clean shutdown
//! sequence.
//!
//! Deadlock prevention via global lock ordering.
//! Global lock order:
//! TaskQueue < ZoneManager < HealthMonitor < EventLog < StepGate
//! All components must acquire locks only in this order and never in reverse.
//!
//! Also avoid blocking or nested lock acquisition while holding any of those
//! mutexes (no sleep, join, I/O, or heavy logging in critical sections). The
//! heartbeat-aware wait helpers in `TaskQueue`, `ZoneManager`, and `StepGate`
//! drop their mutex before calling `on_wait` so liveness updates do not extend
//! unrelated critical sections.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::event_log::{EventKind, EventLog};
use crate::health_monitor::HealthMonitor;
use crate::metrics::Metrics;
use crate::robot::robot_worker;
use crate::step_gate::StepGate;
use crate::summary::{
    DashboardMetricsSummary, QueueSummary, RobotState, RobotSummary, SystemSnapshot, ZoneSummary,
};
use crate::task::Task;
use crate::task_queue::TaskQueue;
use crate::traits::{HeartbeatRegistry, TaskProvider};
use crate::types::{RobotId, RobotStatus, DEFAULT_MONITOR_INTERVAL_MS};
use crate::zone_manager::ZoneManager;

/// Orchestrates the entire Blaze system.
pub struct Coordinator {
    task_queue: Arc<TaskQueue>,
    zone_manager: Arc<ZoneManager>,
    health_monitor: Arc<HealthMonitor>,
    event_log: Arc<EventLog>,
    metrics: Arc<Metrics>,
    step_gate: Option<Arc<StepGate>>,
    robot_handles: Vec<JoinHandle<()>>,
    monitor_handle: Option<JoinHandle<()>>,
    monitor_shutdown: Arc<AtomicBool>,
}

impl Coordinator {
    /// Create a new coordinator.
    ///
    /// `heartbeat_timeout` is how long a robot may stay silent before
    /// being considered offline.
    pub fn new(heartbeat_timeout: Duration) -> Self {
        Self {
            task_queue: Arc::new(TaskQueue::new()),
            zone_manager: Arc::new(ZoneManager::new()),
            health_monitor: Arc::new(HealthMonitor::new(heartbeat_timeout)),
            event_log: Arc::new(EventLog::new()),
            metrics: Arc::new(Metrics::new()),
            step_gate: None,
            robot_handles: Vec::new(),
            monitor_handle: None,
            monitor_shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Attach an optional step gate for manual event-by-event control.
    pub fn set_step_gate(&mut self, gate: Arc<StepGate>) {
        self.step_gate = Some(gate);
    }

    /// Spawn a single robot worker thread.
    ///
    /// `id` is the robot identifier.
    /// `fail_flag` optionally injects failure. When set to true, the robot
    /// stops sending heartbeats.
    pub fn spawn_robot(&mut self, id: RobotId, fail_flag: Option<Arc<AtomicBool>>) {
        let tq = Arc::clone(&self.task_queue);
        let zm = Arc::clone(&self.zone_manager);
        let hm = Arc::clone(&self.health_monitor);
        let el = Arc::clone(&self.event_log);
        let metrics = Arc::clone(&self.metrics);
        let step_gate = self.step_gate.clone();

        let handle = thread::spawn(move || {
            robot_worker(id, tq, zm, hm, el, metrics, fail_flag, step_gate);
        });
        self.robot_handles.push(handle);
    }

    /// Convenience: spawn `count` normal robots (IDs `0..count`, no fail
    /// flag).
    pub fn spawn_robots(&mut self, count: usize) {
        for id in 0..count {
            self.spawn_robot(id, None);
        }
    }

    /// Start the background health-monitor thread.
    ///
    /// The thread periodically calls `check_timeouts` and logs any newly
    /// offline robots.  It exits once `monitor_shutdown` is set.
    pub fn start_monitor(&mut self) {
        let hm = Arc::clone(&self.health_monitor);
        let el = Arc::clone(&self.event_log);
        let metrics = Arc::clone(&self.metrics);
        let shutdown = Arc::clone(&self.monitor_shutdown);
        let step_gate = self.step_gate.clone();

        let handle = thread::spawn(move || {
            while !shutdown.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(DEFAULT_MONITOR_INTERVAL_MS));
                let timed_out = hm.check_timeouts();
                for robot_id in timed_out {
                    if let Some(ref g) = step_gate {
                        g.wait_before_event();
                    }
                    el.log(EventKind::RobotTimedOut { robot_id });
                    metrics.record_robot_offline(robot_id);
                }
            }
        });
        self.monitor_handle = Some(handle);
    }

    /// Submit a task to the queue.
    pub fn submit_task(&self, task: Task) {
        self.task_queue.push_task(task);
    }

    /// Shut down the system gracefully.
    ///
    /// 1. Signals the monitor thread to stop.
    /// 2. Shuts down the task queue (workers drain remaining tasks then
    ///    exit).
    /// 3. Joins all worker threads and the monitor thread.
    pub fn shutdown(mut self) {
        self.monitor_shutdown.store(true, Ordering::Relaxed);
        self.task_queue.shutdown();

        for handle in self.robot_handles.drain(..) {
            let _ = handle.join();
        }
        if let Some(h) = self.monitor_handle.take() {
            let _ = h.join();
        }

        self.event_log.log(EventKind::SystemShutdown);
    }

    /// Return a reference-counted handle to the event log.
    pub fn event_log(&self) -> Arc<EventLog> {
        Arc::clone(&self.event_log)
    }

    /// Return a reference-counted handle to metrics.
    pub fn metrics(&self) -> Arc<Metrics> {
        Arc::clone(&self.metrics)
    }

    /// Return a reference-counted handle to the health monitor.
    pub fn health_monitor(&self) -> Arc<HealthMonitor> {
        Arc::clone(&self.health_monitor)
    }

    /// Return a reference-counted handle to the task queue.
    pub fn task_queue(&self) -> Arc<TaskQueue> {
        Arc::clone(&self.task_queue)
    }

    /// Return a reference-counted handle to the zone manager.
    pub fn zone_manager(&self) -> Arc<ZoneManager> {
        Arc::clone(&self.zone_manager)
    }

    /// Build a read-only snapshot for terminal dashboard rendering.
    pub fn snapshot(&self) -> SystemSnapshot {
        let queue_raw = self.task_queue.snapshot();
        let zone_raw = self.zone_manager.snapshot();
        let health_raw = self.health_monitor.snapshot();
        let metrics_raw = self.metrics.snapshot();

        let queue = QueueSummary {
            urgent_count: queue_raw.urgent_count,
            normal_count: queue_raw.normal_count,
            total_count: queue_raw.total_count,
            total_pushed: queue_raw.total_pushed,
            tasks: queue_raw.tasks,
        };

        let zones: Vec<ZoneSummary> = zone_raw
            .iter()
            .map(|z| ZoneSummary {
                zone: z.zone,
                occupant: z.occupant,
                waiting_robots: z.waiting_robots.clone(),
            })
            .collect();

        let health_map: HashMap<RobotId, _> = health_raw
            .into_iter()
            .map(|r| (r.robot_id, r))
            .collect();

        let robot_count = self.robot_handles.len();
        let robots: Vec<RobotSummary> = (0..robot_count)
            .map(|robot_id| {
                let r = match health_map.get(&robot_id) {
                    Some(h) => h,
                    None => {
                        return RobotSummary {
                            robot_id,
                            state: RobotState::Idle,
                            status: RobotStatus::Online,
                            current_task_id: None,
                            current_zone: None,
                        };
                    }
                };
                let is_waiting = zones
                    .iter()
                    .any(|z| z.waiting_robots.iter().any(|&id| id == r.robot_id));
                let state = if r.status == RobotStatus::Offline {
                    RobotState::Offline
                } else if is_waiting {
                    RobotState::WaitingZone
                } else if r.current_task.is_some() || r.current_zone.is_some() {
                    RobotState::Busy
                } else {
                    RobotState::Idle
                };
                RobotSummary {
                    robot_id: r.robot_id,
                    state,
                    status: r.status,
                    current_task_id: r.current_task,
                    current_zone: r.current_zone,
                }
            })
            .collect();

        SystemSnapshot {
            queue,
            zones,
            robots,
            metrics: DashboardMetricsSummary::from(&metrics_raw),
        }
    }
}
