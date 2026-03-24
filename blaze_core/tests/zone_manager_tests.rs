use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use blaze_core::errors::BlazeError;
use blaze_core::traits::ZoneAccess;
use blaze_core::types::ZoneId;
use blaze_core::zone_manager::ZoneManager;

#[test]
fn enter_and_leave_zone() {
    let zm = ZoneManager::new();
    zm.enter_zone(ZoneId::WardA, 0);
    assert!(zm.is_occupied(ZoneId::WardA));
    assert!(zm.leave_zone(ZoneId::WardA, 0).is_ok());
    assert!(!zm.is_occupied(ZoneId::WardA));
}

#[test]
fn leave_zone_wrong_robot_returns_error() {
    let zm = ZoneManager::new();
    zm.enter_zone(ZoneId::WardA, 0);
    let result = zm.leave_zone(ZoneId::WardA, 1);
    assert!(matches!(result, Err(BlazeError::ZoneNotOwned { .. })));
    assert!(zm.leave_zone(ZoneId::WardA, 0).is_ok());
}

#[test]
fn second_robot_blocks_until_zone_free() {
    let zm = Arc::new(ZoneManager::new());
    let counter = Arc::new(AtomicUsize::new(0));

    zm.enter_zone(ZoneId::Lobby, 0);

    let zm2 = Arc::clone(&zm);
    let counter2 = Arc::clone(&counter);
    let handle = thread::spawn(move || {
        zm2.enter_zone(ZoneId::Lobby, 1);
        counter2.fetch_add(1, Ordering::SeqCst);
        let _ = zm2.leave_zone(ZoneId::Lobby, 1);
    });

    thread::sleep(Duration::from_millis(100));
    assert_eq!(counter.load(Ordering::SeqCst), 0, "robot 1 should still be blocked");

    let _ = zm.leave_zone(ZoneId::Lobby, 0);
    handle.join().unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1, "robot 1 should have entered after release");
}

#[test]
fn different_zones_occupied_simultaneously() {
    let zm = ZoneManager::new();
    zm.enter_zone(ZoneId::WardA, 0);
    zm.enter_zone(ZoneId::WardB, 1);
    assert!(zm.is_occupied(ZoneId::WardA));
    assert!(zm.is_occupied(ZoneId::WardB));
    assert!(zm.leave_zone(ZoneId::WardA, 0).is_ok());
    assert!(zm.leave_zone(ZoneId::WardB, 1).is_ok());
}

#[test]
fn enter_zone_with_timeout_aborts_when_on_wait_false() {
    let zm = Arc::new(ZoneManager::new());
    zm.enter_zone(ZoneId::WardA, 0);

    let zm2 = Arc::clone(&zm);
    let handle = thread::spawn(move || {
        zm2.enter_zone_with_timeout(
            ZoneId::WardA,
            1,
            Duration::from_millis(20),
            || false,
        )
    });

    assert!(!handle.join().unwrap(), "on_wait false should abort enter");

    let row = zm
        .snapshot()
        .into_iter()
        .find(|z| z.zone == ZoneId::WardA)
        .unwrap();
    assert!(
        !row.waiting_robots.contains(&1),
        "robot 1 should be removed from waiting after abort"
    );
    let _ = zm.leave_zone(ZoneId::WardA, 0);
}

#[test]
fn enter_zone_with_heartbeat_enters_after_release() {
    let zm = Arc::new(ZoneManager::new());
    zm.enter_zone(ZoneId::Lobby, 0);

    let zm2 = Arc::clone(&zm);
    let entered = Arc::new(AtomicBool::new(false));
    let entered2 = Arc::clone(&entered);
    let handle = thread::spawn(move || {
        let ok = zm2.enter_zone_with_heartbeat(ZoneId::Lobby, 1, || true);
        entered2.store(ok, Ordering::SeqCst);
        let _ = zm2.leave_zone(ZoneId::Lobby, 1);
    });

    thread::sleep(Duration::from_millis(50));
    assert!(!entered.load(Ordering::SeqCst));
    let _ = zm.leave_zone(ZoneId::Lobby, 0);
    handle.join().unwrap();
    assert!(entered.load(Ordering::SeqCst));
}
