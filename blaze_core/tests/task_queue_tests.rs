use std::collections::HashSet;
use std::sync::Arc;
use std::thread;

use blaze_core::task::Task;
use blaze_core::task_queue::TaskQueue;
use blaze_core::traits::TaskProvider;
use blaze_core::types::{TaskKind, TaskPriority, ZoneId};

#[test]
fn urgent_tasks_popped_before_normal() {
    let queue = TaskQueue::new();

    queue.push_task(Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::WardA, 10));
    queue.push_task(Task::new(TaskPriority::Normal, TaskKind::Cleaning, ZoneId::WardB, 10));
    queue.push_task(Task::new(TaskPriority::Urgent, TaskKind::Emergency, ZoneId::Lobby, 10));

    let first = queue.pop_task_blocking().unwrap();
    assert_eq!(first.priority, TaskPriority::Urgent);

    let second = queue.pop_task_blocking().unwrap();
    assert_eq!(second.priority, TaskPriority::Normal);
}

#[test]
fn no_duplicate_tasks_across_threads() {
    let queue = Arc::new(TaskQueue::new());
    let task_count = 50;

    for _ in 0..task_count {
        queue.push_task(Task::new(
            TaskPriority::Normal,
            TaskKind::Delivery,
            ZoneId::Lobby,
            1,
        ));
    }
    queue.shutdown();

    let mut handles = Vec::new();
    for _ in 0..4 {
        let q = Arc::clone(&queue);
        handles.push(thread::spawn(move || {
            let mut ids = Vec::new();
            while let Some(task) = q.pop_task_blocking() {
                ids.push(task.id);
            }
            ids
        }));
    }

    let mut all_ids = Vec::new();
    for h in handles {
        all_ids.extend(h.join().unwrap());
    }

    let unique: HashSet<_> = all_ids.iter().copied().collect();
    assert_eq!(unique.len(), all_ids.len(), "duplicate task IDs detected");
    assert_eq!(all_ids.len(), task_count);
}

#[test]
fn blocking_pop_wakes_on_push() {
    let queue = Arc::new(TaskQueue::new());
    let q = Arc::clone(&queue);

    let handle = thread::spawn(move || q.pop_task_blocking());

    thread::sleep(std::time::Duration::from_millis(50));
    queue.push_task(Task::new(
        TaskPriority::Normal,
        TaskKind::Inspection,
        ZoneId::WardA,
        1,
    ));

    let result = handle.join().unwrap();
    assert!(result.is_some());
}

#[test]
fn shutdown_unblocks_waiting_threads() {
    let queue = Arc::new(TaskQueue::new());
    let q = Arc::clone(&queue);

    let handle = thread::spawn(move || q.pop_task_blocking());

    thread::sleep(std::time::Duration::from_millis(50));
    queue.shutdown();

    let result = handle.join().unwrap();
    assert!(result.is_none());
}

#[test]
fn remaining_tasks_drained_after_shutdown() {
    let queue = TaskQueue::new();

    queue.push_task(Task::new(TaskPriority::Normal, TaskKind::Delivery, ZoneId::WardA, 1));
    queue.push_task(Task::new(TaskPriority::Normal, TaskKind::Cleaning, ZoneId::WardB, 1));
    queue.shutdown();

    assert!(queue.pop_task_blocking().is_some());
    assert!(queue.pop_task_blocking().is_some());
    assert!(queue.pop_task_blocking().is_none());
}
