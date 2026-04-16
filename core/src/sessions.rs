//! Discover and list Claude Code sessions across all projects.
//! Scans ~/.claude/projects/ for .jsonl transcript files and extracts
//! session metadata (ID, project, timestamps, turn count, branch).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Metadata for a single Claude Code session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    /// UUID session identifier (filename stem)
    pub session_id: String,
    /// Decoded project path (from the directory name)
    pub project_path: String,
    /// First timestamp in the transcript
    pub started_at: String,
    /// Last timestamp in the transcript
    pub last_activity: String,
    /// Number of user/assistant turns (excluding progress/system)
    pub turns: usize,
    /// Git branch at session start (if available)
    pub branch: Option<String>,
    /// First user message (truncated) as task summary
    pub task_summary: String,
    /// Full path to the .jsonl file
    pub transcript_path: String,
}

/// Scan all Claude Code sessions and return metadata sorted by last_activity (newest first).
pub fn list_sessions() -> Result<Vec<SessionEntry>> {
    let claude_projects = claude_projects_dir()?;
    let mut entries = Vec::new();

    let projects = std::fs::read_dir(&claude_projects)?;
    for project_entry in projects.flatten() {
        if !project_entry.path().is_dir() {
            continue;
        }

        let project_name = project_entry.file_name().to_string_lossy().to_string();
        let project_path = decode_project_path(&project_name);

        let Ok(files) = std::fs::read_dir(project_entry.path()) else {
            continue;
        };

        for file_entry in files.flatten() {
            let path = file_entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) && path.is_file() {
                if let Some(entry) = parse_session_metadata(&path, &project_path) {
                    entries.push(entry);
                }
            }
        }
    }

    // Sort by last_activity descending (newest first)
    entries.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));

    Ok(entries)
}

/// Find a session by ID prefix (supports short IDs like first 8 chars).
pub fn find_session(session_id: &str) -> Result<Option<SessionEntry>> {
    let sessions = list_sessions()?;
    let matched: Vec<_> = sessions
        .into_iter()
        .filter(|s| s.session_id.starts_with(session_id))
        .collect();

    match matched.len() {
        0 => Ok(None),
        1 => Ok(Some(matched.into_iter().next().unwrap())),
        _ => anyhow::bail!(
            "Ambiguous session ID '{}' matches {} sessions. Use more characters.",
            session_id,
            matched.len()
        ),
    }
}

/// Decode a project directory path from the encoded directory name.
/// e.g. "-Users-manavaryasingh-myproject" -> "/Users/manavaryasingh/myproject"
fn decode_project_path(encoded: &str) -> String {
    if encoded.starts_with('-') {
        format!("/{}", encoded[1..].replace('-', "/"))
    } else {
        encoded.replace('-', "/")
    }
}

/// Parse minimal metadata from a .jsonl transcript without reading the whole file.
fn parse_session_metadata(path: &Path, fallback_project_path: &str) -> Option<SessionEntry> {
    let session_id = path.file_stem()?.to_string_lossy().to_string();

    // Skip non-UUID-looking filenames (e.g. "skill-injections")
    if !session_id.contains('-') || session_id.len() < 32 {
        return None;
    }

    let content = std::fs::read_to_string(path).ok()?;
    if content.is_empty() {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();

    let mut first_timestamp = String::new();
    let mut last_timestamp = String::new();
    let mut branch: Option<String> = None;
    let mut project_path: Option<String> = None;
    let mut turns = 0usize;
    let mut task_summary = String::new();

    for line in &lines {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };

        // Grab timestamp
        if let Some(ts) = val.get("timestamp").and_then(|v| v.as_str()) {
            if first_timestamp.is_empty() {
                first_timestamp = ts.to_string();
            }
            last_timestamp = ts.to_string();
        }

        // Grab cwd (actual project path) from the first entry that has one
        if project_path.is_none() {
            if let Some(cwd) = val.get("cwd").and_then(|v| v.as_str()) {
                if !cwd.is_empty() {
                    project_path = Some(cwd.to_string());
                }
            }
        }

        // Grab branch from first entry that has one
        if branch.is_none() {
            if let Some(b) = val.get("gitBranch").and_then(|v| v.as_str()) {
                if !b.is_empty() {
                    branch = Some(b.to_string());
                }
            }
        }

        // Count turns and grab first user message as task summary
        let msg_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match msg_type {
            "user" => {
                // Only count non-tool-result user messages
                if val.get("toolUseResult").is_none() {
                    turns += 1;
                    if task_summary.is_empty() {
                        let message = val.get("message").cloned().unwrap_or_default();
                        let text = extract_user_text(message.get("content"));
                        if text.len() > 5 && !text.starts_with('/') {
                            task_summary = truncate(&text, 80);
                        }
                    }
                }
            }
            "assistant" => {
                turns += 1;
            }
            _ => {}
        }
    }

    if first_timestamp.is_empty() {
        return None;
    }

    // Format timestamps for display (ISO -> local-friendly)
    let started_at = format_timestamp(&first_timestamp);
    let last_activity = format_timestamp(&last_timestamp);

    if task_summary.is_empty() {
        task_summary = "(no user message)".into();
    }

    Some(SessionEntry {
        session_id,
        project_path: project_path.unwrap_or_else(|| fallback_project_path.to_string()),
        started_at,
        last_activity,
        turns,
        branch,
        task_summary,
        transcript_path: path.to_string_lossy().to_string(),
    })
}

fn extract_user_text(content: Option<&serde_json::Value>) -> String {
    let Some(c) = content else { return String::new() };
    if let Some(s) = c.as_str() {
        return s.to_string();
    }
    if let Some(arr) = c.as_array() {
        for item in arr {
            if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                    return t.to_string();
                }
            }
        }
    }
    String::new()
}

fn format_timestamp(ts: &str) -> String {
    // Try to parse ISO 8601 and format as local time
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        let local: chrono::DateTime<chrono::Local> = dt.into();
        return local.format("%Y-%m-%d %H:%M").to_string();
    }
    // Fallback: return as-is but truncated
    ts.chars().take(16).collect()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

fn claude_projects_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    let dir = PathBuf::from(home).join(".claude/projects");
    if !dir.exists() {
        anyhow::bail!("Claude projects directory not found: {}", dir.display());
    }
    Ok(dir)
}
