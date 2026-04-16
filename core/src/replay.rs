//! `relay replay` — Re-send a saved handoff to any agent for testing/comparison.

use anyhow::Result;
use std::path::Path;

/// Replay a handoff file against a specific agent.
pub fn replay_handoff(
    handoff_path: &Path,
    config: &crate::Config,
    agent_name: Option<&str>,
    dry_run: bool,
) -> Result<ReplayResult> {
    let handoff_text = std::fs::read_to_string(handoff_path)?;
    let project_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .to_string_lossy()
        .to_string();

    if dry_run {
        return Ok(ReplayResult {
            agent: agent_name.unwrap_or("dry-run").into(),
            success: true,
            message: format!("Would replay {} bytes to {}", handoff_text.len(), agent_name.unwrap_or("first available")),
            handoff_size: handoff_text.len(),
        });
    }

    let result = if let Some(name) = agent_name {
        crate::agents::handoff_to_named(config, name, &handoff_text, &project_dir, true)?
    } else {
        crate::agents::handoff_to_first_available(config, &handoff_text, &project_dir)?
    };

    Ok(ReplayResult {
        agent: result.agent,
        success: result.success,
        message: result.message,
        handoff_size: handoff_text.len(),
    })
}

/// Find a handoff file by index (0 = most recent) or path.
pub fn resolve_handoff_path(project_dir: &Path, specifier: &str) -> Result<std::path::PathBuf> {
    // If it's a path, use directly
    let as_path = std::path::PathBuf::from(specifier);
    if as_path.exists() {
        return Ok(as_path);
    }

    // Otherwise treat as index (0 = most recent)
    let index: usize = specifier.parse().unwrap_or(0);
    let entries = crate::history::list_handoffs(project_dir, index + 1)?;
    entries.get(index)
        .map(|e| project_dir.join(".relay").join(&e.filename))
        .ok_or_else(|| anyhow::anyhow!("No handoff found at index {index}"))
}

#[derive(Debug, serde::Serialize)]
pub struct ReplayResult {
    pub agent: String,
    pub success: bool,
    pub message: String,
    pub handoff_size: usize,
}
