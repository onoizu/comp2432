//! Error types used by the Blaze core.
//!
//! We keep this enum small on purpose: only domain-level failures are
//! represented here. Internal lock poisoning is treated as fatal and is not
//! passed around as a normal recoverable error.

use std::fmt;

use crate::types::{RobotId, ZoneId};

/// Errors that can happen during normal Blaze operations.
#[derive(Debug)]
pub enum BlazeError {

    /// The robot tried to leave a zone owned by someone else.
    ZoneNotOwned {
        zone: ZoneId,
        robot: RobotId,
    },

    /// The requested robot ID was never registered in the system.
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
