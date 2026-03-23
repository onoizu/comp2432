use std::sync::Arc;
use std::thread;
use std::time::Duration;

use blaze_core::event_log::{EventKind, EventLog};
use blaze_core::metrics::Metrics;
use blaze_core::summary::{
    DashboardMetricsSummary, QueueSummary, RobotState, RobotSummary, SystemSnapshot, ZoneSummary,
};
use blaze_core::task::Task;
use blaze_core::traits::ZoneAccess;
use blaze_core::types::{RobotStatus, TaskKind, TaskPriority, ZoneId};
use blaze_core::zone_manager::ZoneManager;

#[test]
fn metrics_records_core_counters() {
    let metrics = Metrics::new();
    metrics.start_scenario("metrics-test");
    metrics.record_task_completed(0);
    metrics.record_task_completed(0);
    metrics.record_zone_wait(1, ZoneId::WardA);
    metrics.record_robot_offline(2);
    metrics.end_scenario();

    let snap = metrics.snapshot();
    assert_eq!(snap.total_completed_tasks, 2);
    assert_eq!(snap.total_zone_wait_events, 1);
    assert_eq!(snap.total_offline_detections, 1);
    assert_eq!(snap.per_robot_completed_tasks.get(&0), Some(&2));
    assert_eq!(snap.per_zone_wait_counts.get(&ZoneId::WardA), Some(&1));
    assert!(snap.runtime_ms.is_some());
}

#[test]
fn event_log_pretty_and_timeline_are_structured() {
    let log = EventLog::new();
    log.log(EventKind::RobotStarted { robot_id: 0 });
    log.log(EventKind::TaskReceived {
        robot_id: 0,
        task_id: 10,
    });
    log.log(EventKind::ZoneWaiting {
        robot_id: 0,
        zone: ZoneId::WardA,
    });

    let pretty = log.dump_pretty();
    assert!(pretty.contains("[ROBOT ]"));
    assert!(pretty.contains("[TASK  ]"));
    assert!(pretty.contains("[ZONE  ]"));

    let timeline = log.timeline();
    assert!(!timeline.is_empty());
    assert_eq!(timeline[0].code, "ROBOT_STARTED");
}

#[test]
fn zone_manager_snapshot_shows_waiting_order() {
    let zone_manager = Arc::new(ZoneManager::new());
    zone_manager.enter_zone(ZoneId::WardA, 0);

    let zm = Arc::clone(&zone_manager);
    let handle = thread::spawn(move || {
        zm.enter_zone(ZoneId::WardA, 1);
        let _ = zm.leave_zone(ZoneId::WardA, 1);
    });

    thread::sleep(Duration::from_millis(80));
    let snapshot = zone_manager.snapshot();
    let ward_a = snapshot
        .iter()
        .find(|z| z.zone == ZoneId::WardA)
        .expect("WardA snapshot should exist");
    assert!(
        ward_a.waiting_robots.contains(&1),
        "Robot 1 should be visible in waiting queue"
    );

    let _ = zone_manager.leave_zone(ZoneId::WardA, 0);
    handle.join().expect("waiting thread should join");
}

#[test]
fn coordinator_snapshot_contains_queue_and_metrics() {
    let mut coord = blaze_core::coordinator::Coordinator::new(Duration::from_secs(3));
    coord.start_monitor();
    coord.spawn_robots(1);
    coord.submit_task(Task::new(
        TaskPriority::Normal,
        TaskKind::Delivery,
        ZoneId::Lobby,
        40,
    ));
    thread::sleep(Duration::from_millis(120));

    let snap = coord.snapshot();
    assert!(snap.queue.total_count <= 1);
    assert!(!snap.robots.is_empty());

    coord.shutdown();
}

#[test]
fn event_log_count_and_incremental_timeline() {
    let log = EventLog::new();
    assert_eq!(log.event_count(), 0);

    log.log(EventKind::RobotStarted { robot_id: 0 });
    log.log(EventKind::RobotStarted { robot_id: 1 });
    log.log(EventKind::TaskReceived {
        robot_id: 0,
        task_id: 1,
    });
    assert_eq!(log.event_count(), 3);

    let since_1 = log.timeline_since(1);
    assert_eq!(since_1.len(), 2);
    assert_eq!(since_1[0].code, "ROBOT_STARTED");
    assert_eq!(since_1[1].code, "TASK_RECEIVED");

    let since_3 = log.timeline_since(3);
    assert!(since_3.is_empty());
}

#[test]
fn event_log_json_since_produces_valid_json() {
    let log = EventLog::new();
    log.log(EventKind::RobotStarted { robot_id: 0 });
    log.log(EventKind::ZoneEntered {
        robot_id: 0,
        zone: ZoneId::WardA,
    });

    let json = log.events_json_since(0);
    assert!(json.contains("\"total_count\":2"));
    assert!(json.contains("\"code\":\"ROBOT_STARTED\""));
    assert!(json.contains("\"code\":\"ZONE_ENTERED\""));
    assert!(json.contains("\"index\":0"));
    assert!(json.contains("\"index\":1"));

    let json_partial = log.events_json_since(1);
    assert!(json_partial.contains("\"total_count\":2"));
    assert!(!json_partial.contains("ROBOT_STARTED"));
    assert!(json_partial.contains("ZONE_ENTERED"));
}

#[test]
fn system_snapshot_to_json_has_stable_field_names() {
    let snapshot = SystemSnapshot {
        queue: QueueSummary {
            urgent_count: 1,
            normal_count: 3,
            total_count: 4,
            total_pushed: 6,
            tasks: vec![],
        },
        zones: vec![ZoneSummary {
            zone: ZoneId::WardA,
            occupant: Some(0),
            waiting_robots: vec![1, 2],
        }],
        robots: vec![
            RobotSummary {
                robot_id: 0,
                state: RobotState::Busy,
                status: RobotStatus::Online,
                current_task_id: Some(42),
                current_zone: Some(ZoneId::WardA),
            },
            RobotSummary {
                robot_id: 1,
                state: RobotState::WaitingZone,
                status: RobotStatus::Online,
                current_task_id: Some(43),
                current_zone: None,
            },
        ],
        metrics: DashboardMetricsSummary {
            completed_task_count: 5,
            total_wait_count: 2,
            offline_count: 0,
            runtime_ms: Some(1200),
        },
    };

    let json = snapshot.to_json(true, "Test Scenario");
    assert!(json.contains("\"running\":true"));
    assert!(json.contains("\"scenario_name\":\"Test Scenario\""));
    assert!(json.contains("\"urgent_count\":1"));
    assert!(json.contains("\"normal_count\":3"));
    assert!(json.contains("\"total_count\":4"));
    assert!(json.contains("\"zone\":\"WardA\""));
    assert!(json.contains("\"occupant\":0"));
    assert!(json.contains("\"waiting_robots\":[1,2]"));
    assert!(json.contains("\"robot_id\":0"));
    assert!(json.contains("\"state\":\"Busy\""));
    assert!(json.contains("\"state\":\"WaitingZone\""));
    assert!(json.contains("\"current_task_id\":42"));
    assert!(json.contains("\"current_zone\":\"WardA\""));
    assert!(json.contains("\"current_zone\":null"));
    assert!(json.contains("\"completed_task_count\":5"));
    assert!(json.contains("\"runtime_ms\":1200"));
}

#[test]
fn system_snapshot_json_idle_state() {
    let snapshot = SystemSnapshot {
        queue: QueueSummary {
            urgent_count: 0,
            normal_count: 0,
            total_count: 0,
            total_pushed: 0,
            tasks: vec![],
        },
        zones: vec![],
        robots: vec![],
        metrics: DashboardMetricsSummary {
            completed_task_count: 0,
            total_wait_count: 0,
            offline_count: 0,
            runtime_ms: None,
        },
    };

    let json = snapshot.to_json(false, "");
    assert!(json.contains("\"running\":false"));
    assert!(json.contains("\"zones\":[]"));
    assert!(json.contains("\"robots\":[]"));
    assert!(json.contains("\"runtime_ms\":null"));
}
