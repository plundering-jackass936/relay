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

        // Write handoff to a temp file
        let tmp = std::env::temp_dir().join("relay_handoff.md");
        std::fs::write(&tmp, handoff_prompt)?;

        // Launch codex with the prompt
        let mut child = Command::new(&binary)
            .current_dir(project_dir)
            .arg("--model")
            .arg(&self.model)
            .arg("--quiet")
            .arg(handoff_prompt)
            .spawn()?;

        // Don't wait — let it run in the foreground
        let _ = child.wait();

        Ok(HandoffResult {
            agent: "codex".into(),
            success: true,
            message: format!("Codex ({}) launched with handoff context", self.model),
            handoff_file: Some(tmp.to_string_lossy().to_string()),
        })
    }
}
