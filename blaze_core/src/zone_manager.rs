//! Zone access control with mutual exclusion.
//!
//! Only one robot may be inside a zone at a time. If a zone is busy, other
//! robots wait until it is released.
//! This directly targets the PDF requirement: no two robots in one zone.
//!
//! Lock order: 2 (TaskQueue < ZoneManager < HealthMonitor < EventLog < StepGate).
//!
//! Same rule as [`crate::task_queue::TaskQueue`]: no blocking, nested locks,
//! or heavy work while holding this [`Mutex`], except waiting on this struct's
//! [`Condvar`]. [`enter_zone_with_timeout`] runs `on_wait` only after releasing
//! the mutex; [`enter_zone_with_heartbeat`] is a thin wrapper with the default
//! heartbeat interval.


use std::collections::{HashMap, VecDeque};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use crate::errors::BlazeError;
use crate::traits::ZoneAccess;
use crate::types::{RobotId, ZoneId, DEFAULT_HEARTBEAT_INTERVAL_MS};


/// Manages who currently owns each hospital zone.
pub struct ZoneManager {
    inner: Mutex<ZoneManagerInner>,
    condvar: Condvar,
}

struct ZoneManagerInner {
    occupancy: HashMap<ZoneId, Option<RobotId>>,
    waiting: HashMap<ZoneId, VecDeque<RobotId>>,
}

/// Immutable per-zone snapshot for dashboard rendering.
#[derive(Debug, Clone)]
pub struct ZoneStateSnapshot {
    pub zone: ZoneId,
    pub occupant: Option<RobotId>,
    pub waiting_robots: Vec<RobotId>,
}




impl ZoneManager {
    /// Creates a manager with all zones initially free.
    pub fn new() -> Self {
        let mut occupancy = HashMap::new();
        let mut waiting = HashMap::new();
        for &zone in ZoneId::all() {
            occupancy.insert(zone, None);
            waiting.insert(zone, VecDeque::new());
        }
        Self {
            inner: Mutex::new(ZoneManagerInner { occupancy, waiting }),
            condvar: Condvar::new(),
        }
    }

    /// Add `robot` to the waiting queue for `zone` if the zone is occupied.
    /// Call this before logging ZoneWaiting so the dashboard snapshot shows
    /// the robot in waiting_robots at the same step as the event.
    pub fn add_to_waiting_if_occupied(&self, zone: ZoneId, robot: RobotId) {
        let mut guard = self.inner.lock().expect("zone manager lock poisoned");
        if guard.occupancy.get(&zone).is_some_and(|v| v.is_some()) {
            let waiting = guard.waiting.get_mut(&zone).expect("zone waiting missing");
            if !waiting.contains(&robot) {
                waiting.push_back(robot);
            }
        }
    }


    /// Return immutable snapshots for every zone, including waiting robots.
    pub fn snapshot(&self) -> Vec<ZoneStateSnapshot> {
        let guard = self.inner.lock().expect("zone manager lock poisoned");
        let mut rows = Vec::new();
        for &zone in ZoneId::all() {
            let occupant = guard.occupancy.get(&zone).copied().flatten();
            let waiting_robots = guard
                .waiting
                .get(&zone)
                .map(|q| q.iter().copied().collect())
                .unwrap_or_default();
            rows.push(ZoneStateSnapshot {
                zone,
                occupant,
                waiting_robots,
            });
        }
        rows
    }



    /// Tries to enter a zone with periodic timeout-based waiting.
    ///
    /// Returns `true` if the robot successfully enters the zone, or `false` if
    /// the caller stops waiting (`on_wait` returns false), e.g. robot offline.
    ///
    /// Design notes:
    /// - Shared zone state is only mutated while this mutex is held.
    /// - `on_wait` always runs without holding the zone lock, so heartbeat and
    ///   similar callbacks do not extend the critical section or nest locks.
    /// - Waiters are appended to the visible waiting queue before blocking so
    ///   snapshots and UI stay consistent.
    pub fn enter_zone_with_timeout<F>(
        &self,
        zone: ZoneId,
        robot: RobotId,
        timeout: Duration,
        mut on_wait: F,
    ) -> bool
    where
        F: FnMut() -> bool,
    {
        loop {
            let should_wait = {
                let mut guard = self.inner.lock().expect("zone manager lock poisoned");

                if guard.occupancy[&zone].is_none() {
                    false
                } else {
                    let waiting = guard
                        .waiting
                        .get_mut(&zone)
                        .expect("zone waiting missing");

                    if !waiting.contains(&robot) {
                        waiting.push_back(robot);
                    }

                    let (_guard, _) = self
                        .condvar
                        .wait_timeout(guard, timeout)
                        .expect("zone manager lock poisoned");
                    true
                }
            };

            if !on_wait() {
                let mut guard = self.inner.lock().expect("zone manager lock poisoned");
                Self::remove_waiting_locked(&mut guard, zone, robot);
                return false;
            }

            if should_wait {
                continue;
            }

            let mut guard = self.inner.lock().expect("zone manager lock poisoned");

            if guard.occupancy[&zone].is_some() {
                continue;
            }

            Self::remove_waiting_locked(&mut guard, zone, robot);
            guard.occupancy.insert(zone, Some(robot));
            return true;
        }
    }

    /// Same as [`Self::enter_zone_with_timeout`] with `DEFAULT_HEARTBEAT_INTERVAL_MS`.
    pub fn enter_zone_with_heartbeat<F>(&self, zone: ZoneId, robot: RobotId, on_wait: F) -> bool
    where
        F: FnMut() -> bool,
    {
        self.enter_zone_with_timeout(
            zone,
            robot,
            Duration::from_millis(DEFAULT_HEARTBEAT_INTERVAL_MS),
            on_wait,
        )
    }

    /// Remove `robot` from `zone`'s waiting queue. Caller must hold the mutex.
    fn remove_waiting_locked(guard: &mut ZoneManagerInner, zone: ZoneId, robot: RobotId) {
        guard
            .waiting
            .get_mut(&zone)
            .expect("zone waiting missing")
            .retain(|&id| id != robot);
    }
}




impl ZoneAccess for ZoneManager {
    fn enter_zone(&self, zone: ZoneId, robot: RobotId) {
        let mut guard = self.inner.lock().expect("zone manager lock poisoned");
        while guard.occupancy[&zone].is_some() {
            let waiting = guard.waiting.get_mut(&zone).expect("zone waiting missing");
            if !waiting.contains(&robot) {
                waiting.push_back(robot);
            }
            guard = self.condvar.wait(guard).expect("zone manager lock poisoned");
        }
        guard
            .waiting
            .get_mut(&zone)
            .expect("zone waiting missing")
            .retain(|&id| id != robot);
        guard.occupancy.insert(zone, Some(robot));
    }

    fn leave_zone(&self, zone: ZoneId, robot: RobotId) -> Result<(), BlazeError> {
        let mut guard = self.inner.lock().expect("zone manager lock poisoned");
        match guard.occupancy.get(&zone) {
            Some(&Some(occupant)) if occupant == robot => {
                guard.occupancy.insert(zone, None);
                self.condvar.notify_all();
                Ok(())
            }
            _ => Err(BlazeError::ZoneNotOwned { zone, robot }),
        }
    }

    fn is_occupied(&self, zone: ZoneId) -> bool {
        let guard = self.inner.lock().expect("zone manager lock poisoned");
        guard.occupancy.get(&zone).is_some_and(|v| v.is_some())
    }
}
