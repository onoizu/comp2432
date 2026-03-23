//! Pre-built demo scenarios.
//!
//! Each function returns a `Vec<Task>` (and optionally extra metadata)
//! that can be fed into a [`Coordinator`](blaze_core::coordinator::Coordinator).

use blaze_core::task::Task;
use blaze_core::types::{TaskKind, TaskPriority, ZoneId};

/// Basic delivery scenario — 3 tasks in different zones, 1 urgent.
///
/// Demonstrates parallel execution and priority scheduling.
pub fn basic_delivery() -> Vec<Task> {
    vec![
        Task::new(TaskPriority::Urgent, TaskKind::Emergency, ZoneId::EmergencyRoom, 40),
        Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::WardA, 50),
        Task::new(TaskPriority::Normal, TaskKind::Cleaning, ZoneId::WardB, 50),
    ]
}

/// Zone-conflict scenario — 3 tasks all target WardA.
///
/// Demonstrates mutex: only one robot enters at a time, others wait.
pub fn zone_conflict() -> Vec<Task> {
    vec![
        Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::WardA, 60),
        Task::new(TaskPriority::Normal, TaskKind::Cleaning, ZoneId::WardA, 60),
        Task::new(TaskPriority::Urgent, TaskKind::Emergency, ZoneId::WardA, 40),
    ]
}

/// Timeout demo scenario — staggered two-phase design.
///
/// Phase 1: R0 and R1 receive `initial_tasks` and enter their zones.
/// Phase 2: After a delay, R2 is spawned and receives `late_task` (WardB).
///          WardB is already occupied by R0, so R2 must wait. R2 then times
///          out, reclaims the task, and another robot re-executes it.
///
/// # Returns
///
/// `(initial_tasks, late_task, fail_robot_id)`
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

