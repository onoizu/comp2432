//! Entry point for the Project Blaze demo.
//!
//! Runs three scenarios sequentially, printing the structured event log
//! after each one.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use blaze_core::coordinator::Coordinator;
use blaze_core::types::DEFAULT_HEARTBEAT_TIMEOUT_MS;

fn main() {
    run_basic_delivery();
    run_zone_conflict();
    run_timeout_demo();
}

// -----------------------------------------------------------------------
// Scenario runners
// -----------------------------------------------------------------------

fn run_basic_delivery() {
    println!("========================================");
    println!(" Scenario 1: Basic Delivery");
    println!("========================================\n");

    let timeout = Duration::from_millis(DEFAULT_HEARTBEAT_TIMEOUT_MS);
    let mut coord = Coordinator::new(timeout);
    coord.start_monitor();
    coord.spawn_robots(3);

    for task in blaze_sim::scenarios::basic_delivery() {
        coord.submit_task(task);
    }

    thread::sleep(Duration::from_millis(800));

    let log = coord.event_log();
    coord.shutdown();
    print!("{}", log.dump());
    println!();
}

fn run_zone_conflict() {
    println!("========================================");
    println!(" Scenario 2: Zone Conflict");
    println!("========================================\n");

    let timeout = Duration::from_millis(DEFAULT_HEARTBEAT_TIMEOUT_MS);
    let mut coord = Coordinator::new(timeout);
    coord.start_monitor();
    coord.spawn_robots(3);

    for task in blaze_sim::scenarios::zone_conflict() {
        coord.submit_task(task);
    }

    thread::sleep(Duration::from_millis(1500));

    let log = coord.event_log();
    coord.shutdown();
    print!("{}", log.dump());
    println!();
}

fn run_timeout_demo() {
    println!("========================================");
    println!(" Scenario 3: Timeout Demo");
    println!("========================================\n");

    let timeout = Duration::from_millis(DEFAULT_HEARTBEAT_TIMEOUT_MS);
    let mut coord = Coordinator::new(timeout);
    coord.start_monitor();

    let (tasks, fail_robot_id) = blaze_sim::scenarios::timeout_demo();
    let robot_count = 3;

    let fail_flag = Arc::new(AtomicBool::new(false));

    for id in 0..robot_count {
        if id == fail_robot_id {
            coord.spawn_robot(id, Some(Arc::clone(&fail_flag)));
        } else {
            coord.spawn_robot(id, None);
        }
    }

    for task in tasks {
        coord.submit_task(task);
    }

    thread::sleep(Duration::from_millis(200));
    fail_flag.store(true, Ordering::Relaxed);

    thread::sleep(Duration::from_millis(DEFAULT_HEARTBEAT_TIMEOUT_MS + 2000));

    let log = coord.event_log();
    coord.shutdown();
    print!("{}", log.dump());
    println!();
}
