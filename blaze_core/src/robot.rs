//! Robot worker thread logic.
//! The public entry point is [`robot_worker`], which is meant to be called
//! inside `std::thread::spawn`.  It loops: fetch a task, enter the target
//! zone, simulate work, leave the zone, and repeat until the queue shuts
//! down.
//!
//! OS concept demonstrated: cooperative priority scheduling.
//! Robots executing preemptible Normal tasks periodically check whether an
//! Urgent task is pending. If so, the robot cooperatively yields: it releases
//! the zone, requeues the interrupted task as restartable, and loops back to
//! pop the higher-priority work. This is *not* hard preemption — the robot
//! voluntarily checks at safe points during chunked execution.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::event_log::{EventKind, EventLog};
use crate::health_monitor::HealthMonitor;
use crate::metrics::Metrics;
use crate::step_gate::StepGate;
use crate::task::Task;
use crate::task_queue::TaskQueue;
use crate::traits::{HeartbeatRegistry, ZoneAccess};
use crate::types::{RobotId, RobotStatus, TaskPriority, DEFAULT_HEARTBEAT_INTERVAL_MS};
use crate::zone_manager::ZoneManager;

/// Result of the interruptible work sleep.
enum InterruptReason {
    /// The robot was marked offline by the health monitor.
    Offline,
    /// OS concept demonstrated: cooperative priority scheduling.
    /// OS concept demonstrated: interruptible execution of low-priority work.
    /// The robot voluntarily yielded a preemptible Normal task because an
    /// Urgent task arrived in the queue.
    Yielded,
    /// Work completed without interruption.
    Completed,
}

/// Run the main loop for a single robot worker.
///
/// This function is designed to be called inside `std::thread::spawn`.
/// It returns when the task queue is shut down and drained.
///
/// Parameters:
/// `id` is the robot identifier.
/// `task_queue` provides incoming tasks.
/// `zone_manager` controls zone ownership.
/// `health_monitor` tracks liveness and status.
/// `event_log` records structured events.
/// `metrics` collects runtime counters.
/// `fail_flag` lets tests simulate a crash by stopping heartbeats.
/// `step_gate` optionally pauses before each event for manual stepping.
pub fn robot_worker(
    id: RobotId,
    task_queue: Arc<TaskQueue>,
    zone_manager: Arc<ZoneManager>,
    health_monitor: Arc<HealthMonitor>,
    event_log: Arc<EventLog>,
    metrics: Arc<Metrics>,
    fail_flag: Option<Arc<AtomicBool>>,
    step_gate: Option<Arc<StepGate>>,
) {
    let hm_check = Arc::clone(&health_monitor);
    let is_offline = move || hm_check.status(id) == Some(RobotStatus::Offline);

    let mut send_heartbeat = || {
        if !fail_flag
            .as_ref()
            .is_some_and(|f| f.load(Ordering::Relaxed))
        {
            health_monitor.heartbeat(id);
        }
    };

    maybe_wait(&step_gate, &mut send_heartbeat);
    event_log.log(EventKind::RobotStarted { robot_id: id });

    loop {
        // Critical sync point: keep step gate before `pop`.
        // That keeps `pop` and `TaskReceived` in one release and avoids a
        // brief UI mismatch where task is gone but receive event is not shown.
        maybe_wait(&step_gate, &mut send_heartbeat);
        let task = match task_queue.pop_task_blocking_with_heartbeat(&mut send_heartbeat) {
            Some(t) => t,
            None => {
                event_log.log(EventKind::RobotStopped { robot_id: id });
                return;
            }
        };

        health_monitor.register(id);

        if !fail_flag
            .as_ref()
            .is_some_and(|f| f.load(Ordering::Relaxed))
        {
            health_monitor.heartbeat(id);
        }

        let task_id = task.id;
        let zone = task.target_zone;
        let duration = task.duration_ms;
        let priority = task.priority;
        let kind = task.kind;
        let preemptible = task.preemptible;

        event_log.log(EventKind::TaskReceived {
            robot_id: id,
            task_id,
        });

        health_monitor.update_task(id, Some(task_id));

        if zone_manager.is_occupied(zone) {
            zone_manager.add_to_waiting_if_occupied(zone, id);
            maybe_wait(&step_gate, &mut send_heartbeat);
            event_log.log(EventKind::ZoneWaiting {
                robot_id: id,
                zone,
            });
            metrics.record_zone_wait(id, zone);
        }

        maybe_wait(&step_gate, &mut send_heartbeat);
        if is_offline() {
            let reclaimed = Task {
                id: task_id, priority, kind, target_zone: zone,
                duration_ms: duration, preemptible,
            };
            task_queue.reclaim_task(reclaimed);
            event_log.log(EventKind::TaskReclaimed {
                robot_id: id,
                task_id,
            });
            health_monitor.update_task(id, None);
            maybe_wait(&step_gate, &mut send_heartbeat);
            event_log.log(EventKind::RobotStopped { robot_id: id });
            return;
        }
        let entered = zone_manager.enter_zone_with_heartbeat(zone, id, || {
            send_heartbeat();
            !is_offline()
        });
        if !entered {
            let reclaimed = Task {
                id: task_id, priority, kind, target_zone: zone,
                duration_ms: duration, preemptible,
            };
            task_queue.reclaim_task(reclaimed);
            maybe_wait(&step_gate, &mut send_heartbeat);
            event_log.log(EventKind::TaskReclaimed {
                robot_id: id,
                task_id,
            });
            health_monitor.update_task(id, None);
            maybe_wait(&step_gate, &mut send_heartbeat);
            event_log.log(EventKind::RobotStopped { robot_id: id });
            return;
        }
        event_log.log(EventKind::ZoneEntered {
            robot_id: id,
            zone,
        });
        health_monitor.update_zone(id, Some(zone));

        // Interruptible low-priority execution.
        // During chunked sleep, the worker checks:
        // 1) offline status -> reclaim and stop
        // 2) urgent pending on preemptible Normal task -> yield and requeue
        let should_yield = preemptible && priority == TaskPriority::Normal;
        let result = sleep_with_interrupt_check(
            duration,
            &mut send_heartbeat,
            &is_offline,
            should_yield,
            &task_queue,
        );

        match result {
            InterruptReason::Offline => {
                // Offline reclamation path (unchanged from before).
                maybe_wait(&step_gate, &mut send_heartbeat);
                let _ = zone_manager.leave_zone(zone, id);
                event_log.log(EventKind::ZoneLeft {
                    robot_id: id,
                    zone,
                });
                health_monitor.update_zone(id, None);

                let reclaimed = Task {
                    id: task_id, priority, kind, target_zone: zone,
                    duration_ms: duration, preemptible,
                };
                task_queue.reclaim_task(reclaimed);

                maybe_wait(&step_gate, &mut send_heartbeat);
                event_log.log(EventKind::TaskReclaimed {
                    robot_id: id,
                    task_id,
                });
                health_monitor.update_task(id, None);

                maybe_wait(&step_gate, &mut send_heartbeat);
                event_log.log(EventKind::RobotStopped { robot_id: id });
                return;
            }

            InterruptReason::Yielded => {
                // Design simplification:
                // yielded tasks are requeued as restartable work units rather than resumed
                // from partial progress. This keeps the scheduling logic simple and explicit.
                //
                // Cooperative yield path:
                // release zone -> log yield -> requeue at front -> clear state.
                maybe_wait(&step_gate, &mut send_heartbeat);
                let _ = zone_manager.leave_zone(zone, id);
                event_log.log(EventKind::ZoneLeft {
                    robot_id: id,
                    zone,
                });
                health_monitor.update_zone(id, None);

                maybe_wait(&step_gate, &mut send_heartbeat);
                event_log.log(EventKind::TaskYielded {
                    robot_id: id,
                    task_id,
                });

                let reclaimed = Task {
                    id: task_id, priority, kind, target_zone: zone,
                    duration_ms: duration, preemptible,
                };
                task_queue.reclaim_task(reclaimed);

                health_monitor.update_task(id, None);

                // loop back to pop the urgent task.
                continue;
            }

            InterruptReason::Completed => {
                // Normal completion path.
                maybe_wait(&step_gate, &mut send_heartbeat);
                let _ = zone_manager.leave_zone(zone, id);
                event_log.log(EventKind::ZoneLeft {
                    robot_id: id,
                    zone,
                });
                health_monitor.update_zone(id, None);

                maybe_wait(&step_gate, &mut send_heartbeat);
                event_log.log(EventKind::TaskCompleted {
                    robot_id: id,
                    task_id,
                });
                metrics.record_task_completed(id);
                health_monitor.update_task(id, None);
            }
        }
    }
}

#[inline]
fn maybe_wait<F>(gate: &Option<Arc<StepGate>>, on_wait: &mut F)
where
    F: FnMut(),
{
    if let Some(g) = gate {
        g.wait_before_event_with_heartbeat(on_wait);
    }
}

/// Sleep for `duration_ms` in small chunks while sending heartbeats.
///
/// After each chunk, it checks two interrupt conditions.
/// Offline means the health monitor marked this robot offline.
/// Cooperative yield means `yield_enabled` is true and an Urgent task is
/// pending in `task_queue`.
///
/// Returns the reason execution stopped (or `Completed` if the full duration
/// elapsed without interruption).
fn sleep_with_interrupt_check<F, G>(
    duration_ms: u64,
    on_wait: &mut F,
    is_offline: &G,
    yield_enabled: bool,
    task_queue: &TaskQueue,
) -> InterruptReason
where
    F: FnMut(),
    G: Fn() -> bool,
{
    let chunk_ms = DEFAULT_HEARTBEAT_INTERVAL_MS;
    let mut remaining = duration_ms;
    while remaining > 0 {
        let sleep_ms = remaining.min(chunk_ms);
        thread::sleep(Duration::from_millis(sleep_ms));
        remaining -= sleep_ms;

        if is_offline() {
            return InterruptReason::Offline;
        }

        // Cooperative priority scheduling check.
        // The robot checks whether higher-priority work has arrived.
        // Only preemptible Normal tasks participate; Urgent and non-preemptible Normal run to completion.
        if yield_enabled && task_queue.has_urgent_pending() {
            return InterruptReason::Yielded;
        }

        if remaining > 0 {
            on_wait();
        }
    }
    InterruptReason::Completed
}
