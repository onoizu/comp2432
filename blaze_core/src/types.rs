//! Shared type aliases, enums, and constants used throughout Project Blaze.

use std::fmt;

/// Unique identifier for a robot worker thread.
pub type RobotId = usize;

/// Unique identifier for a task.
pub type TaskId = u64;




/// Constants

/// How long a robot may be silent before the monitor marks it offline (ms).
pub const DEFAULT_HEARTBEAT_TIMEOUT_MS: u64 = 3000;

/// How often a healthy robot sends a heartbeat (ms).
pub const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 500;

/// How often the monitor thread scans for timed-out robots (ms).
pub const DEFAULT_MONITOR_INTERVAL_MS: u64 = 1000;




/// ZoneId

/// Identifies a physical zone inside the hospital.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneId {
    EmergencyRoom,
    PharmacyHall,
    WardA,
    WardB,
    Lobby,
}

impl ZoneId {
    /// Returns a slice containing every `ZoneId` variant.
    pub fn all() -> &'static [ZoneId] {
        &[
            ZoneId::EmergencyRoom,
            ZoneId::PharmacyHall,
            ZoneId::WardA,
            ZoneId::WardB,
            ZoneId::Lobby,
        ]
    }
}

impl fmt::Display for ZoneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZoneId::EmergencyRoom => write!(f, "EmergencyRoom"),
            ZoneId::PharmacyHall => write!(f, "PharmacyHall"),
            ZoneId::WardA => write!(f, "WardA"),
            ZoneId::WardB => write!(f, "WardB"),
            ZoneId::Lobby => write!(f, "Lobby"),
        }
    }
}




/// TaskPriority

/// Priority level of a task. Urgent tasks are dequeued before normal ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskPriority {
    Urgent,
    Normal,
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskPriority::Urgent => write!(f, "Urgent"),
            TaskPriority::Normal => write!(f, "Normal"),
        }
    }
}


/// TaskKind

/// The type of work a task represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    Delivery,
    Cleaning,
    Inspection,
    Emergency,
}

impl fmt::Display for TaskKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskKind::Delivery => write!(f, "Delivery"),
            TaskKind::Cleaning => write!(f, "Cleaning"),
            TaskKind::Inspection => write!(f, "Inspection"),
            TaskKind::Emergency => write!(f, "Emergency"),
        }
    }
}



/// RobotStatus

/// Whether a robot is considered alive by the health monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotStatus {
    Online,
    Offline,
}

impl fmt::Display for RobotStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RobotStatus::Online => write!(f, "Online"),
            RobotStatus::Offline => write!(f, "Offline"),
        }
    }
}
