//! Lightweight local HTTP server for the Blaze web dashboard.
//!
//! Uses only `std::net::TcpListener` — no external HTTP crate needed.
//! All endpoints are read-only views of core state plus a controlled
//! scenario-start trigger.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use blaze_core::coordinator::Coordinator;
use blaze_core::event_log::EventLog;
use blaze_core::metrics::Metrics;
use blaze_core::step_gate::StepGate;
use blaze_core::summary::SystemSnapshot;
use blaze_core::types::DEFAULT_HEARTBEAT_TIMEOUT_MS;
use blaze_sim::demo;

const INDEX_HTML: &str = include_str!("../web/index.html");
const STYLES_CSS: &str = include_str!("../web/styles.css");
const APP_JS: &str = include_str!("../web/app.js");

struct AppState {
    coordinator: Option<Coordinator>,
    event_log: Option<Arc<EventLog>>,
    metrics: Option<Arc<Metrics>>,
    step_gate: Option<Arc<StepGate>>,
    scenario_name: String,
    running: bool,
    last_snapshot: Option<SystemSnapshot>,
}

impl AppState {
    fn new() -> Self {
        Self {
            coordinator: None,
            event_log: None,
            metrics: None,
            step_gate: None,
            scenario_name: String::new(),
            running: false,
            last_snapshot: None,
        }
    }

    fn start_scenario_by_id(&mut self, id: &str, manual: bool) {
        if let Some(ref g) = self.step_gate {
            g.resume();
        }
        if let Some(coord) = self.coordinator.take() {
            self.last_snapshot = Some(coord.snapshot());
            coord.shutdown();
        }
        self.step_gate = None;

        let scenario = match demo::build_scenario(id) {
            Some(s) => s,
            None => return,
        };

        let timeout_ms = scenario.heartbeat_timeout_ms.unwrap_or(DEFAULT_HEARTBEAT_TIMEOUT_MS);
        let mut coord = Coordinator::new(Duration::from_millis(timeout_ms));

        if manual {
            let gate = Arc::new(StepGate::new_paused());
            coord.set_step_gate(Arc::clone(&gate));
            self.step_gate = Some(gate);
        }

        let metrics = coord.metrics();
        let event_log = coord.event_log();

        metrics.start_scenario(scenario.name);
        coord.start_monitor();

        let fail_flag = Arc::new(AtomicBool::new(false));

        // Phase 1: start non-failing robots first, then submit initial tasks.
        for robot_id in 0..scenario.robot_count {
            if scenario.fail_robot_id == Some(robot_id) {
                continue;
            }
            coord.spawn_robot(robot_id, None);
        }
        for task in scenario.tasks {
            coord.submit_task(task);
        }

        // Phase 2: after delay, start failing robot and submit late tasks.
        if let Some(delay) = scenario.late_delay_ms {
            thread::sleep(Duration::from_millis(delay));
        }
        if let Some(fid) = scenario.fail_robot_id {
            coord.spawn_robot(fid, Some(Arc::clone(&fail_flag)));
        }
        for task in scenario.late_tasks {
            coord.submit_task(task);
        }

        // `fail_after_ms` starts counting from here (after Phase 2).
        if let Some(after_ms) = scenario.fail_after_ms {
            let ff = Arc::clone(&fail_flag);
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(after_ms));
                ff.store(true, Ordering::Relaxed);
            });
        }

        self.event_log = Some(event_log);
        self.metrics = Some(metrics);
        self.scenario_name = scenario.name.to_string();
        self.running = true;
        self.coordinator = Some(coord);
    }

    fn time_pause(&mut self) {
        if let Some(ref g) = self.step_gate {
            g.pause();
        }
    }

    fn time_resume(&mut self) {
        if let Some(ref g) = self.step_gate {
            g.resume();
        }
    }

    fn time_step(&mut self) {
        if let Some(ref g) = self.step_gate {
            g.step();
        }
    }

    fn time_state_json(&self) -> String {
        let paused = self
            .step_gate
            .as_ref()
            .map(|g| g.is_paused())
            .unwrap_or(false);
        let manual_mode = self.step_gate.is_some();
        format!(
            "{{\"manual_mode\":{},\"paused\":{}}}",
            manual_mode,
            paused
        )
    }

    fn snapshot_json(&self) -> String {
        if let Some(ref coord) = self.coordinator {
            let snap = coord.snapshot();
            snap.to_json(self.running, &self.scenario_name)
        } else if let Some(ref snap) = self.last_snapshot {
            snap.to_json(false, &self.scenario_name)
        } else {
            empty_state_json()
        }
    }

    fn events_json(&self, since: usize) -> String {
        if let Some(ref el) = self.event_log {
            el.events_json_since(since)
        } else {
            "{\"events\":[],\"total_count\":0}".to_string()
        }
    }
}

fn empty_state_json() -> String {
    "{\"running\":false,\"scenario_name\":\"\",\"queue\":{\"urgent_count\":0,\"normal_count\":0,\"total_count\":0,\"total_pushed\":0,\"tasks\":[]},\"zones\":[],\"robots\":[],\"metrics\":{\"completed_task_count\":0,\"total_wait_count\":0,\"offline_count\":0,\"runtime_ms\":null}}".to_string()
}

fn scenarios_json() -> String {
    let scenarios = demo::available_scenarios();
    let mut out = String::with_capacity(512);
    out.push_str("{\"scenarios\":[");
    for (i, s) in scenarios.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"id\":\"");
        out.push_str(s.id);
        out.push_str("\",\"name\":\"");
        out.push_str(s.name);
        out.push_str("\",\"description\":\"");
        out.push_str(s.description);
        out.push_str("\"}");
    }
    out.push_str("]}");
    out
}

/// Run the server on an already-bound listener (blocking).
/// Used for tests; prefer `start_server` for normal use.
pub fn run_server(listener: TcpListener) {
    let mut state = AppState::new();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream, &mut state),
            Err(e) => eprintln!("connection error: {e}"),
        }
    }
}

/// Start the dashboard server on the given port (blocking).
pub fn start_server(port: u16) {
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).expect("failed to bind server address");
    println!("==============================================");
    println!("  Project Blaze — Web Dashboard");
    println!("  http://{addr}");
    println!("  Press Ctrl+C to stop");
    println!("==============================================");
    run_server(listener);
}

fn handle_connection(mut stream: TcpStream, state: &mut AppState) {
    let mut buf = [0u8; 4096];
    let n = match stream.read(&mut buf) {
        Ok(0) => return,
        Ok(n) => n,
        Err(_) => return,
    };

    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = match request.lines().next() {
        Some(l) => l,
        None => return,
    };

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let full_path = parts[1];
    let (path, query) = match full_path.find('?') {
        Some(idx) => (&full_path[..idx], &full_path[idx + 1..]),
        None => (full_path, ""),
    };

    let (status, content_type, body) = match (method, path) {
        ("GET", "/") => (200, "text/html; charset=utf-8", INDEX_HTML.to_string()),
        ("GET", "/styles.css") => (200, "text/css; charset=utf-8", STYLES_CSS.to_string()),
        ("GET", "/app.js") => (200, "application/javascript; charset=utf-8", APP_JS.to_string()),
        ("GET", "/api/state") => (200, "application/json", state.snapshot_json()),
        ("GET", "/api/events") => {
            let since = parse_query_usize(query, "since").unwrap_or(0);
            (200, "application/json", state.events_json(since))
        }
        ("GET", "/api/scenarios") => (200, "application/json", scenarios_json()),
        ("POST", "/api/scenario/start") => {
            let id = parse_query_str(query, "id");
            let manual = parse_query_bool(query, "manual");
            state.start_scenario_by_id(&id, manual);
            (200, "application/json", "{\"ok\":true}".to_string())
        }
        ("GET", "/api/time/state") => (200, "application/json", state.time_state_json()),
        ("POST", "/api/time/pause") => {
            state.time_pause();
            (200, "application/json", "{\"ok\":true}".to_string())
        }
        ("POST", "/api/time/resume") => {
            state.time_resume();
            (200, "application/json", "{\"ok\":true}".to_string())
        }
        ("POST", "/api/time/step") => {
            state.time_step();
            (200, "application/json", "{\"ok\":true}".to_string())
        }
        _ => (404, "text/plain", "Not Found".to_string()),
    };

    let status_text = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Error",
    };

    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn parse_query_usize(query: &str, key: &str) -> Option<usize> {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return v.parse().ok();
            }
        }
    }
    None
}

fn parse_query_str<'a>(query: &'a str, key: &str) -> String {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return v.to_string();
            }
        }
    }
    String::new()
}

fn parse_query_bool(query: &str, key: &str) -> bool {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on");
            }
        }
    }
    false
}
