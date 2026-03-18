use std::thread;
use std::time::Duration;

use blaze_core::health_monitor::HealthMonitor;
use blaze_core::traits::HeartbeatRegistry;
use blaze_core::types::RobotStatus;

#[test]
fn heartbeat_keeps_robot_online() {
    let hm = HealthMonitor::new(Duration::from_millis(200));
    hm.register(0);

    for _ in 0..5 {
        thread::sleep(Duration::from_millis(50));
        hm.heartbeat(0);
    }

    assert_eq!(hm.status(0), Some(RobotStatus::Online));
    let timed_out = hm.check_timeouts();
    assert!(timed_out.is_empty());
}

#[test]
fn missing_heartbeat_triggers_offline() {
    let hm = HealthMonitor::new(Duration::from_millis(100));
    hm.register(0);

    thread::sleep(Duration::from_millis(200));
    let timed_out = hm.check_timeouts();
    assert!(timed_out.contains(&0));
    assert_eq!(hm.status(0), Some(RobotStatus::Offline));
}

#[test]
fn offline_robot_ignores_heartbeat() {
    let hm = HealthMonitor::new(Duration::from_millis(100));
    hm.register(0);

    thread::sleep(Duration::from_millis(200));
    hm.check_timeouts();
    assert_eq!(hm.status(0), Some(RobotStatus::Offline));

    hm.heartbeat(0);
    assert_eq!(hm.status(0), Some(RobotStatus::Offline));
}

#[test]
fn check_timeouts_returns_only_newly_offline() {
    let hm = HealthMonitor::new(Duration::from_millis(100));
    hm.register(0);

    thread::sleep(Duration::from_millis(200));
    let first = hm.check_timeouts();
    assert!(first.contains(&0));

    let second = hm.check_timeouts();
    assert!(second.is_empty(), "already-offline robots should not be reported again");
}
