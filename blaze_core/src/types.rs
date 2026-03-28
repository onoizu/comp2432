//! Shared IDs, enums, and constants used across Blaze.

use std::fmt;

/// Numeric ID for a robot worker.
pub type RobotId = usize;

/// Numeric ID for a task.
pub type TaskId = u64;




/// Constants

/// Max silence time before a robot is marked offline (milliseconds).
pub const DEFAULT_HEARTBEAT_TIMEOUT_MS: u64 = 3000;

/// How often a healthy robot sends a heartbeat (milliseconds).
pub const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 500;

/// How often the monitor checks for timed-out robots (milliseconds).
pub const DEFAULT_MONITOR_INTERVAL_MS: u64 = 1000;




/// ZoneId

/// A physical hospital zone that a robot can enter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneId {
    EmergencyRoom,
    PharmacyHall,
    WardA,
    WardB,
    Lobby,
}

impl ZoneId {
    /// Returns every zone as a fixed list.
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

/// Task priority. Urgent tasks are always taken before normal tasks.
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

/// What kind of work the robot should do.
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

/// Current liveness state reported by the health monitor.
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
