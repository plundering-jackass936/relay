//! Codex CLI agent adapter.
//! Launches `codex` (OpenAI Codex CLI) as a subprocess with the handoff prompt.

use super::Agent;
use crate::{AgentStatus, CodexConfig, HandoffResult};
use anyhow::Result;
use std::process::Command;

pub struct CodexAgent {
    binary: String,
    model: String,
}

impl CodexAgent {
    pub fn new(config: &CodexConfig) -> Self {
        Self {
            binary: config.binary.clone().unwrap_or_else(|| "codex".into()),
            model: config.model.clone(),
        }
    }

    fn find_binary(&self) -> Option<String> {
        // Check if binary exists in PATH
        let output = Command::new("which")
            .arg(&self.binary)
            .output()
            .ok()?;
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        None
    }
}

impl Agent for CodexAgent {
    fn name(&self) -> &str { "codex" }

    fn check_available(&self) -> AgentStatus {
        match self.find_binary() {
            Some(path) => {
                // Try to get version
                let version = Command::new(&path)
                    .arg("--version")
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

                AgentStatus {
                    name: "codex".into(),
                    available: true,
                    reason: format!("Found at {path}"),
                    version,
                }
            }
            None => AgentStatus {
                name: "codex".into(),
                available: false,
                reason: format!("'{}' not found in PATH", self.binary),
                version: None,
            },
        }
    }

    fn execute(&self, handoff_prompt: &str, project_dir: &str) -> Result<HandoffResult> {
        let binary = self.find_binary().unwrap_or(self.binary.clone());

        // Write handoff to a temp file for reference
        let tmp = std::env::temp_dir().join("relay_handoff.md");
        std::fs::write(&tmp, handoff_prompt)?;

        // Use `codex exec --full-auto` for non-interactive auto-approval
        let output = Command::new(&binary)
            .current_dir(project_dir)
            .arg("exec")
            .arg("--full-auto")
            .arg("-m")
            .arg(&self.model)
            .arg(handoff_prompt)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.is_empty() {
            println!("{stdout}");
        }
        if !stderr.is_empty() {
            eprintln!("{stderr}");
        }

        Ok(HandoffResult {
            agent: "codex".into(),
            success: output.status.success(),
            message: if output.status.success() {
                format!("Codex ({}) completed handoff task", self.model)
            } else {
                format!("Codex exited with code {:?}", output.status.code())
            },
            handoff_file: Some(tmp.to_string_lossy().to_string()),
        })
    }
}
