//! Zone access control with mutual exclusion.
//!
//! At most one robot may occupy a given zone at any time.  A robot that
//! tries to enter an occupied zone blocks until the current occupant
//! leaves.
//!
//! Lock order: 2 (TaskQueue < ZoneManager < HealthMonitor < EventLog < StepGate).

use std::collections::{HashMap, VecDeque};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use crate::errors::BlazeError;
use crate::traits::ZoneAccess;
use crate::types::{RobotId, ZoneId, DEFAULT_HEARTBEAT_INTERVAL_MS};

/// Manages exclusive access to hospital zones.
///
/// Each zone is either free (`None`) or occupied by exactly one robot
/// (`Some(robot_id)`).
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
    /// Create a new manager with all zones initially free.
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

    /// Block until `zone` is free, then mark it as owned by `robot`.
    /// While waiting, calls `on_wait` periodically. If `on_wait` returns `false`,
    /// aborts the wait (e.g. robot was marked offline) and returns `false`.
    /// Returns `true` if the robot successfully entered the zone.
    pub fn enter_zone_with_heartbeat<F>(&self, zone: ZoneId, robot: RobotId, mut on_wait: F) -> bool
    where
        F: FnMut() -> bool,
    {
        let timeout = Duration::from_millis(DEFAULT_HEARTBEAT_INTERVAL_MS);
        let mut guard = self.inner.lock().expect("zone manager lock poisoned");
        while guard.occupancy[&zone].is_some() {
            let waiting = guard.waiting.get_mut(&zone).expect("zone waiting missing");
            if !waiting.contains(&robot) {
                waiting.push_back(robot);
            }
            let (g, _) = self
                .condvar
                .wait_timeout(guard, timeout)
                .expect("zone manager lock poisoned");
            guard = g;
            if !on_wait() {
                guard
                    .waiting
                    .get_mut(&zone)
                    .expect("zone waiting missing")
                    .retain(|&id| id != robot);
                return false;
            }
        }
        if !on_wait() {
            guard
                .waiting
                .get_mut(&zone)
                .expect("zone waiting missing")
                .retain(|&id| id != robot);
            return false;
        }
        guard
            .waiting
            .get_mut(&zone)
            .expect("zone waiting missing")
            .retain(|&id| id != robot);
        guard.occupancy.insert(zone, Some(robot));
        true
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
