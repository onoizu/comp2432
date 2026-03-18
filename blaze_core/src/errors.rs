//! Unified error type for Project Blaze.
//!
//! Only operations that genuinely can fail surface errors here.  Lock
//! poisoning is treated as unrecoverable and handled via `.expect()` at
//! call sites rather than being propagated through this type.

use std::fmt;

use crate::types::{RobotId, ZoneId};

/// Errors that can occur during Blaze operations.
#[derive(Debug)]
pub enum BlazeError {
    /// A robot tried to leave a zone it does not currently own.
    ZoneNotOwned {
        zone: ZoneId,
        robot: RobotId,
    },
    /// An operation referenced a robot that has not been registered.
    RobotNotRegistered(RobotId),
}

impl fmt::Display for BlazeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlazeError::ZoneNotOwned { zone, robot } => {
                write!(f, "robot {robot} does not own zone {zone}")
            }
            BlazeError::RobotNotRegistered(id) => {
                write!(f, "robot {id} is not registered")
            }
        }
    }
}

impl std::error::Error for BlazeError {}
