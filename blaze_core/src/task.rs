//! Task definition and factory.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::types::{TaskId, TaskKind, TaskPriority, ZoneId};

/// Monotonically increasing counter used by [`Task::new`].
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

/// A unit of work that a robot can execute.
///
/// Tasks are moved by ownership through the system — they are pushed into
/// the queue and popped out without cloning.
pub struct Task {
    pub id: TaskId,
    pub priority: TaskPriority,
    pub kind: TaskKind,
    pub target_zone: ZoneId,
    /// Simulated duration of this task in milliseconds.
    pub duration_ms: u64,
}

impl Task {
    /// Create a new task with an auto-incremented ID.
    ///
    /// # Arguments
    ///
    /// * `priority` — Whether the task is urgent or normal.
    /// * `kind` — The category of work.
    /// * `target_zone` — The zone where the work must be performed.
    /// * `duration_ms` — How long the simulated work takes (milliseconds).
    pub fn new(
        priority: TaskPriority,
        kind: TaskKind,
        target_zone: ZoneId,
        duration_ms: u64,
    ) -> Self {
        Self {
            id: NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed),
            priority,
            kind,
            target_zone,
            duration_ms,
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Task#{} [{}/{}] -> {} ({}ms)",
            self.id, self.priority, self.kind, self.target_zone, self.duration_ms,
        )
    }
}
