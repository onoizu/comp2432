//! Trait abstractions for the three core subsystems.
//!
//! Each trait defines the minimum contract that robot workers and the
//! coordinator depend on.  Concrete implementations live in their own
//! modules (`task_queue`, `zone_manager`, `health_monitor`).

use crate::errors::BlazeError;
use crate::task::Task;
use crate::types::{RobotId, RobotStatus, ZoneId};

/// A thread-safe source and sink for robot tasks.
pub trait TaskProvider {
    /// Enqueue a task.  Infallible — the underlying storage is unbounded.
    fn push_task(&self, task: Task);

    /// Block the calling thread until a task is available.
    ///
    /// Returns `None` only after [`shutdown`](TaskProvider::shutdown) has been
    /// called **and** no tasks remain.
    fn pop_task_blocking(&self) -> Option<Task>;

    /// Signal that no more tasks will arrive.
    ///
    /// All threads currently blocked in `pop_task_blocking` will be woken and
    /// will drain any remaining tasks before receiving `None`.
    fn shutdown(&self);

    /// Number of tasks waiting to be popped.
    fn pending_count(&self) -> usize;
}

/// Mutual-exclusion control over hospital zones.
pub trait ZoneAccess {
    /// Block the calling thread until `zone` is free, then mark it as owned
    /// by `robot`.
    fn enter_zone(&self, zone: ZoneId, robot: RobotId);

    /// Release `zone`.
    ///
    /// # Errors
    ///
    /// Returns [`BlazeError::ZoneNotOwned`] if `robot` is not the current
    /// occupant of `zone`.
    fn leave_zone(&self, zone: ZoneId, robot: RobotId) -> Result<(), BlazeError>;

    /// Check whether `zone` is currently occupied.
    fn is_occupied(&self, zone: ZoneId) -> bool;
}

/// Registry that tracks robot liveness via periodic heartbeats.
pub trait HeartbeatRegistry {
    /// Register a new robot as `Online`.
    fn register(&self, robot: RobotId);

    /// Record a heartbeat for `robot`.
    ///
    /// If the robot is already `Offline`, this is a no-op (no auto-recovery).
    fn heartbeat(&self, robot: RobotId);

    /// Return the current status of `robot`, or `None` if it has never been
    /// registered.
    fn status(&self, robot: RobotId) -> Option<RobotStatus>;

    /// Scan all registered robots and mark any that have exceeded the
    /// heartbeat timeout as `Offline`.
    ///
    /// Returns the IDs of robots that were **newly** marked offline during
    /// this call (robots already offline are not included).
    fn check_timeouts(&self) -> Vec<RobotId>;
}
