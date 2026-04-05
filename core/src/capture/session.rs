//! Read Claude Code session state from .jsonl transcript files.

use std::path::Path;

/// Extracted session info from Claude's transcript.
pub struct SessionInfo {
    pub current_task: String,
    pub decisions: Vec<String>,
    pub last_error: Option<String>,
    pub last_output: Option<String>,
}

/// Read the latest Claude session transcript and extract state.
pub fn read_latest_session(project_dir: &Path) -> SessionInfo {
    // Claude stores transcripts in ~/.claude/projects/<project_hash>/<session>.jsonl
    // Try to find and parse the latest one

    let claude_dir = find_claude_project_dir(project_dir);

    if let Some(dir) = claude_dir {
        if let Some(latest) = find_latest_jsonl(&dir) {
            return parse_session_transcript(&latest);
        }
    }

    // Fallback: infer from git state
    SessionInfo {
        current_task: infer_task_from_git(project_dir),
        decisions: Vec::new(),
        last_error: None,
        last_output: None,
    }
}

fn find_claude_project_dir(project_dir: &Path) -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let claude_projects = std::path::PathBuf::from(&home).join(".claude/projects");
    if !claude_projects.exists() {
        return None;
    }

    // Claude encodes the project path: /Users/user/myproject -> -Users-user-myproject
    let proj_str = project_dir.to_string_lossy().replace('/', "-");
    let candidate = claude_projects.join(&proj_str);
    if candidate.exists() {
        return Some(candidate);
    }

    // Try matching by suffix
    if let Ok(entries) = std::fs::read_dir(&claude_projects) {
        let dir_name = project_dir.file_name()?.to_string_lossy();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(&*dir_name) && entry.path().is_dir() {
                return Some(entry.path());
            }
        }
    }

    None
}

fn find_latest_jsonl(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                if let Ok(meta) = path.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if newest.as_ref().map_or(true, |(_, t)| modified > *t) {
                            newest = Some((path, modified));
                        }
                    }
                }
            }
        }
    }
    newest.map(|(p, _)| p)
}

fn parse_session_transcript(path: &std::path::Path) -> SessionInfo {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return default_session_info(),
    };

    let mut current_task = String::new();
    let mut decisions = Vec::new();
    let mut last_error = None;
    let mut last_output = None;

    // Read last 200 lines of the JSONL for efficiency
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(200);

    for line in &lines[start..] {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else { continue };

        // Extract user messages for task context
        if val.get("type").and_then(|v| v.as_str()) == Some("human") {
            if let Some(msg) = val.get("message").and_then(|m| {
                m.get("content").and_then(|c| {
                    if let Some(s) = c.as_str() {
                        Some(s.to_string())
                    } else if let Some(arr) = c.as_array() {
                        arr.iter()
                            .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                            .last()
                            .map(String::from)
                    } else {
                        None
                    }
                })
            }) {
                // Keep the last substantive user message as current task
                if msg.len() > 10 && !msg.starts_with('/') {
                    current_task = if msg.len() > 500 {
                        format!("{}...", &msg[..500])
                    } else {
                        msg
                    };
                }
            }
        }

        // Extract assistant tool use for last output
        if val.get("type").and_then(|v| v.as_str()) == Some("assistant") {
            if let Some(content) = val.get("message").and_then(|m| m.get("content")) {
                if let Some(arr) = content.as_array() {
                    for item in arr {
                        // Look for tool results
                        if item.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                            if let Some(output) = item.get("content").and_then(|c| c.as_str()) {
                                // Check for errors
                                let lower = output.to_lowercase();
                                if lower.contains("error") || lower.contains("failed") || lower.contains("panic") {
                                    last_error = Some(truncate(output, 500));
                                }
                                last_output = Some(truncate(output, 500));
                            }
                        }
                        // Look for text content with decisions
                        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                // Extract lines that look like decisions
                                for line in text.lines() {
                                    let trimmed = line.trim();
                                    if (trimmed.starts_with("I'll use") ||
                                        trimmed.starts_with("I chose") ||
                                        trimmed.starts_with("Decision:") ||
                                        trimmed.starts_with("Using ") ||
                                        trimmed.starts_with("Approach:"))
                                        && trimmed.len() > 15
                                    {
                                        decisions.push(truncate(trimmed, 200));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if current_task.is_empty() {
        current_task = "Could not determine current task from session transcript".into();
    }

    // Deduplicate and limit decisions
    decisions.dedup();
    decisions.truncate(10);

    SessionInfo {
        current_task,
        decisions,
        last_error,
        last_output,
    }
}

fn infer_task_from_git(project_dir: &Path) -> String {
    // Infer from recent commit messages
    let output = std::process::Command::new("git")
        .current_dir(project_dir)
        .args(["log", "--oneline", "-1", "--no-decorate"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !msg.is_empty() {
                return format!("Recent work: {msg}");
            }
        }
    }

    "Unknown — no session transcript or git history found".into()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

fn default_session_info() -> SessionInfo {
    SessionInfo {
        current_task: "Could not read session transcript".into(),
        decisions: Vec::new(),
        last_error: None,
        last_output: None,
    }
}
