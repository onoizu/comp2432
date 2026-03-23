//! Health monitoring via periodic heartbeats.
//!
//! Each robot is expected to call [`heartbeat`](HeartbeatRegistry::heartbeat)
//! periodically.  A background monitor thread calls
//! [`check_timeouts`](HeartbeatRegistry::check_timeouts) to detect robots
//! that have gone silent and marks them as `Offline`.
//!
//! Once a robot is marked `Offline` it stays `Offline` , there is no automatic recovery.
//!
//! Lock order: 3 (TaskQueue < ZoneManager < HealthMonitor < EventLog < StepGate).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::traits::HeartbeatRegistry;
use crate::types::{RobotId, RobotStatus, TaskId, ZoneId};

/// Immutable per-robot health snapshot for dashboard rendering.
#[derive(Debug, Clone, Copy)]
pub struct RobotHealthSnapshot {
    pub robot_id: RobotId,
    pub status: RobotStatus,
    pub current_task: Option<TaskId>,
    pub current_zone: Option<ZoneId>,
}

/// Per-robot health record.
pub struct RobotHealth {
    pub last_seen: Instant,
    pub status: RobotStatus,
    pub current_task: Option<TaskId>,
    pub current_zone: Option<ZoneId>,
}

/// Thread-safe registry of robot health records.
pub struct HealthMonitor {
    registry: Mutex<HashMap<RobotId, RobotHealth>>,
    timeout: Duration,
}

impl HealthMonitor {
    /// Create a new monitor with the given heartbeat timeout.
    ///
    /// `timeout` is the maximum silent duration before a robot is marked offline.
    pub fn new(timeout: Duration) -> Self {
        Self {
            registry: Mutex::new(HashMap::new()),
            timeout,
        }
    }

    /// Update the current task for `robot`.
    pub fn update_task(&self, robot: RobotId, task_id: Option<TaskId>) {
        let mut guard = self.registry.lock().expect("health monitor lock poisoned");
        if let Some(entry) = guard.get_mut(&robot) {
            entry.current_task = task_id;
        }
    }

    /// Update the current zone for `robot`
    pub fn update_zone(&self, robot: RobotId, zone: Option<ZoneId>) {
        let mut guard = self.registry.lock().expect("health monitor lock poisoned");
        if let Some(entry) = guard.get_mut(&robot) {
            entry.current_zone = zone;
        }
    }

    /// Return immutable snapshots for all registered robots.
    pub fn snapshot(&self) -> Vec<RobotHealthSnapshot> {
        let guard = self.registry.lock().expect("health monitor lock poisoned");
        let mut rows: Vec<RobotHealthSnapshot> = guard
            .iter()
            .map(|(&robot_id, entry)| RobotHealthSnapshot {
                robot_id,
                status: entry.status,
                current_task: entry.current_task,
                current_zone: entry.current_zone,
            })
            .collect();
        rows.sort_by_key(|r| r.robot_id);
        rows
    }
}

impl HeartbeatRegistry for HealthMonitor {
    fn register(&self, robot: RobotId) {
        let mut guard = self.registry.lock().expect("health monitor lock poisoned");
        guard.insert(robot, RobotHealth {
            last_seen: Instant::now(),
            status: RobotStatus::Online,
            current_task: None,
            current_zone: None,
        });
    }

    fn heartbeat(&self, robot: RobotId) {
        let mut guard = self.registry.lock().expect("health monitor lock poisoned");
        if let Some(entry) = guard.get_mut(&robot) {
            if entry.status == RobotStatus::Online {
                entry.last_seen = Instant::now();
            }
        }
    }

    fn status(&self, robot: RobotId) -> Option<RobotStatus> {
        let guard = self.registry.lock().expect("health monitor lock poisoned");
        guard.get(&robot).map(|h| h.status)
    }

    fn check_timeouts(&self) -> Vec<RobotId> {
        let mut guard = self.registry.lock().expect("health monitor lock poisoned");
        let now = Instant::now();
        let mut newly_offline = Vec::new();
        for (&id, health) in guard.iter_mut() {
            if health.status == RobotStatus::Online
                && now.duration_since(health.last_seen) > self.timeout
            {
                health.status = RobotStatus::Offline;
                newly_offline.push(id);
            }
        }
        newly_offline
    }
}
