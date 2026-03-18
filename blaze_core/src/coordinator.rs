//! Top-level coordinator that wires all subsystems together.
//!
//! The [`Coordinator`] owns the shared task queue, zone manager, health
//! monitor, and event log.  It spawns robot worker threads and a
//! background health-monitor thread, and provides a clean shutdown
//! sequence.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::event_log::{EventKind, EventLog};
use crate::health_monitor::HealthMonitor;
use crate::robot::robot_worker;
use crate::task::Task;
use crate::task_queue::TaskQueue;
use crate::traits::{HeartbeatRegistry, TaskProvider};
use crate::types::{RobotId, DEFAULT_MONITOR_INTERVAL_MS};
use crate::zone_manager::ZoneManager;

/// Orchestrates the entire Blaze system.
pub struct Coordinator {
    task_queue: Arc<TaskQueue>,
    zone_manager: Arc<ZoneManager>,
    health_monitor: Arc<HealthMonitor>,
    event_log: Arc<EventLog>,
    robot_handles: Vec<JoinHandle<()>>,
    monitor_handle: Option<JoinHandle<()>>,
    monitor_shutdown: Arc<AtomicBool>,
}

impl Coordinator {
    /// Create a new coordinator.
    ///
    /// # Arguments
    ///
    /// * `heartbeat_timeout` — How long a robot may be silent before it is
    ///   considered offline.
    pub fn new(heartbeat_timeout: Duration) -> Self {
        Self {
            task_queue: Arc::new(TaskQueue::new()),
            zone_manager: Arc::new(ZoneManager::new()),
            health_monitor: Arc::new(HealthMonitor::new(heartbeat_timeout)),
            event_log: Arc::new(EventLog::new()),
            robot_handles: Vec::new(),
            monitor_handle: None,
            monitor_shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Spawn a single robot worker thread.
    ///
    /// # Arguments
    ///
    /// * `id` — Unique robot identifier.
    /// * `fail_flag` — Optional flag for failure injection.  When set to
    ///   `true` the robot stops sending heartbeats.
    pub fn spawn_robot(&mut self, id: RobotId, fail_flag: Option<Arc<AtomicBool>>) {
        self.health_monitor.register(id);

        let tq = Arc::clone(&self.task_queue);
        let zm = Arc::clone(&self.zone_manager);
        let hm = Arc::clone(&self.health_monitor);
        let el = Arc::clone(&self.event_log);

        let handle = thread::spawn(move || {
            robot_worker(id, tq, zm, hm, el, fail_flag);
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
        let shutdown = Arc::clone(&self.monitor_shutdown);

        let handle = thread::spawn(move || {
            while !shutdown.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(DEFAULT_MONITOR_INTERVAL_MS));
                let timed_out = hm.check_timeouts();
                for robot_id in timed_out {
                    el.log(EventKind::RobotTimedOut { robot_id });
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

    /// Return a reference-counted handle to the health monitor.
    pub fn health_monitor(&self) -> Arc<HealthMonitor> {
        Arc::clone(&self.health_monitor)
    }

    /// Return a reference-counted handle to the task queue.
    pub fn task_queue(&self) -> Arc<TaskQueue> {
        Arc::clone(&self.task_queue)
    }
}
