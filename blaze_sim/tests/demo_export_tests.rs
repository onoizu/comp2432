use std::fs;

use blaze_core::task::Task;
use blaze_core::types::{TaskKind, TaskPriority, ZoneId};
use blaze_sim::demo::{run_scenario_with_export, DemoScenario};

#[test]
fn scenario_export_writes_json_file() {
    let temp_root = std::env::temp_dir().join("blaze_demo_export_test");
    let _ = fs::remove_dir_all(&temp_root);

    let scenario = DemoScenario {
        name: "Export Test",
        robot_count: 1,
        tasks: vec![Task::new(
            TaskPriority::Normal,
            TaskKind::Delivery,
            ZoneId::Lobby,
            20,
        )],
        runtime_ms: 250,
        fail_robot_id: None,
        fail_after_ms: None,
        heartbeat_timeout_ms: None,
        late_tasks: vec![],
        late_delay_ms: None,
    };

    let report =
        run_scenario_with_export(scenario, 120, &temp_root).expect("scenario should run and export");

    let expected_file = temp_root.join("export_test.json");
    assert!(expected_file.exists(), "export json file should exist");

    let content = fs::read_to_string(&expected_file).expect("should read export file");
    assert!(content.contains("\"event_timeline\""));
    assert!(content.contains("\"metrics\""));
    assert_eq!(report.scenario_name, "Export Test");

    /// Verify unified field names match web API schema.
    assert!(
        content.contains("\"completed_task_count\""),
        "export should use unified field name completed_task_count"
    );
    assert!(
        content.contains("\"total_wait_count\""),
        "export should use unified field name total_wait_count"
    );
    assert!(
        content.contains("\"offline_count\""),
        "export should use unified field name offline_count"
    );
    assert!(
        content.contains("\"urgent_count\""),
        "export snapshot queue should use urgent_count"
    );
    assert!(
        content.contains("\"waiting_robots\""),
        "export zones should use waiting_robots not waiting"
    );
    assert!(
        content.contains("\"robot_activity\""),
        "export should include robot_activity section"
    );

    let _ = fs::remove_dir_all(&temp_root);
}
