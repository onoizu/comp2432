use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use blaze_core::coordinator::Coordinator;
use blaze_core::event_log::EventKind;
use blaze_core::task::Task;
use blaze_core::traits::HeartbeatRegistry;
use blaze_core::types::{TaskKind, TaskPriority, RobotStatus, ZoneId};

#[test]
fn full_lifecycle_all_tasks_completed() {
    let mut coord = Coordinator::new(Duration::from_secs(5));
    coord.start_monitor();
    coord.spawn_robots(2);

    let task_ids: Vec<u64> = (0..4)
        .map(|_| {
            let t = Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::Lobby, 20);
            let id = t.id;
            coord.submit_task(t);
            id
        })
        .collect();

    thread::sleep(Duration::from_millis(500));

    let log = coord.event_log();
    coord.shutdown();

    let events = log.events();
    for tid in &task_ids {
        let found = events.iter().any(|e| matches!(e, EventKind::TaskCompleted { task_id, .. } if *task_id == *tid));
        assert!(found, "task {tid} should have a TaskCompleted event");
    }
}

#[test]
fn zone_conflict_no_overlap() {
    let mut coord = Coordinator::new(Duration::from_secs(5));
    coord.start_monitor();
    coord.spawn_robots(3);

    for _ in 0..3 {
        coord.submit_task(Task::new(
            TaskPriority::Normal,
            TaskKind::Delivery,
            ZoneId::WardA,
            50,
        ));
    }

    thread::sleep(Duration::from_millis(1000));

    let log = coord.event_log();
    coord.shutdown();

    let events = log.events();
    let mut inside: Vec<usize> = Vec::new();
    for ev in &events {
        match ev {
            EventKind::ZoneEntered { robot_id, zone: ZoneId::WardA } => {
                assert!(
                    !inside.contains(robot_id),
                    "robot {robot_id} entered WardA while already inside"
                );
                assert!(
                    inside.is_empty(),
                    "robot {robot_id} entered WardA while {:?} still inside",
                    inside
                );
                inside.push(*robot_id);
            }
            EventKind::ZoneLeft { robot_id, zone: ZoneId::WardA } => {
                inside.retain(|r| r != robot_id);
            }
            _ => {}
        }
    }
}

#[test]
fn timeout_demo_detects_offline() {
    let mut coord = Coordinator::new(Duration::from_millis(300));
    coord.start_monitor();

    let fail_flag = Arc::new(AtomicBool::new(false));

    coord.spawn_robot(0, None);
    coord.spawn_robot(1, None);
    coord.spawn_robot(2, Some(Arc::clone(&fail_flag)));

    for _ in 0..3 {
        coord.submit_task(Task::new(
            TaskPriority::Normal,
            TaskKind::Delivery,
            ZoneId::Lobby,
            50,
        ));
    }

    thread::sleep(Duration::from_millis(100));
    fail_flag.store(true, Ordering::Relaxed);

    thread::sleep(Duration::from_millis(1500));

    let hm = coord.health_monitor();
    assert_eq!(hm.status(2), Some(RobotStatus::Offline));

    let log = coord.event_log();
    let has_timeout = log.has_event(|e| matches!(e, EventKind::RobotTimedOut { robot_id: 2 }));
    assert!(has_timeout, "robot 2 should have been logged as timed out");

    coord.shutdown();
}

/// Cooperative preemption: a single robot running a preemptible Normal task
/// yields when an Urgent task is pushed, completes the Urgent task first,
/// then finishes the reclaimed Normal task.
#[test]
fn cooperative_preemption_yields_for_urgent() {
    let mut coord = Coordinator::new(Duration::from_secs(10));
    coord.start_monitor();
    coord.spawn_robot(0, None);

    let normal = Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::WardA, 5000);
    let normal_id = normal.id;
    coord.submit_task(normal);

    thread::sleep(Duration::from_millis(1500));

    let urgent = Task::new(TaskPriority::Urgent, TaskKind::Emergency, ZoneId::WardB, 500);
    let urgent_id = urgent.id;
    coord.submit_task(urgent);

    thread::sleep(Duration::from_millis(8000));

    let log = coord.event_log();
    let events = log.events();

    let has_yield = events.iter().any(|e| {
        matches!(e, EventKind::TaskYielded { robot_id: 0, task_id } if *task_id == normal_id)
    });
    assert!(has_yield, "robot 0 should yield the Normal task when Urgent arrives");

    let urgent_completed = events.iter().any(|e| {
        matches!(e, EventKind::TaskCompleted { task_id, .. } if *task_id == urgent_id)
    });
    assert!(urgent_completed, "Urgent task should be completed");

    let normal_completed = events.iter().any(|e| {
        matches!(e, EventKind::TaskCompleted { task_id, .. } if *task_id == normal_id)
    });
    assert!(normal_completed, "reclaimed Normal task should eventually complete");

    let yield_idx = events.iter().position(|e| {
        matches!(e, EventKind::TaskYielded { task_id, .. } if *task_id == normal_id)
    }).unwrap();
    let urgent_complete_idx = events.iter().position(|e| {
        matches!(e, EventKind::TaskCompleted { task_id, .. } if *task_id == urgent_id)
    }).unwrap();
    let normal_complete_idx = events.iter().position(|e| {
        matches!(e, EventKind::TaskCompleted { task_id, .. } if *task_id == normal_id)
    }).unwrap();

    assert!(
        yield_idx < urgent_complete_idx,
        "yield should happen before Urgent completes"
    );
    assert!(
        urgent_complete_idx < normal_complete_idx,
        "Urgent should complete before reclaimed Normal"
    );

    coord.shutdown();
}
