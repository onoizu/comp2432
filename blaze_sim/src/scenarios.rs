//! Pre-built demo scenarios.
//!
//! Each function returns tasks that can be submitted directly to a
//! `Coordinator`.
//! The three scenarios map to the PDF demo checklist:
//! concurrent work, zone conflict, and timeout detection.

use blaze_core::task::Task;
use blaze_core::types::{TaskKind, TaskPriority, ZoneId};

/// Basic mixed workload across different zones.
///
/// Good first run: robots can work in parallel because most tasks target
/// different places.
pub fn basic_delivery() -> Vec<Task> {
    vec![
        Task::new(TaskPriority::Urgent, TaskKind::Emergency, ZoneId::EmergencyRoom, 40),
        Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::WardA, 50),
        Task::new(TaskPriority::Normal, TaskKind::Cleaning, ZoneId::WardB, 50),
    ]
}

/// Zone-conflict workload where many tasks target the same zone.
///
/// Shows the lock behavior: only one robot is allowed in `WardA` at a time.
pub fn zone_conflict() -> Vec<Task> {
    vec![
        Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::WardA, 60),
        Task::new(TaskPriority::Normal, TaskKind::Cleaning, ZoneId::WardA, 60),
        Task::new(TaskPriority::Urgent, TaskKind::Emergency, ZoneId::WardA, 40),
    ]
}

/// Timeout demo workload.
///
/// Returns initial tasks, a late task, and the robot ID that should be forced
/// to "fail" (stop heartbeats) so the monitor can detect a timeout.
pub fn timeout_demo() -> (Vec<Task>, Task, usize) {
    let initial_tasks = vec![
        Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::WardB, 5000),
        Task::new(TaskPriority::Normal, TaskKind::Cleaning, ZoneId::WardA, 5000),
    ];
    let late_task = Task::new(TaskPriority::Normal, TaskKind::Inspection, ZoneId::WardB, 3000);
    let fail_robot_id = 2;
    (initial_tasks, late_task, fail_robot_id)
}

/// Cooperative preemption demo — single robot, delayed Urgent.
///
/// Phase 1: R0 receives a long preemptible Normal task (WardA, 5000ms).
/// Phase 2: After ~2000ms, an Urgent Emergency task (WardB, 1000ms) is pushed.
///          R0 detects the Urgent task during periodic interrupt checks,
///          cooperatively yields the Normal task, completes the Urgent task,
///          then picks up the reclaimed Normal and finishes it.
///
/// Demonstrates cooperative priority scheduling and interruptible execution
/// of low-priority work.
///
/// # Returns
///
/// `(initial_tasks, late_urgent_task)`
pub fn cooperative_preemption_demo() -> (Vec<Task>, Task) {
    let initial_tasks = vec![
        Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::WardA, 5000),
    ];
    let late_urgent = Task::new(TaskPriority::Urgent, TaskKind::Emergency, ZoneId::WardB, 1000);
    (initial_tasks, late_urgent)
}

