//! Robot worker thread logic.
//!
//! The public entry point is [`robot_worker`], which is meant to be called
//! inside `std::thread::spawn`.  It loops: fetch a task, enter the target
//! zone, simulate work, leave the zone, and repeat until the queue shuts
//! down.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::event_log::{EventKind, EventLog};
use crate::health_monitor::HealthMonitor;
use crate::task_queue::TaskQueue;
use crate::traits::{HeartbeatRegistry, TaskProvider, ZoneAccess};
use crate::types::RobotId;
use crate::zone_manager::ZoneManager;

/// Run the main loop for a single robot worker.
///
/// This function is designed to be called inside `std::thread::spawn`.
/// It returns when the task queue is shut down and drained.
///
/// # Arguments
///
/// * `id` — Unique identifier for this robot.
/// * `task_queue` — Shared task queue to pop work from.
/// * `zone_manager` — Shared zone access controller.
/// * `health_monitor` — Shared health registry for heartbeats.
/// * `event_log` — Shared structured event log.
/// * `fail_flag` — If `Some` and set to `true`, the worker stops sending
///   heartbeats, simulating a crash for the timeout demo.
pub fn robot_worker(
    id: RobotId,
    task_queue: Arc<TaskQueue>,
    zone_manager: Arc<ZoneManager>,
    health_monitor: Arc<HealthMonitor>,
    event_log: Arc<EventLog>,
    fail_flag: Option<Arc<AtomicBool>>,
) {
    event_log.log(EventKind::RobotStarted { robot_id: id });

    loop {
        let should_skip_heartbeat = fail_flag
            .as_ref()
            .is_some_and(|f| f.load(Ordering::Relaxed));

        if !should_skip_heartbeat {
            health_monitor.heartbeat(id);
        }

        let task = match task_queue.pop_task_blocking() {
            Some(t) => t,
            None => {
                event_log.log(EventKind::RobotStopped { robot_id: id });
                return;
            }
        };

        let task_id = task.id;
        let zone = task.target_zone;
        let duration = task.duration_ms;

        event_log.log(EventKind::TaskReceived {
            robot_id: id,
            task_id,
        });

        health_monitor.update_task(id, Some(task_id));

        if zone_manager.is_occupied(zone) {
            event_log.log(EventKind::ZoneWaiting {
                robot_id: id,
                zone,
            });
        }

        zone_manager.enter_zone(zone, id);
        event_log.log(EventKind::ZoneEntered {
            robot_id: id,
            zone,
        });
        health_monitor.update_zone(id, Some(zone));

        thread::sleep(Duration::from_millis(duration));

        let _ = zone_manager.leave_zone(zone, id);
        event_log.log(EventKind::ZoneLeft {
            robot_id: id,
            zone,
        });
        health_monitor.update_zone(id, None);

        event_log.log(EventKind::TaskCompleted {
            robot_id: id,
            task_id,
        });
        health_monitor.update_task(id, None);
    }
}
