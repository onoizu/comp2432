//! Structured, thread-safe event log.
//!
//! Every significant action in the system (task received, zone entered,
//! robot timed out, etc.) is recorded as an [`Event`] with a typed
//! [`EventKind`].  This makes test assertions stable and avoids fragile
//! string matching.

use std::fmt;
use std::sync::Mutex;
use std::time::Instant;

use crate::types::{RobotId, TaskId, ZoneId};

// ---------------------------------------------------------------------------
// EventKind
// ---------------------------------------------------------------------------

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
    RobotTimedOut { robot_id: RobotId },
    SystemShutdown,
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
            EventKind::RobotTimedOut { robot_id } => {
                write!(f, "Robot {robot_id} timed out")
            }
            EventKind::SystemShutdown => write!(f, "System shutdown"),
        }
    }
}

// ---------------------------------------------------------------------------
// Event / EventLog
// ---------------------------------------------------------------------------

/// A single recorded event with a timestamp.
pub struct Event {
    pub timestamp: Instant,
    pub kind: EventKind,
}

/// Append-only, thread-safe event log.
pub struct EventLog {
    events: Mutex<Vec<Event>>,
    start: Instant,
}

impl EventLog {
    /// Create a new, empty event log.
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

    /// Return a snapshot of all recorded events (clones the kind, copies
    /// the timestamp).
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
}
