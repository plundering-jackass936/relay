//! Ollama local agent adapter — uses the Ollama REST API.

use super::Agent;
use crate::{AgentStatus, HandoffResult, OllamaConfig};
use anyhow::Result;

pub struct OllamaAgent {
    url: String,
    model: String,
}

impl OllamaAgent {
    pub fn new(config: &OllamaConfig) -> Self {
        Self {
            url: config.url.clone(),
            model: config.model.clone(),
        }
    }
}

impl Agent for OllamaAgent {
    fn name(&self) -> &str { "ollama" }

    fn check_available(&self) -> AgentStatus {
        // Ping Ollama's API
        let tag_url = format!("{}/api/tags", self.url);
        match ureq::get(&tag_url).call() {
            Ok(resp) => {
                let body: serde_json::Value = resp.into_json().unwrap_or_default();
                let models = body.get("models")
                    .and_then(|m| m.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);

                // Check if our target model is available
                let has_model = body.get("models")
                    .and_then(|m| m.as_array())
                    .map(|arr| arr.iter().any(|m| {
                        m.get("name").and_then(|n| n.as_str())
                            .map(|n| n.starts_with(&self.model))
                            .unwrap_or(false)
                    }))
                    .unwrap_or(false);

                if has_model {
                    AgentStatus {
                        name: "ollama".into(),
                        available: true,
                        reason: format!("Running at {}, {} models, '{}' available", self.url, models, self.model),
                        version: Some(self.model.clone()),
                    }
                } else {
                    AgentStatus {
                        name: "ollama".into(),
                        available: true,
                        reason: format!("Running but model '{}' not pulled. {} models available", self.model, models),
                        version: None,
                    }
                }
            }
            Err(_) => AgentStatus {
                name: "ollama".into(),
                available: false,
                reason: format!("Not reachable at {}", self.url),
                version: None,
            },
        }
    }

    fn execute(&self, handoff_prompt: &str, _project_dir: &str) -> Result<HandoffResult> {
        let url = format!("{}/api/generate", self.url);

        let body = serde_json::json!({
            "model": self.model,
            "prompt": handoff_prompt,
            "stream": false
        });

        let resp = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&body)?;

        let resp_json: serde_json::Value = resp.into_json()?;
        let text = resp_json
            .get("response")
            .and_then(|r| r.as_str())
            .unwrap_or("(no response)");

        println!("{text}");

        Ok(HandoffResult {
            agent: "ollama".into(),
            success: true,
            message: format!("Ollama ({}) responded", self.model),
            handoff_file: None,
        })
    }
}
