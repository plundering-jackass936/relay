use relay::{Config, AgentStatus};
use relay::agents;

#[test]
fn get_agents_returns_all_configured() {
    let config = Config {
        general: relay::GeneralConfig {
            priority: vec!["codex".into(), "ollama".into(), "openai".into()],
            ..Default::default()
        },
        agents: Default::default(),
    };
    let agent_list = agents::get_agents(&config);
    assert_eq!(agent_list.len(), 3);
    assert_eq!(agent_list[0].name(), "codex");
    assert_eq!(agent_list[1].name(), "ollama");
    assert_eq!(agent_list[2].name(), "openai");
}

#[test]
fn unknown_agent_skipped() {
    let config = Config {
        general: relay::GeneralConfig {
            priority: vec!["nonexistent_agent".into(), "codex".into()],
            ..Default::default()
        },
        agents: Default::default(),
    };
    let agent_list = agents::get_agents(&config);
    assert_eq!(agent_list.len(), 1);
    assert_eq!(agent_list[0].name(), "codex");
}

#[test]
fn check_all_agents_returns_statuses() {
    let config = Config {
        general: relay::GeneralConfig {
            priority: vec!["openai".into()],
            ..Default::default()
        },
        agents: Default::default(),
    };
    let statuses = agents::check_all_agents(&config);
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].name, "openai");
    // No API key set so should be unavailable
    assert!(!statuses[0].available);
}

#[test]
fn handoff_to_unavailable_fails_gracefully() {
    let config = Config {
        general: relay::GeneralConfig {
            priority: vec!["openai".into()],
            ..Default::default()
        },
        agents: Default::default(),
    };
    let result = agents::handoff_to_named(&config, "openai", "test", "/tmp", false).unwrap();
    assert!(!result.success);
    assert!(result.message.contains("not available"));
}

#[test]
fn handoff_to_unknown_agent_fails() {
    let config = Config {
        general: Default::default(),
        agents: Default::default(),
    };
    let result = agents::handoff_to_named(&config, "doesnotexist", "test", "/tmp", false).unwrap();
    assert!(!result.success);
    assert!(result.message.contains("Unknown agent"));
}
