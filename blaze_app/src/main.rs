//! CLI entry point for the Blaze demo.
//!
//! `cargo run -- -- web` ：launches the local web dashboard.
//! `cargo run` runs the default scenarios one by one and prints their event logs (and writes exports under `./output`).

use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--web") {
        let port = args
            .iter()
            .position(|a| a == "--port")
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(3000);
        blaze_app::server::start_server(port);
    } else {
        run_terminal_demo();
    }
}

fn run_terminal_demo() {
    let scenarios = blaze_sim::demo::default_scenarios();
    let mut reports = Vec::new();
    let output_root = Path::new("output");

    for scenario in scenarios {
        let report = blaze_sim::demo::run_scenario_with_export(scenario, 400, output_root)
            .expect("failed to run scenario and export report");
        reports.push(report);
    }

    println!("========================================");
    println!(" Demo Summary");
    println!("========================================");
    for report in &reports {
        println!(
            "Scenario: {} | completed={} waits={} offline={} runtime={:?}ms",
            report.scenario_name,
            report.metrics.total_completed_tasks,
            report.metrics.total_zone_wait_events,
            report.metrics.total_offline_detections,
            report.metrics.runtime_ms
        );
    }
    println!("Exports written under ./output");
}
