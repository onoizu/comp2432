//! Runtime metrics collector for demo summaries and exports.
//!
//! The collector is intentionally lightweight and thread-safe. It receives
//! updates from real system event paths (robot worker, monitor thread), then
//! exposes immutable snapshots for dashboard and export use.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::types::{RobotId, ZoneId};

/// Immutable metrics snapshot used by presenters and exporters.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub scenario_name: Option<String>,
    pub runtime_ms: Option<u128>,
    pub total_completed_tasks: u64,
    pub total_zone_wait_events: u64,
    pub total_offline_detections: u64,
    pub per_robot_completed_tasks: HashMap<RobotId, u64>,
    pub per_zone_wait_counts: HashMap<ZoneId, u64>,
}

#[derive(Debug, Default)]
struct MetricsInner {
    scenario_name: Option<String>,
    scenario_start: Option<Instant>,
    runtime_ms: Option<u128>,
    total_completed_tasks: u64,
    total_zone_wait_events: u64,
    total_offline_detections: u64,
    per_robot_completed_tasks: HashMap<RobotId, u64>,
    per_zone_wait_counts: HashMap<ZoneId, u64>,
}

/// Thread-safe runtime metrics collector.
pub struct Metrics {
    inner: Mutex<MetricsInner>,
}

impl Metrics {
    /// Create a new empty metrics collector.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MetricsInner::default()),
        }
    }

    /// Mark the start of a scenario run.
    pub fn start_scenario(&self, name: impl Into<String>) {
        let mut guard = self.inner.lock().expect("metrics lock poisoned");
        guard.scenario_name = Some(name.into());
        guard.scenario_start = Some(Instant::now());
        guard.runtime_ms = None;
    }

    /// Mark the end of a scenario run and compute runtime.
    pub fn end_scenario(&self) {
        let mut guard = self.inner.lock().expect("metrics lock poisoned");
        if let Some(start) = guard.scenario_start.take() {
            guard.runtime_ms = Some(start.elapsed().as_millis());
        }
    }

    /// Record one completed task for `robot_id`.
    pub fn record_task_completed(&self, robot_id: RobotId) {
        let mut guard = self.inner.lock().expect("metrics lock poisoned");
        guard.total_completed_tasks += 1;
        let entry = guard.per_robot_completed_tasks.entry(robot_id).or_insert(0);
        *entry += 1;
    }

    /// Record one zone waiting event.
    pub fn record_zone_wait(&self, _robot_id: RobotId, zone_id: ZoneId) {
        let mut guard = self.inner.lock().expect("metrics lock poisoned");
        guard.total_zone_wait_events += 1;
        let entry = guard.per_zone_wait_counts.entry(zone_id).or_insert(0);
        *entry += 1;
    }

    /// Record one offline detection event.
    pub fn record_robot_offline(&self, _robot_id: RobotId) {
        let mut guard = self.inner.lock().expect("metrics lock poisoned");
        guard.total_offline_detections += 1;
    }

    /// Return an immutable snapshot of all collected metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let guard = self.inner.lock().expect("metrics lock poisoned");
        MetricsSnapshot {
            scenario_name: guard.scenario_name.clone(),
            runtime_ms: guard.runtime_ms,
            total_completed_tasks: guard.total_completed_tasks,
            total_zone_wait_events: guard.total_zone_wait_events,
            total_offline_detections: guard.total_offline_detections,
            per_robot_completed_tasks: guard.per_robot_completed_tasks.clone(),
            per_zone_wait_counts: guard.per_zone_wait_counts.clone(),
        }
    }
}
