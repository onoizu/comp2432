//! Pre-built demo scenarios.
//!
//! Each function returns a `Vec<Task>` (and optionally extra metadata)
//! that can be fed into a [`Coordinator`](blaze_core::coordinator::Coordinator).

use blaze_core::task::Task;
use blaze_core::types::{TaskKind, TaskPriority, ZoneId};

/// Basic delivery scenario — several tasks spread across different zones.
///
/// Good for demonstrating that multiple robots execute tasks concurrently
/// without data races.
pub fn basic_delivery() -> Vec<Task> {
    vec![
        Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::WardA, 100),
        Task::new(TaskPriority::Normal, TaskKind::Cleaning, ZoneId::WardB, 80),
        Task::new(TaskPriority::Normal, TaskKind::Inspection, ZoneId::Lobby, 120),
        Task::new(TaskPriority::Urgent, TaskKind::Emergency, ZoneId::EmergencyRoom, 60),
        Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::PharmacyHall, 90),
    ]
}

/// Zone-conflict scenario — multiple tasks target the same zone.
///
/// Demonstrates that zone mutual exclusion works: only one robot can be
/// inside `WardA` at a time while others block and wait.
pub fn zone_conflict() -> Vec<Task> {
    vec![
        Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::WardA, 150),
        Task::new(TaskPriority::Normal, TaskKind::Cleaning, ZoneId::WardA, 150),
        Task::new(TaskPriority::Urgent, TaskKind::Emergency, ZoneId::WardA, 100),
        Task::new(TaskPriority::Normal, TaskKind::Inspection, ZoneId::WardA, 120),
    ]
}

/// Timeout demo scenario.
///
/// Returns a task list together with the robot index that should receive a
/// `fail_flag`.  The coordinator should spawn that robot with
/// `Some(fail_flag)` and set the flag to `true` after a short delay so the
/// health monitor marks it offline.
///
/// # Returns
///
/// `(tasks, fail_robot_id)` where `fail_robot_id` is the `RobotId` of
/// the robot that will be injected with a failure.
pub fn timeout_demo() -> (Vec<Task>, usize) {
    let tasks = vec![
        Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::Lobby, 200),
        Task::new(TaskPriority::Normal, TaskKind::Cleaning, ZoneId::WardB, 200),
        Task::new(TaskPriority::Normal, TaskKind::Inspection, ZoneId::PharmacyHall, 200),
    ];
    let fail_robot_id = 2;
    (tasks, fail_robot_id)
}
