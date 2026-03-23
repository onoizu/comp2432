//! Read-only system summary snapshot used by demo dashboard rendering.
//!
//! This module does not mutate core state. It only aggregates immutable views
//! from the real task queue, zone manager, health monitor, and metrics.

use std::fmt;

use crate::metrics::MetricsSnapshot;
use crate::task_queue::QueuedTaskInfo;
use crate::types::{RobotId, RobotStatus, TaskId, ZoneId};

/// Queue summary section.
#[derive(Debug, Clone)]
pub struct QueueSummary {
    pub urgent_count: usize,
    pub normal_count: usize,
    pub total_count: usize,
    pub total_pushed: usize,
    pub tasks: Vec<QueuedTaskInfo>,
}

/// Per-zone summary section.
#[derive(Debug, Clone)]
pub struct ZoneSummary {
    pub zone: ZoneId,
    pub occupant: Option<RobotId>,
    pub waiting_robots: Vec<RobotId>,
}

/// High-level robot state for dashboard readability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotState {
    Idle,
    Busy,
    WaitingZone,
    Offline,
}

impl RobotState {
    /// Return a stable string representation used by JSON and display layers.
    pub fn as_str(&self) -> &'static str {
        match self {
            RobotState::Idle => "Idle",
            RobotState::Busy => "Busy",
            RobotState::WaitingZone => "WaitingZone",
            RobotState::Offline => "Offline",
        }
    }
}

impl fmt::Display for RobotState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Per-robot summary section.
#[derive(Debug, Clone, Copy)]
pub struct RobotSummary {
    pub robot_id: RobotId,
    pub state: RobotState,
    pub status: RobotStatus,
    pub current_task_id: Option<TaskId>,
    pub current_zone: Option<ZoneId>,
}

/// Metrics summary section.
#[derive(Debug, Clone)]
pub struct DashboardMetricsSummary {
    pub completed_task_count: u64,
    pub total_wait_count: u64,
    pub offline_count: u64,
    pub runtime_ms: Option<u128>,
}

impl From<&MetricsSnapshot> for DashboardMetricsSummary {
    fn from(value: &MetricsSnapshot) -> Self {
        Self {
            completed_task_count: value.total_completed_tasks,
            total_wait_count: value.total_zone_wait_events,
            offline_count: value.total_offline_detections,
            runtime_ms: value.runtime_ms,
        }
    }
}

/// Unified dashboard snapshot.
#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub queue: QueueSummary,
    pub zones: Vec<ZoneSummary>,
    pub robots: Vec<RobotSummary>,
    pub metrics: DashboardMetricsSummary,
}

impl SystemSnapshot {
    /// Serialize snapshot to a JSON string for API consumption.
    pub fn to_json(&self, running: bool, scenario_name: &str) -> String {
        let mut out = String::with_capacity(2048);
        out.push_str("{\"running\":");
        out.push_str(if running { "true" } else { "false" });
        out.push_str(",\"scenario_name\":\"");
        json_escape_into(&mut out, scenario_name);
        out.push_str("\",\"queue\":{\"urgent_count\":");
        push_usize(&mut out, self.queue.urgent_count);
        out.push_str(",\"normal_count\":");
        push_usize(&mut out, self.queue.normal_count);
        out.push_str(",\"total_count\":");
        push_usize(&mut out, self.queue.total_count);
        out.push_str(",\"total_pushed\":");
        push_usize(&mut out, self.queue.total_pushed);
        out.push_str(",\"tasks\":[");
        for (i, t) in self.queue.tasks.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"id\":");
            push_u64(&mut out, t.id);
            out.push_str(",\"priority\":\"");
            out.push_str(&t.priority.to_string());
            out.push_str("\",\"kind\":\"");
            out.push_str(&t.kind.to_string());
            out.push_str("\",\"zone\":\"");
            out.push_str(&t.target_zone.to_string());
            out.push_str("\"}");
        }
        out.push_str("]},\"zones\":[");
        for (i, z) in self.zones.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"zone\":\"");
            out.push_str(&z.zone.to_string());
            out.push_str("\",\"occupant\":");
            push_option_usize(&mut out, z.occupant);
            out.push_str(",\"waiting_robots\":[");
            for (j, &rid) in z.waiting_robots.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                push_usize(&mut out, rid);
            }
            out.push_str("]}");
        }
        out.push_str("],\"robots\":[");
        for (i, r) in self.robots.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"robot_id\":");
            push_usize(&mut out, r.robot_id);
            out.push_str(",\"state\":\"");
            out.push_str(r.state.as_str());
            out.push_str("\",\"status\":\"");
            out.push_str(&r.status.to_string());
            out.push_str("\",\"current_task_id\":");
            push_option_u64(&mut out, r.current_task_id);
            out.push_str(",\"current_zone\":");
            match r.current_zone {
                Some(z) => {
                    out.push('"');
                    out.push_str(&z.to_string());
                    out.push('"');
                }
                None => out.push_str("null"),
            }
            out.push('}');
        }
        out.push_str("],\"metrics\":{\"completed_task_count\":");
        push_u64(&mut out, self.metrics.completed_task_count);
        out.push_str(",\"total_wait_count\":");
        push_u64(&mut out, self.metrics.total_wait_count);
        out.push_str(",\"offline_count\":");
        push_u64(&mut out, self.metrics.offline_count);
        out.push_str(",\"runtime_ms\":");
        push_option_u128(&mut out, self.metrics.runtime_ms);
        out.push_str("}}");
        out
    }
}

fn push_usize(buf: &mut String, v: usize) {
    use std::fmt::Write;
    let _ = write!(buf, "{v}");
}

fn push_u64(buf: &mut String, v: u64) {
    use std::fmt::Write;
    let _ = write!(buf, "{v}");
}

fn push_option_usize(buf: &mut String, v: Option<usize>) {
    match v {
        Some(n) => push_usize(buf, n),
        None => buf.push_str("null"),
    }
}

fn push_option_u64(buf: &mut String, v: Option<u64>) {
    match v {
        Some(n) => push_u64(buf, n),
        None => buf.push_str("null"),
    }
}

fn push_option_u128(buf: &mut String, v: Option<u128>) {
    use std::fmt::Write;
    match v {
        Some(n) => {
            let _ = write!(buf, "{n}");
        }
        None => buf.push_str("null"),
    }
}

fn json_escape_into(buf: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '\\' => buf.push_str("\\\\"),
            '"' => buf.push_str("\\\""),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            _ => buf.push(ch),
        }
    }
}
