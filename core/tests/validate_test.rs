use relay::{Config, GeneralConfig, AgentsConfig};

#[test]
fn validate_returns_results_for_all_agents() {
    let config = Config {
        general: GeneralConfig::default(),
        agents: AgentsConfig::default(),
    };
    let results = relay::validate::validate_config(&config);
    // Should have config check + one result per agent in priority
    assert!(results.len() > config.general.priority.len());
    // First result is config validation
    assert_eq!(results[0].agent, "config");
}

#[test]
fn validate_detects_placeholder_keys() {
    // Only test openai which has no CLI fallback — purely API key based
    let mut config = Config {
        general: GeneralConfig {
            priority: vec!["openai".into()],
            ..Default::default()
        },
        agents: AgentsConfig::default(),
    };
    config.agents.openai.api_key = Some("your-key".into());
    // Clear env var for test isolation
    std::env::remove_var("OPENAI_API_KEY");

    let results = relay::validate::validate_config(&config);
    let openai = results.iter().find(|r| r.agent == "openai").unwrap();
    assert_eq!(openai.status, "warn");
    assert!(openai.message.contains("placeholder"));
}

#[test]
fn validate_reports_missing_keys() {
    let config = Config {
        general: GeneralConfig {
            priority: vec!["openai".into()],
            ..Default::default()
        },
        agents: AgentsConfig::default(),
    };
    std::env::remove_var("OPENAI_API_KEY");

    let results = relay::validate::validate_config(&config);
    let openai = results.iter().find(|r| r.agent == "openai").unwrap();
    assert_eq!(openai.status, "error");
}
