use std::sync::Arc;
use std::thread;
use std::time::Duration;

use blaze_core::step_gate::StepGate;

#[test]
fn step_gate_paused_blocks_until_step() {
    let gate = Arc::new(StepGate::new_paused());
    assert!(gate.is_paused());

    let g = Arc::clone(&gate);
    let handle = thread::spawn(move || {
        g.wait_before_event();
        "released"
    });

    thread::sleep(Duration::from_millis(50));
    gate.step();
    let result = handle.join().expect("thread should join");
    assert_eq!(result, "released");
}

#[test]
fn step_gate_resume_unblocks_all() {
    let gate = Arc::new(StepGate::new_paused());
    let mut handles = Vec::new();
    for _ in 0..3 {
        let g = Arc::clone(&gate);
        handles.push(thread::spawn(move || {
            g.wait_before_event();
            "released"
        }));
    }

    thread::sleep(Duration::from_millis(30));
    gate.resume();
    for h in handles {
        assert_eq!(h.join().expect("join"), "released");
    }
}

#[test]
fn step_gate_running_does_not_block() {
    let gate = Arc::new(StepGate::new_running());
    assert!(!gate.is_paused());
    gate.wait_before_event();
    gate.wait_before_event();
}

#[test]
fn step_gate_pause_and_step_one_at_a_time() {
    let gate = Arc::new(StepGate::new_running());
    gate.pause();

    let g = Arc::clone(&gate);
    let h1 = thread::spawn(move || {
        g.wait_before_event();
        1
    });

    thread::sleep(Duration::from_millis(30));
    gate.step();
    assert_eq!(h1.join().expect("join"), 1);

    let g = Arc::clone(&gate);
    let h2 = thread::spawn(move || {
        g.wait_before_event();
        2
    });
    thread::sleep(Duration::from_millis(30));
    gate.step();
    assert_eq!(h2.join().expect("join"), 2);
}
