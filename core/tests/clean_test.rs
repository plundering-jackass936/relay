use std::fs;

#[test]
fn clean_empty_dir() {
    let tmp = std::env::temp_dir().join("relay_clean_test_empty");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let result = relay::clean::clean_handoffs(&tmp, 5, None, false).unwrap();
    assert!(result.removed.is_empty());
    assert!(result.kept.is_empty());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn clean_keeps_recent() {
    let tmp = std::env::temp_dir().join("relay_clean_test_keep");
    let _ = fs::remove_dir_all(&tmp);
    let relay_dir = tmp.join(".relay");
    fs::create_dir_all(&relay_dir).unwrap();

    // Create 5 handoff files
    for i in 1..=5 {
        fs::write(relay_dir.join(format!("handoff_20260{i}01_120000.md")), format!("handoff {i}")).unwrap();
        // Small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let result = relay::clean::clean_handoffs(&tmp, 3, None, false).unwrap();
    assert_eq!(result.kept.len(), 3);
    assert_eq!(result.removed.len(), 2);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn clean_dry_run_does_not_delete() {
    let tmp = std::env::temp_dir().join("relay_clean_test_dry");
    let _ = fs::remove_dir_all(&tmp);
    let relay_dir = tmp.join(".relay");
    fs::create_dir_all(&relay_dir).unwrap();

    for i in 1..=3 {
        fs::write(relay_dir.join(format!("handoff_2026010{i}_120000.md")), "test").unwrap();
    }

    let result = relay::clean::clean_handoffs(&tmp, 1, None, true).unwrap();
    assert_eq!(result.removed.len(), 2);

    // Files should still exist
    let remaining = fs::read_dir(&relay_dir).unwrap().count();
    assert_eq!(remaining, 3);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn parse_duration_works() {
    assert_eq!(relay::clean::parse_duration("7d"), Some(7 * 86400));
    assert_eq!(relay::clean::parse_duration("24h"), Some(24 * 3600));
    assert_eq!(relay::clean::parse_duration("30m"), Some(30 * 60));
    assert_eq!(relay::clean::parse_duration("invalid"), None);
}
