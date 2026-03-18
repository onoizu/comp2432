//! Thread-safe two-level priority task queue.
//!
//! Urgent tasks are always dequeued before normal ones.  When both levels
//! are empty the calling thread blocks on a [`Condvar`] until new work
//! arrives or the queue is shut down.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

use crate::task::Task;
use crate::traits::TaskProvider;
use crate::types::TaskPriority;

/// Internal state protected by a [`Mutex`].
struct TaskQueueInner {
    urgent: VecDeque<Task>,
    normal: VecDeque<Task>,
    shutdown: bool,
}

/// A thread-safe, two-level priority task queue.
///
/// Workers call [`pop_task_blocking`](TaskProvider::pop_task_blocking) to
/// obtain work.  The call blocks until a task is available or the queue has
/// been shut down **and** drained.
pub struct TaskQueue {
    inner: Mutex<TaskQueueInner>,
    condvar: Condvar,
}

impl TaskQueue {
    /// Create an empty task queue.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(TaskQueueInner {
                urgent: VecDeque::new(),
                normal: VecDeque::new(),
                shutdown: false,
            }),
            condvar: Condvar::new(),
        }
    }
}

impl TaskProvider for TaskQueue {
    fn push_task(&self, task: Task) {
        let mut guard = self.inner.lock().expect("task queue lock poisoned");
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
