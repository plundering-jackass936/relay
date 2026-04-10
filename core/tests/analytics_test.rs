use std::fs;

#[test]
fn analytics_record_and_query() {
    let tmp = std::env::temp_dir().join("relay_analytics_test.db");
    let _ = fs::remove_file(&tmp);

    let conn = relay::analytics::open_db_at(&tmp).unwrap();

    // Record some handoffs
    relay::analytics::record_handoff(&conn, "codex", true, 150, 5000, 1400, "full", "/tmp/proj", "Fix auth", None, 0).unwrap();
    relay::analytics::record_handoff(&conn, "gemini", true, 2300, 8000, 2200, "full", "/tmp/proj", "Add tests", None, 0).unwrap();
    relay::analytics::record_handoff(&conn, "openai", false, 500, 3000, 850, "minimal", "/tmp/proj", "Refactor", Some("API error"), 1).unwrap();

    let stats = relay::analytics::get_stats(&conn).unwrap();

    assert_eq!(stats.total_handoffs, 3);
    assert_eq!(stats.successful, 2);
    assert_eq!(stats.failed, 1);
    assert!(stats.success_rate > 60.0);
    assert_eq!(stats.agents.len(), 3);
    assert_eq!(stats.recent.len(), 3);

    let _ = fs::remove_file(&tmp);
}

#[test]
fn analytics_empty_db() {
    let tmp = std::env::temp_dir().join("relay_analytics_empty.db");
    let _ = fs::remove_file(&tmp);

    let conn = relay::analytics::open_db_at(&tmp).unwrap();
    let stats = relay::analytics::get_stats(&conn).unwrap();

    assert_eq!(stats.total_handoffs, 0);
    assert_eq!(stats.success_rate, 0.0);

    let _ = fs::remove_file(&tmp);
}

#[test]
fn analytics_agent_stats_aggregate() {
    let tmp = std::env::temp_dir().join("relay_analytics_agg.db");
    let _ = fs::remove_file(&tmp);

    let conn = relay::analytics::open_db_at(&tmp).unwrap();

    // Same agent, multiple handoffs
    relay::analytics::record_handoff(&conn, "codex", true, 100, 1000, 300, "full", "/tmp", "task1", None, 0).unwrap();
    relay::analytics::record_handoff(&conn, "codex", true, 200, 2000, 600, "full", "/tmp", "task2", None, 0).unwrap();
    relay::analytics::record_handoff(&conn, "codex", false, 50, 500, 150, "full", "/tmp", "task3", Some("err"), 0).unwrap();

    let stats = relay::analytics::get_stats(&conn).unwrap();
    let codex = stats.agents.iter().find(|a| a.agent == "codex").unwrap();

    assert_eq!(codex.total, 3);
    assert_eq!(codex.successful, 2);
    assert_eq!(codex.failed, 1);

    let _ = fs::remove_file(&tmp);
}
