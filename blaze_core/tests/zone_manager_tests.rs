use std::sync::atomic::{AtomicUsize, Ordering};
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
