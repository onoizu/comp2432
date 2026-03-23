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
    /// If true, this task may yield when urgent work appears.
    /// Only Normal tasks use this. Urgent tasks are never interrupted.
    /// Yielded tasks restart from the beginning.
    pub preemptible: bool,
}

impl Task {
    /// Create a new task with an auto-incremented ID.
    /// Normal tasks default to preemptible; Urgent tasks default to
    /// non-preemptible.
    ///
    /// Parameters:
    /// `priority` sets urgent vs normal.
    /// `kind` sets the work category.
    /// `target_zone` sets where the work runs.
    /// `duration_ms` sets simulated runtime in milliseconds.
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
            preemptible: priority == TaskPriority::Normal,
        }
    }

    /// Create a task with an explicit preemption policy.
    /// Use this when a Normal task must not be interrupted 
    pub fn new_with_preemptible(
        priority: TaskPriority,
        kind: TaskKind,
        target_zone: ZoneId,
        duration_ms: u64,
        preemptible: bool,
    ) -> Self {
        Self {
            id: NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed),
            priority,
            kind,
            target_zone,
            duration_ms,
            preemptible,
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pre = if self.preemptible { "preemptible" } else { "non-preemptible" };
        write!(
            f,
            "Task#{} [{}/{}/{}] -> {} ({}ms)",
            self.id, self.priority, self.kind, pre, self.target_zone, self.duration_ms,
        )
    }
}
