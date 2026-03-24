//! Optional step gate for manual event-by-event time control.
//!
//! When attached to a coordinator, robot workers and the health monitor
//! block before emitting each event until a step is released. Used by
//! the web dashboard for step-by-step demonstration mode.
//!
//! Lock order: 5 (TaskQueue < ZoneManager < HealthMonitor < EventLog < StepGate).
//!
//! Do not call `on_wait` (or any code that may block or take other locks) while
//! holding [`StepGate`]'s mutex. [`wait_before_event_with_heartbeat`] drops the
//! guard after each timed wait before invoking the callback.

use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// Gate that blocks event emission until a step is released.
///
/// Used when manual mode is enabled: robots block before each
/// `event_log.log()` until `release_step()` is called.
pub struct StepGate {
    condvar: Condvar,
    state: Mutex<StepState>,
}

struct StepState {
    paused: bool,
    steps_remaining: usize,
}

impl StepGate {
    /// Create a new step gate in paused state (no steps available).
    pub fn new_paused() -> Self {
        Self {
            condvar: Condvar::new(),
            state: Mutex::new(StepState {
                paused: true,
                steps_remaining: 0,
            }),
        }
    }

    /// Create a new step gate in running state (no blocking).
    pub fn new_running() -> Self {
        Self {
            condvar: Condvar::new(),
            state: Mutex::new(StepState {
                paused: false,
                steps_remaining: 0,
            }),
        }
    }

    /// Block until allowed to emit one event. Returns immediately if not paused.
    pub fn wait_before_event(&self) {
        let mut guard = self.state.lock().expect("step gate lock poisoned");
        while guard.paused && guard.steps_remaining == 0 {
            guard = self.condvar.wait(guard).expect("step gate wait");
        }
        if guard.paused && guard.steps_remaining > 0 {
            guard.steps_remaining -= 1;
        }
    }

    /// Block until allowed to emit one event, calling `on_wait` periodically
    /// while waiting so the caller can send heartbeats.
    ///
    /// `on_wait` runs only after releasing the step-gate mutex so heartbeat
    /// does not nest inside this lock.
    pub fn wait_before_event_with_heartbeat<F>(&self, mut on_wait: F)
    where
        F: FnMut(),
    {
        let timeout = Duration::from_millis(500);
        loop {
            let mut guard = self.state.lock().expect("step gate lock poisoned");
            if !(guard.paused && guard.steps_remaining == 0) {
                if guard.paused && guard.steps_remaining > 0 {
                    guard.steps_remaining -= 1;
                }
                return;
            }
            let (g, _) = self
                .condvar
                .wait_timeout(guard, timeout)
                .expect("step gate wait");
            drop(g);
            on_wait();
        }
    }

    /// Pause: future events will block until step or resume.
    pub fn pause(&self) {
        let mut guard = self.state.lock().expect("step gate lock poisoned");
        guard.paused = true;
    }

    /// Release one step: one waiting thread may proceed.
    pub fn step(&self) {
        let mut guard = self.state.lock().expect("step gate lock poisoned");
        guard.steps_remaining = guard.steps_remaining.saturating_add(1);
        self.condvar.notify_one();
    }

    /// Resume: all future events proceed without blocking.
    pub fn resume(&self) {
        let mut guard = self.state.lock().expect("step gate lock poisoned");
        guard.paused = false;
        guard.steps_remaining = 0;
        self.condvar.notify_all();
    }

    /// Return whether the gate is currently paused.
    pub fn is_paused(&self) -> bool {
        let guard = self.state.lock().expect("step gate lock poisoned");
        guard.paused
    }
}
