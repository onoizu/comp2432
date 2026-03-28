//! Thread-safe queue with two priority levels.
//!
//! Urgent tasks are always popped before normal tasks. If the queue is empty,
//! workers wait until a task arrives or shutdown begins.
//! This is the "task queue" part of the Project-B minimal scope.
//!
//! Lock order: 1 (TaskQueue < ZoneManager < HealthMonitor < EventLog < StepGate).
//!
//! Deadlock-related rule: while holding this queue's [`Mutex`], do not sleep,
//! join, block on I/O, take other subsystem locks, or do heavy work. The only
//! blocking wait allowed here is [`Condvar::wait`] / [`Condvar::wait_timeout`]
//! on this queue's condvar (they release the mutex during the wait). After a
//! timed wake, [`pop_task_blocking_with_heartbeat`] drops the guard before
//! calling `on_wait` so heartbeat can update the health monitor without holding
//! the queue mutex.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use crate::task::Task;
use crate::traits::TaskProvider;
use crate::types::{TaskId, TaskKind, TaskPriority, ZoneId, DEFAULT_HEARTBEAT_INTERVAL_MS};



/// Per-task metadata exposed in queue snapshots.
#[derive(Debug, Clone)]
pub struct QueuedTaskInfo {
    pub id: TaskId,
    pub priority: TaskPriority,
    pub kind: TaskKind,
    pub target_zone: ZoneId,
}

/// Immutable queue snapshot for read-only dashboard usage.
#[derive(Debug, Clone)]
pub struct TaskQueueSnapshot {
    pub urgent_count: usize,
    pub normal_count: usize,
    pub total_count: usize,
    pub total_pushed: usize,
    pub shutdown: bool,
    pub tasks: Vec<QueuedTaskInfo>,
}

/// Internal queue state protected by a mutex.
struct TaskQueueInner {
    urgent: VecDeque<Task>,
    normal: VecDeque<Task>,
    total_pushed: usize,
    shutdown: bool,
}

/// Shared queue used by all worker threads.
pub struct TaskQueue {
    inner: Mutex<TaskQueueInner>,
    condvar: Condvar,
}

impl TaskQueue {
    /// Creates an empty queue.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(TaskQueueInner {
                urgent: VecDeque::new(),
                normal: VecDeque::new(),
                total_pushed: 0,
                shutdown: false,
            }),
            condvar: Condvar::new(),
        }
    }

    /// Return an immutable snapshot of current queue state.
    pub fn snapshot(&self) -> TaskQueueSnapshot {
        let guard = self.inner.lock().expect("task queue lock poisoned");
        let urgent_count = guard.urgent.len();
        let normal_count = guard.normal.len();
        let tasks: Vec<QueuedTaskInfo> = guard
            .urgent
            .iter()
            .chain(guard.normal.iter())
            .map(|t| QueuedTaskInfo {
                id: t.id,
                priority: t.priority,
                kind: t.kind,
                target_zone: t.target_zone,
            })
            .collect();
        TaskQueueSnapshot {
            urgent_count,
            normal_count,
            total_count: urgent_count + normal_count,
            total_pushed: guard.total_pushed,
            shutdown: guard.shutdown,
            tasks,
        }
    }




    /// OS concept: priority-based task yielding trigger.
    ///
    /// Robots executing preemptible Normal tasks periodically call this to
    /// decide whether to cooperatively yield. A `true` return means at least
    /// one Urgent task is waiting, and the caller should consider yielding.
    /// Returns `true` if the urgent queue contains pending tasks.
    pub fn has_urgent_pending(&self) -> bool {
        let guard = self.inner.lock().expect("task queue lock poisoned");
        !guard.urgent.is_empty()
    }



    /// Put a task back at the front of its priority queue (reclamation).
    /// Does NOT increment `total_pushed` since the task was already counted.
    pub fn reclaim_task(&self, task: Task) {
        let mut guard = self.inner.lock().expect("task queue lock poisoned");
        match task.priority {
            TaskPriority::Urgent => guard.urgent.push_front(task),
            TaskPriority::Normal => guard.normal.push_front(task),
        }
        self.condvar.notify_one();
    }



    /// Block until a task is available or the queue is shut down.
    /// While waiting, calls `on_wait` periodically so the caller can send
    /// heartbeats and avoid being marked offline.
    ///
    /// `on_wait` runs only after the queue mutex is released, so it may take
    /// other locks (e.g. the health monitor) without nesting inside this one.
    pub fn pop_task_blocking_with_heartbeat<F>(&self, mut on_wait: F) -> Option<Task>
    where
        F: FnMut(),
    {
        let timeout = Duration::from_millis(DEFAULT_HEARTBEAT_INTERVAL_MS);
        loop {
            let mut guard = self.inner.lock().expect("task queue lock poisoned");
            if let Some(task) = guard.urgent.pop_front() {
                return Some(task);
            }
            if let Some(task) = guard.normal.pop_front() {
                return Some(task);
            }
            if guard.shutdown {
                return None;
            }
            let (g, _) = self
                .condvar
                .wait_timeout(guard, timeout)
                .expect("task queue lock poisoned");
            drop(g);
            on_wait();
        }
    }
}




impl TaskProvider for TaskQueue {
    fn push_task(&self, task: Task) {
        let mut guard = self.inner.lock().expect("task queue lock poisoned");
        guard.total_pushed += 1;
        match task.priority {
            TaskPriority::Urgent => guard.urgent.push_back(task),
            TaskPriority::Normal => guard.normal.push_back(task),
        }
        self.condvar.notify_one();
    }

    fn pop_task_blocking(&self) -> Option<Task> {
        let mut guard = self.inner.lock().expect("task queue lock poisoned");
        loop {
            if let Some(task) = guard.urgent.pop_front() {
                return Some(task);
            }
            if let Some(task) = guard.normal.pop_front() {
                return Some(task);
            }
            if guard.shutdown {
                return None;
            }
            guard = self.condvar.wait(guard).expect("task queue lock poisoned");
        }
    }

    fn shutdown(&self) {
        let mut guard = self.inner.lock().expect("task queue lock poisoned");
        guard.shutdown = true;
        self.condvar.notify_all();
    }

    fn pending_count(&self) -> usize {
        let guard = self.inner.lock().expect("task queue lock poisoned");
        guard.urgent.len() + guard.normal.len()
    }
}
