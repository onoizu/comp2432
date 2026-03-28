//! Core library for Project Blaze.
//!
//! This crate follows the minimal scope in the Project-B OS brief:
//! task queue, zone access control, health monitoring, and coordination.

pub mod types;
pub mod errors;
pub mod traits;
pub mod task;
pub mod task_queue;
pub mod zone_manager;
pub mod health_monitor;
pub mod robot;
pub mod coordinator;
pub mod event_log;
pub mod metrics;
pub mod step_gate;
pub mod summary;
