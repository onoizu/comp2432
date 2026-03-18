//! Zone access control with mutual exclusion.
//!
//! At most one robot may occupy a given zone at any time.  A robot that
//! tries to enter an occupied zone blocks until the current occupant
//! leaves.

use std::collections::HashMap;
use std::sync::{Condvar, Mutex};

use crate::errors::BlazeError;
use crate::traits::ZoneAccess;
use crate::types::{RobotId, ZoneId};

/// Manages exclusive access to hospital zones.
///
/// Each zone is either free (`None`) or occupied by exactly one robot
/// (`Some(robot_id)`).
pub struct ZoneManager {
    inner: Mutex<HashMap<ZoneId, Option<RobotId>>>,
    condvar: Condvar,
}

impl ZoneManager {
    /// Create a new manager with all zones initially free.
    pub fn new() -> Self {
        let mut map = HashMap::new();
        for &zone in ZoneId::all() {
            map.insert(zone, None);
        }
        Self {
            inner: Mutex::new(map),
            condvar: Condvar::new(),
        }
    }
}

impl ZoneAccess for ZoneManager {
    fn enter_zone(&self, zone: ZoneId, robot: RobotId) {
        let mut guard = self.inner.lock().expect("zone manager lock poisoned");
        while guard[&zone].is_some() {
            guard = self.condvar.wait(guard).expect("zone manager lock poisoned");
        }
        guard.insert(zone, Some(robot));
    }

    fn leave_zone(&self, zone: ZoneId, robot: RobotId) -> Result<(), BlazeError> {
        let mut guard = self.inner.lock().expect("zone manager lock poisoned");
        match guard.get(&zone) {
            Some(&Some(occupant)) if occupant == robot => {
                guard.insert(zone, None);
                self.condvar.notify_all();
                Ok(())
            }
            _ => Err(BlazeError::ZoneNotOwned { zone, robot }),
        }
    }

    fn is_occupied(&self, zone: ZoneId) -> bool {
        let guard = self.inner.lock().expect("zone manager lock poisoned");
        guard.get(&zone).is_some_and(|v| v.is_some())
    }
}
