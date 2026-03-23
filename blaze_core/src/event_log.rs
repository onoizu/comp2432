//! Structured, thread-safe event log.
//!
//! Every significant action in the system (task received, zone entered,
//! robot timed out, etc.) is recorded as an [`Event`] with a typed
//! [`EventKind`].  This makes test assertions stable and avoids fragile
//! string matching.
//!
//! Lock order: 4 (TaskQueue < ZoneManager < HealthMonitor < EventLog < StepGate).

use std::fmt;
use std::sync::Mutex;
use std::time::Instant;

use crate::types::{RobotId, TaskId, ZoneId};


/// EventKind

/// Discriminated union of every event the system can produce.
///
/// All fields are `Copy` types so the whole enum is `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    RobotStarted { robot_id: RobotId },
    RobotStopped { robot_id: RobotId },
    TaskReceived { robot_id: RobotId, task_id: TaskId },
    ZoneWaiting { robot_id: RobotId, zone: ZoneId },
    ZoneEntered { robot_id: RobotId, zone: ZoneId },
    ZoneLeft { robot_id: RobotId, zone: ZoneId },
    TaskCompleted { robot_id: RobotId, task_id: TaskId },
    TaskReclaimed { robot_id: RobotId, task_id: TaskId },
    /// Cooperative priority yield: robot voluntarily stopped a preemptible
    /// Normal task because an Urgent task arrived.
    TaskYielded { robot_id: RobotId, task_id: TaskId },
    RobotTimedOut { robot_id: RobotId },
    SystemShutdown,
}

/// Stable event category used by terminal formatting and exports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCategory {
    Robot,
    Task,
    Zone,
    Health,
    System,
}

impl EventCategory {
    /// Return a fixed-width category tag
    pub fn tag(self) -> &'static str {
        match self {
            EventCategory::Robot => "ROBOT ",
            EventCategory::Task => "TASK  ",
            EventCategory::Zone => "ZONE  ",
            EventCategory::Health => "HEALTH",
            EventCategory::System => "SYSTEM",
        }
    }
}

impl EventKind {
    /// Return the category of this event kind.
    pub fn category(&self) -> EventCategory {
        match self {
            EventKind::RobotStarted { .. } | EventKind::RobotStopped { .. } => EventCategory::Robot,
            EventKind::TaskReceived { .. } | EventKind::TaskCompleted { .. } | EventKind::TaskReclaimed { .. } | EventKind::TaskYielded { .. } => EventCategory::Task,
            EventKind::ZoneWaiting { .. } | EventKind::ZoneEntered { .. } | EventKind::ZoneLeft { .. } => {
                EventCategory::Zone
            }
            EventKind::RobotTimedOut { .. } => EventCategory::Health,
            EventKind::SystemShutdown => EventCategory::System,
        }
    }

    /// Return a stable machine-friendly event code.
    pub fn code(&self) -> &'static str {
        match self {
            EventKind::RobotStarted { .. } => "ROBOT_STARTED",
            EventKind::RobotStopped { .. } => "ROBOT_STOPPED",
            EventKind::TaskReceived { .. } => "TASK_RECEIVED",
            EventKind::ZoneWaiting { .. } => "ZONE_WAITING",
            EventKind::ZoneEntered { .. } => "ZONE_ENTERED",
            EventKind::ZoneLeft { .. } => "ZONE_LEFT",
            EventKind::TaskCompleted { .. } => "TASK_COMPLETED",
            EventKind::TaskReclaimed { .. } => "TASK_RECLAIMED",
            EventKind::TaskYielded { .. } => "TASK_YIELDED",
            EventKind::RobotTimedOut { .. } => "ROBOT_TIMED_OUT",
            EventKind::SystemShutdown => "SYSTEM_SHUTDOWN",
        }
    }
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventKind::RobotStarted { robot_id } => {
                write!(f, "Robot {robot_id} started")
            }
            EventKind::RobotStopped { robot_id } => {
                write!(f, "Robot {robot_id} stopped")
            }
            EventKind::TaskReceived { robot_id, task_id } => {
                write!(f, "Robot {robot_id} received task {task_id}")
            }
            EventKind::ZoneWaiting { robot_id, zone } => {
                write!(f, "Robot {robot_id} waiting for {zone}")
            }
            EventKind::ZoneEntered { robot_id, zone } => {
                write!(f, "Robot {robot_id} entered {zone}")
            }
            EventKind::ZoneLeft { robot_id, zone } => {
                write!(f, "Robot {robot_id} left {zone}")
            }
            EventKind::TaskCompleted { robot_id, task_id } => {
                write!(f, "Robot {robot_id} completed task {task_id}")
            }
            EventKind::TaskReclaimed { robot_id, task_id } => {
                write!(f, "Task {task_id} reclaimed from Robot {robot_id}")
            }
            EventKind::TaskYielded { robot_id, task_id } => {
                write!(f, "Robot {robot_id} yielded task {task_id} (cooperative preemption)")
            }
            EventKind::RobotTimedOut { robot_id } => {
                write!(f, "Robot {robot_id} timed out")
            }
            EventKind::SystemShutdown => write!(f, "System shutdown"),
        }
    }
}


///EventLog


/// A single recorded event with a timestamp.
pub struct Event {
    pub timestamp: Instant,
    pub kind: EventKind,
}

/// Export-friendly timeline row with relative time and stable metadata.
#[derive(Debug, Clone)]
pub struct TimelineRow {
    pub elapsed_ms: u128,
    pub category: EventCategory,
    pub code: &'static str,
    pub message: String,
}

/// Append-only, thread-safe event log.
pub struct EventLog {
    events: Mutex<Vec<Event>>,
    start: Instant,
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            start: Instant::now(),
        }
    }

    /// Record an event with the current timestamp.
    pub fn log(&self, kind: EventKind) {
        let mut guard = self.events.lock().expect("event log lock poisoned");
        guard.push(Event {
            timestamp: Instant::now(),
            kind,
        });
    }

    /// Return a snapshot of all recorded events (clones the kind, copies the timestamp).
    pub fn events(&self) -> Vec<EventKind> {
        let guard = self.events.lock().expect("event log lock poisoned");
        guard.iter().map(|e| e.kind).collect()
    }

    /// Check whether any recorded event satisfies `predicate`.
    pub fn has_event(&self, predicate: impl Fn(&EventKind) -> bool) -> bool {
        let guard = self.events.lock().expect("event log lock poisoned");
        guard.iter().any(|e| predicate(&e.kind))
    }

    /// Return a human-readable dump of all events with relative timestamps.
    pub fn dump(&self) -> String {
        let guard = self.events.lock().expect("event log lock poisoned");
        let mut buf = String::new();
        for event in guard.iter() {
            let elapsed = event.timestamp.duration_since(self.start);
            buf.push_str(&format!(
                "[{:>6}ms] {}\n",
                elapsed.as_millis(),
                event.kind,
            ));
        }
        buf
    }

    /// Return event dump with category tags for terminal demos
    pub fn dump_pretty(&self) -> String {
        let guard = self.events.lock().expect("event log lock poisoned");
        let mut buf = String::new();
        for event in guard.iter() {
            let elapsed = event.timestamp.duration_since(self.start).as_millis();
            let category = event.kind.category().tag();
            buf.push_str(&format!(
                "[{:04}ms][{}] {}\n",
                elapsed,
                category,
                event.kind,
            ));
        }
        buf
    }

    /// Return export-friendly timeline rows.
    pub fn timeline(&self) -> Vec<TimelineRow> {
        let guard = self.events.lock().expect("event log lock poisoned");
        guard
            .iter()
            .map(|event| TimelineRow {
                elapsed_ms: event.timestamp.duration_since(self.start).as_millis(),
                category: event.kind.category(),
                code: event.kind.code(),
                message: event.kind.to_string(),
            })
            .collect()
    }

    /// Return the number of events recorded so far.
    pub fn event_count(&self) -> usize {
        let guard = self.events.lock().expect("event log lock poisoned");
        guard.len()
    }

    /// Return timeline rows starting from `start_index` 
    pub fn timeline_since(&self, start_index: usize) -> Vec<TimelineRow> {
        let guard = self.events.lock().expect("event log lock poisoned");
        guard
            .iter()
            .skip(start_index)
            .map(|event| TimelineRow {
                elapsed_ms: event.timestamp.duration_since(self.start).as_millis(),
                category: event.kind.category(),
                code: event.kind.code(),
                message: event.kind.to_string(),
            })
            .collect()
    }

    /// Serialize incremental events to JSON for the web API.
    pub fn events_json_since(&self, start_index: usize) -> String {
        let guard = self.events.lock().expect("event log lock poisoned");
        let total = guard.len();
        let mut out = String::with_capacity(1024);
        out.push_str("{\"events\":[");
        let mut first = true;
        for (idx, event) in guard.iter().enumerate().skip(start_index) {
            if !first {
                out.push(',');
            }
            first = false;
            let elapsed = event.timestamp.duration_since(self.start).as_millis();
            let cat = event.kind.category().tag();
            let code = event.kind.code();
            let msg = event.kind.to_string();
            out.push_str("{\"index\":");
            out.push_str(&idx.to_string());
            out.push_str(",\"elapsed_ms\":");
            out.push_str(&elapsed.to_string());
            out.push_str(",\"category\":\"");
            out.push_str(cat);
            out.push_str("\",\"code\":\"");
            out.push_str(code);
            out.push_str("\",\"message\":\"");
            json_escape_event(&mut out, &msg);
            out.push_str("\"}");
        }
        out.push_str("],\"total_count\":");
        out.push_str(&total.to_string());
        out.push('}');
        out
    }
}

fn json_escape_event(buf: &mut String, s: &str) {
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
