//! Plugin system — load custom agent adapters from ~/.relay/plugins/.
//! Each plugin is a directory with a plugin.toml and an executable script.
//!
//! Example plugin structure:
//!   ~/.relay/plugins/my-agent/
//!     plugin.toml     # metadata + config
//!     handoff.sh      # receives handoff text on stdin, runs agent
//!
//! Example plugin.toml:
//!   [plugin]
//!   name = "my-agent"
//!   description = "Custom agent for internal tools"
//!   version = "0.1.0"
//!   command = "./handoff.sh"    # relative to plugin dir
//!   check = "./check.sh"       # optional: check if agent is available

use crate::{AgentStatus, HandoffResult};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Deserialize)]
struct PluginToml {
    plugin: PluginMeta,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PluginMeta {
    name: String,
    description: Option<String>,
    version: Option<String>,
    command: String,
    check: Option<String>,
}

pub struct PluginAgent {
    name: String,
    description: String,
    version: Option<String>,
    command: PathBuf,
    check: Option<PathBuf>,
    plugin_dir: PathBuf,
}

impl PluginAgent {
    fn new(plugin_dir: &Path) -> Result<Self> {
        let toml_path = plugin_dir.join("plugin.toml");
        let content = std::fs::read_to_string(&toml_path)?;
        let config: PluginToml = toml::from_str(&content)?;
        let meta = config.plugin;

        Ok(Self {
            name: meta.name,
            description: meta.description.unwrap_or_default(),
            version: meta.version,
            command: plugin_dir.join(&meta.command),
            check: meta.check.map(|c| plugin_dir.join(c)),
            plugin_dir: plugin_dir.to_path_buf(),
        })
    }
}

impl crate::agents::Agent for PluginAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn check_available(&self) -> AgentStatus {
        // If a check script exists, run it
        if let Some(ref check_cmd) = self.check {
            if check_cmd.exists() {
                let output = std::process::Command::new(check_cmd)
                    .current_dir(&self.plugin_dir)
                    .output();

                return match output {
                    Ok(o) if o.status.success() => AgentStatus {
                        name: self.name.clone(),
                        available: true,
                        reason: format!("Plugin: {}", self.description),
                        version: self.version.clone(),
                    },
                    _ => AgentStatus {
                        name: self.name.clone(),
                        available: false,
                        reason: "Plugin check script failed".into(),
                        version: None,
                    },
                };
            }
        }

        // No check script — just verify command exists
        if self.command.exists() {
            AgentStatus {
                name: self.name.clone(),
                available: true,
                reason: format!("Plugin: {}", self.description),
                version: self.version.clone(),
            }
        } else {
            AgentStatus {
                name: self.name.clone(),
                available: false,
                reason: format!("Plugin command not found: {}", self.command.display()),
                version: None,
            }
        }
    }

    fn execute(&self, handoff_prompt: &str, project_dir: &str) -> Result<HandoffResult> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let mut child = Command::new(&self.command)
            .current_dir(project_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        // Send handoff text via stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(handoff_prompt.as_bytes())?;
        }

        let status = child.wait()?;

        Ok(HandoffResult {
            agent: self.name.clone(),
            success: status.success(),
            message: if status.success() {
                format!("Plugin '{}' completed", self.name)
            } else {
                format!("Plugin '{}' exited with code {:?}", self.name, status.code())
            },
            handoff_file: None,
        })
    }
}

/// Discover and load all plugins from ~/.relay/plugins/.
pub fn discover_plugins() -> Vec<Box<dyn crate::agents::Agent>> {
    let plugins_dir = crate::data_dir().join("plugins");
    if !plugins_dir.exists() {
        return Vec::new();
    }

    let mut agents: Vec<Box<dyn crate::agents::Agent>> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("plugin.toml").exists() {
                match PluginAgent::new(&path) {
                    Ok(agent) => {
                        tracing::info!("Loaded plugin: {}", agent.name);
                        agents.push(Box::new(agent));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load plugin {:?}: {e}", path.file_name());
                    }
                }
            }
        }
    }

    agents
}

/// Create a scaffold for a new plugin.
pub fn scaffold_plugin(name: &str) -> Result<PathBuf> {
    let plugins_dir = crate::data_dir().join("plugins");
    let plugin_dir = plugins_dir.join(name);
    std::fs::create_dir_all(&plugin_dir)?;

    let toml_content = format!(
        r#"[plugin]
name = "{name}"
description = "Custom agent plugin"
version = "0.1.0"
command = "./handoff.sh"
# check = "./check.sh"  # optional: script to check if agent is available
"#
    );
    std::fs::write(plugin_dir.join("plugin.toml"), toml_content)?;

    let script = r#"#!/bin/bash
# Relay plugin: receives handoff context on stdin
# Replace this with your agent logic

HANDOFF=$(cat)
echo "Received handoff ($(echo "$HANDOFF" | wc -c) bytes)"
echo "TODO: implement your agent here"
"#;
    std::fs::write(plugin_dir.join("handoff.sh"), script)?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            plugin_dir.join("handoff.sh"),
            std::fs::Permissions::from_mode(0o755),
        )?;
    }

    Ok(plugin_dir)
}
