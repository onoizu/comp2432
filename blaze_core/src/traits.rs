//! Small interfaces for the three core subsystems.
//!
//! Robot workers and the coordinator depend on these contracts, not on a
//! specific implementation type.

use crate::errors::BlazeError;
use crate::task::Task;
use crate::types::{RobotId, RobotStatus, ZoneId};

/// A thread-safe task source/sink used by robot workers
pub trait TaskProvider {
    /// Adds a task to the queue
    fn push_task(&self, task: Task);



    /// Waits until a task is available.
    ///
    /// Returns `None` only after shutdown is requested and the queue is empty
    fn pop_task_blocking(&self) -> Option<Task>;



    /// Signals that no new tasks will be added.
    /// Waiting workers wake up, finish leftover tasks, then eventually get `None
    fn shutdown(&self);

    /// Number of tasks still waiting in the queue
    fn pending_count(&self) -> usize;
}



/// Controls exclusive access to hospital zones.
pub trait ZoneAccess {
    /// Waits until `zone` is free, then assigns it to robot
    fn enter_zone(&self, zone: ZoneId, robot: RobotId);

    /// Releases zone
    ///
    /// # Errors
    /// Returns [`BlazeError::ZoneNotOwned`] when `robot` is not the current owner.
    fn leave_zone(&self, zone: ZoneId, robot: RobotId) -> Result<(), BlazeError>;

    /// Returns true if `zone` is currently occupied.
    fn is_occupied(&self, zone: ZoneId) -> bool;
}




/// Tracks robot liveness using periodic heartbeats.
pub trait HeartbeatRegistry {
    /// Registers a robot and marks it `Online`.
    fn register(&self, robot: RobotId);


    /// Records a heartbeat for robot
    ///
    /// If the robot is already `Offline`, this does nothing.
    fn heartbeat(&self, robot: RobotId);


    /// Returns current status, or None if the robot was never registered.
    fn status(&self, robot: RobotId) -> Option<RobotStatus>;


    /// Scans robots and marks timed-out ones as `Offline
    ///
    /// Returns only robots that became offline in this call.
    fn check_timeouts(&self) -> Vec<RobotId>;
}
