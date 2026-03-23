//! Scenario-driven demo runner for Project Blaze.
//!
//! This module keeps the demo flow structured and repeatable while using the
//! real `blaze_core` state as the single source of truth.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use std::{fs, io, path::Path, path::PathBuf};

use blaze_core::coordinator::Coordinator;
use blaze_core::event_log::TimelineRow;
use blaze_core::metrics::MetricsSnapshot;
use blaze_core::summary::SystemSnapshot;
use blaze_core::task::Task;
use blaze_core::types::{DEFAULT_HEARTBEAT_TIMEOUT_MS, RobotId};

use crate::scenarios;

/// Declarative scenario description for the demo runner.
pub struct DemoScenario {
    pub name: &'static str,
    pub robot_count: usize,
    pub tasks: Vec<Task>,
    pub runtime_ms: u64,
    pub fail_robot_id: Option<RobotId>,
    pub fail_after_ms: Option<u64>,
    /// Per-scenario heartbeat timeout override (ms).
    /// If `None`, uses `DEFAULT_HEARTBEAT_TIMEOUT_MS`.
    pub heartbeat_timeout_ms: Option<u64>,
    /// Tasks submitted later (started with the failing robot after `late_delay_ms`).
    /// Used to ensure the failing robot's task is submitted only after the
    /// target zone is already occupied.
    pub late_tasks: Vec<Task>,
    /// If `Some(ms)`, non-failing robots start first and submit `tasks`.
    /// After waiting `ms`, the failing robot starts and `late_tasks` are submitted.
    /// `fail_after_ms` is counted from this delayed start point.
    pub late_delay_ms: Option<u64>,
}

/// Final scenario result used by app-layer reporting/export.
pub struct ScenarioReport {
    pub scenario_name: String,
    pub final_snapshot: SystemSnapshot,
    pub timeline: Vec<TimelineRow>,
    pub metrics: MetricsSnapshot,
}

/// Metadata about an available scenario (for UI dropdowns).
pub struct ScenarioInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

/// Return the list of available scenarios with display metadata.
pub fn available_scenarios() -> Vec<ScenarioInfo> {
    vec![
        ScenarioInfo {
            id: "basic_parallel",
            name: "Basic Parallel Scheduling",
            description: "Tasks spread across different zones — robots execute concurrently",
        },
        ScenarioInfo {
            id: "zone_conflict",
            name: "Zone Conflict",
            description: "Multiple tasks target WardA — shows mutex waiting queues",
        },
        ScenarioInfo {
            id: "timeout",
            name: "Timeout & Reclaim",
            description: "One robot goes offline; its task is reclaimed and re-executed",
        },
        ScenarioInfo {
            id: "cooperative_preemption",
            name: "Cooperative Preemption",
            description: "Normal task yields when Urgent arrives — cooperative priority scheduling",
        },
    ]
}

/// Build a `DemoScenario` by its stable id.
pub fn build_scenario(id: &str) -> Option<DemoScenario> {
    match id {
        "basic_parallel" => Some(DemoScenario {
            name: "Basic Parallel Scheduling",
            robot_count: 3,
            tasks: scenarios::basic_delivery(),
            runtime_ms: 400,
            fail_robot_id: None,
            fail_after_ms: None,
            heartbeat_timeout_ms: None,
            late_tasks: vec![],
            late_delay_ms: None,
        }),
        "zone_conflict" => Some(DemoScenario {
            name: "Zone Conflict",
            robot_count: 3,
            tasks: scenarios::zone_conflict(),
            runtime_ms: 500,
            fail_robot_id: None,
            fail_after_ms: None,
            heartbeat_timeout_ms: None,
            late_tasks: vec![],
            late_delay_ms: None,
        }),
        "timeout" => {
            let (initial_tasks, late_task, fail_robot_id) = scenarios::timeout_demo();
            Some(DemoScenario {
                name: "Timeout & Reclaim",
                robot_count: 3,
                tasks: initial_tasks,
                runtime_ms: 12000,
                fail_robot_id: Some(fail_robot_id),
                fail_after_ms: Some(500),
                heartbeat_timeout_ms: Some(1500),
                late_tasks: vec![late_task],
                late_delay_ms: Some(500),
            })
        }
        "cooperative_preemption" => {
            let (initial_tasks, late_urgent) = scenarios::cooperative_preemption_demo();
            Some(DemoScenario {
                name: "Cooperative Preemption",
                robot_count: 1,
                tasks: initial_tasks,
                runtime_ms: 10000,
                fail_robot_id: None,
                fail_after_ms: None,
                heartbeat_timeout_ms: None,
                late_tasks: vec![late_urgent],
                late_delay_ms: Some(2000),
            })
        }
        _ => None,
    }
}

/// Build default narrative scenarios in the expected order.
pub fn default_scenarios() -> Vec<DemoScenario> {
    let (initial_tasks, late_task, fail_robot_id) = scenarios::timeout_demo();
    vec![
        DemoScenario {
            name: "Basic Parallel Scheduling",
            robot_count: 3,
            tasks: scenarios::basic_delivery(),
            runtime_ms: 400,
            fail_robot_id: None,
            fail_after_ms: None,
            heartbeat_timeout_ms: None,
            late_tasks: vec![],
            late_delay_ms: None,
        },
        DemoScenario {
            name: "Zone Conflict",
            robot_count: 3,
            tasks: scenarios::zone_conflict(),
            runtime_ms: 500,
            fail_robot_id: None,
            fail_after_ms: None,
            heartbeat_timeout_ms: None,
            late_tasks: vec![],
            late_delay_ms: None,
        },
        DemoScenario {
            name: "Timeout & Reclaim",
            robot_count: 3,
            tasks: initial_tasks,
            runtime_ms: 12000,
            fail_robot_id: Some(fail_robot_id),
            fail_after_ms: Some(500),
            heartbeat_timeout_ms: Some(1500),
            late_tasks: vec![late_task],
            late_delay_ms: Some(500),
        },
    ]
}

/// Run one scenario and print periodic dashboard snapshots.
pub fn run_scenario(scenario: DemoScenario, snapshot_interval_ms: u64) -> ScenarioReport {
    print_header(scenario.name);

    let timeout_ms = scenario.heartbeat_timeout_ms.unwrap_or(DEFAULT_HEARTBEAT_TIMEOUT_MS);
    let timeout = Duration::from_millis(timeout_ms);
    let mut coord = Coordinator::new(timeout);
    let metrics = coord.metrics();
    let event_log = coord.event_log();

    metrics.start_scenario(scenario.name);
    coord.start_monitor();

    let fail_flag = Arc::new(AtomicBool::new(false));

    /// Phase 1: start non-failing robots first, then submit initial tasks.
    for id in 0..scenario.robot_count {
        if scenario.fail_robot_id == Some(id) {
            continue;
        }
        coord.spawn_robot(id, None);
    }
    for task in scenario.tasks {
        coord.submit_task(task);
    }

    /// Phase 2: after delay, start failing robot and submit late tasks.
    if let Some(delay) = scenario.late_delay_ms {
        thread::sleep(Duration::from_millis(delay));
    }
    if let Some(fid) = scenario.fail_robot_id {
        coord.spawn_robot(fid, Some(Arc::clone(&fail_flag)));
    }
    for task in scenario.late_tasks {
        coord.submit_task(task);
    }

    if let Some(after_ms) = scenario.fail_after_ms {
        let ff = Arc::clone(&fail_flag);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(after_ms));
            ff.store(true, Ordering::Relaxed);
        });
    }

    let start = Instant::now();
    while start.elapsed().as_millis() < u128::from(scenario.runtime_ms) {
        print_dashboard(&coord, &event_log, 6);
        thread::sleep(Duration::from_millis(snapshot_interval_ms));
    }

    metrics.end_scenario();

    let final_snapshot = coord.snapshot();
    let timeline = event_log.timeline();
    let metrics_snapshot = metrics.snapshot();

    coord.shutdown();
    print_final_summary(&final_snapshot);

    ScenarioReport {
        scenario_name: scenario.name.to_string(),
        final_snapshot,
        timeline,
        metrics: metrics_snapshot,
    }
}

/// Run one scenario, then export report JSON into `output_root`.
pub fn run_scenario_with_export(
    scenario: DemoScenario,
    snapshot_interval_ms: u64,
    output_root: &Path,
) -> io::Result<ScenarioReport> {
    let report = run_scenario(scenario, snapshot_interval_ms);
    let _ = export_report_json(&report, output_root)?;
    Ok(report)
}

fn print_header(name: &str) {
    println!("========================================");
    println!(" Scenario: {name}");
    println!("========================================");
}

fn print_dashboard(coord: &Coordinator, log: &blaze_core::event_log::EventLog, latest_count: usize) {
    let snapshot = coord.snapshot();
    println!();
    println!("----- Dashboard Snapshot -----");
    println!(
        "Queue: urgent={}, normal={}, total={}",
        snapshot.queue.urgent_count, snapshot.queue.normal_count, snapshot.queue.total_count
    );

    println!("Zones:");
    for zone in &snapshot.zones {
        let occupant = zone
            .occupant
            .map(|id| format!("Robot {id}"))
            .unwrap_or_else(|| "Free".to_string());
        let waiting = if zone.waiting_robots.is_empty() {
            "-".to_string()
        } else {
            zone.waiting_robots
                .iter()
                .map(|id| format!("Robot {id}"))
                .collect::<Vec<_>>()
                .join(" -> ")
        };
        println!("  {} | occupant: {} | waiting: {}", zone.zone, occupant, waiting);
    }

    println!("Robots:");
    for robot in &snapshot.robots {
        println!(
            "  Robot {} | state: {:?} | task: {:?} | zone: {:?}",
            robot.robot_id, robot.state, robot.current_task_id, robot.current_zone
        );
    }

    println!(
        "Metrics: completed={}, waits={}, offline={}",
        snapshot.metrics.completed_task_count,
        snapshot.metrics.total_wait_count,
        snapshot.metrics.offline_count
    );

    println!("Latest events:");
    let timeline = log.timeline();
    let start = timeline.len().saturating_sub(latest_count);
    for row in &timeline[start..] {
        println!(
            "  [{:04}ms][{}] {}",
            row.elapsed_ms,
            row.category.tag(),
            row.message
        );
    }
}

fn print_final_summary(snapshot: &SystemSnapshot) {
    println!();
    println!("----- Final Summary -----");
    println!(
        "Completed tasks: {} | Wait events: {} | Offline detections: {} | Runtime: {:?} ms",
        snapshot.metrics.completed_task_count,
        snapshot.metrics.total_wait_count,
        snapshot.metrics.offline_count,
        snapshot.metrics.runtime_ms
    );
    println!();
}

/// Export one scenario report into JSON file and return path.
pub fn export_report_json(report: &ScenarioReport, output_root: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(output_root)?;
    let file_name = format!("{}.json", sanitize_file_name(&report.scenario_name));
    let out_path = output_root.join(file_name);
    fs::write(&out_path, report_to_json(report))?;
    Ok(out_path)
}

fn report_to_json(report: &ScenarioReport) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"scenario_name\": \"{}\",\n",
        json_escape(&report.scenario_name)
    ));

    out.push_str("  \"metrics\": {\n");
    out.push_str(&format!(
        "    \"completed_task_count\": {},\n",
        report.metrics.total_completed_tasks
    ));
    out.push_str(&format!(
        "    \"total_wait_count\": {},\n",
        report.metrics.total_zone_wait_events
    ));
    out.push_str(&format!(
        "    \"offline_count\": {},\n",
        report.metrics.total_offline_detections
    ));
    out.push_str(&format!(
        "    \"runtime_ms\": {}\n",
        option_u128_json(report.metrics.runtime_ms)
    ));
    out.push_str("  },\n");

    out.push_str("  \"robot_activity\": {\n");
    out.push_str("    \"per_robot_completed_tasks\": {");
    let mut first = true;
    let mut robot_rows: Vec<_> = report.metrics.per_robot_completed_tasks.iter().collect();
    robot_rows.sort_by_key(|(id, _)| *id);
    for (robot_id, count) in robot_rows {
        if !first {
            out.push_str(", ");
        }
        first = false;
        out.push_str(&format!("\"{robot_id}\": {count}"));
    }
    out.push_str("}\n");
    out.push_str("  },\n");

    out.push_str("  \"zone_wait_counts\": {");
    let mut first_zone = true;
    let mut zone_rows: Vec<_> = report.metrics.per_zone_wait_counts.iter().collect();
    zone_rows.sort_by_key(|(zone, _)| zone.to_string());
    for (zone, count) in zone_rows {
        if !first_zone {
            out.push_str(", ");
        }
        first_zone = false;
        out.push_str(&format!("\"{zone}\": {count}"));
    }
    out.push_str("},\n");

    out.push_str("  \"final_snapshot\": {\n");
    out.push_str(&format!(
        "    \"queue\": {{ \"urgent_count\": {}, \"normal_count\": {}, \"total_count\": {}, \"total_pushed\": {} }},\n",
        report.final_snapshot.queue.urgent_count,
        report.final_snapshot.queue.normal_count,
        report.final_snapshot.queue.total_count,
        report.final_snapshot.queue.total_pushed
    ));
    out.push_str("    \"zones\": [\n");
    for (idx, zone) in report.final_snapshot.zones.iter().enumerate() {
        let comma = if idx + 1 == report.final_snapshot.zones.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "      {{ \"zone\": \"{}\", \"occupant\": {}, \"waiting_robots\": [{}] }}{}\n",
            zone.zone,
            option_usize_json(zone.occupant),
            zone.waiting_robots
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            comma
        ));
    }
    out.push_str("    ],\n");

    out.push_str("    \"robots\": [\n");
    for (idx, robot) in report.final_snapshot.robots.iter().enumerate() {
        let comma = if idx + 1 == report.final_snapshot.robots.len() {
            ""
        } else {
            ","
        };
        let task = option_u64_json(robot.current_task_id);
        let zone = robot
            .current_zone
            .map(|z| format!("\"{z}\""))
            .unwrap_or_else(|| "null".to_string());
        out.push_str(&format!(
            "      {{ \"robot_id\": {}, \"state\": \"{}\", \"status\": \"{}\", \"current_task_id\": {}, \"current_zone\": {} }}{}\n",
            robot.robot_id,
            robot.state,
            robot.status,
            task,
            zone,
            comma
        ));
    }
    out.push_str("    ]\n");

    out.push_str("  },\n");

    out.push_str("  \"event_timeline\": [\n");
    for (idx, row) in report.timeline.iter().enumerate() {
        let comma = if idx + 1 == report.timeline.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "    {{ \"elapsed_ms\": {}, \"category\": \"{}\", \"code\": \"{}\", \"message\": \"{}\" }}{}\n",
            row.elapsed_ms,
            row.category.tag(),
            row.code,
            json_escape(&row.message),
            comma
        ));
    }
    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}

fn sanitize_file_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn json_escape(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn option_u128_json(v: Option<u128>) -> String {
    v.map(|x| x.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn option_u64_json(v: Option<u64>) -> String {
    v.map(|x| x.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn option_usize_json(v: Option<usize>) -> String {
    v.map(|x| x.to_string())
        .unwrap_or_else(|| "null".to_string())
}
