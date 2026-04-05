//! Read TodoWrite state from Claude Code session.

use crate::TodoItem;
use std::path::Path;

/// Attempt to read the current todo list from the latest session transcript.
pub fn read_todos(project_dir: &Path) -> Vec<TodoItem> {
    // Try to read from the latest session's JSONL
    let _session = super::session::read_latest_session(project_dir);

    // Parse from the session info — todos are embedded in tool_use calls
    // For now, try to read from the last TodoWrite call in the transcript
    let transcript_todos = read_todos_from_transcript(project_dir);
    if !transcript_todos.is_empty() {
        return transcript_todos;
    }

    // Fallback: try to infer from git
    Vec::new()
}

fn read_todos_from_transcript(project_dir: &Path) -> Vec<TodoItem> {
    let home = std::env::var("HOME").unwrap_or_default();
    let claude_projects = std::path::PathBuf::from(&home).join(".claude/projects");
    if !claude_projects.exists() {
        return Vec::new();
    }

    let proj_str = project_dir.to_string_lossy().replace('/', "-");
    let dir = claude_projects.join(&proj_str);
    if !dir.exists() {
        // Try suffix match
        let entries = std::fs::read_dir(&claude_projects).ok();
        let dir_name = project_dir.file_name().map(|n| n.to_string_lossy().to_string());
        let matched_dir = entries.and_then(|es| {
            let name = dir_name?;
            for e in es.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.ends_with(&name) && e.path().is_dir() {
                    return Some(e.path());
                }
            }
            None
        });
        if matched_dir.is_none() {
            return Vec::new();
        }
        return parse_todos_from_dir(&matched_dir.unwrap());
    }

    parse_todos_from_dir(&dir)
}

fn parse_todos_from_dir(dir: &Path) -> Vec<TodoItem> {
    // Find latest JSONL
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

    let Some((path, _)) = newest else { return Vec::new() };
    let content = std::fs::read_to_string(&path).unwrap_or_default();

    // Scan for the last TodoWrite tool_use call
    let mut last_todos: Vec<TodoItem> = Vec::new();
    for line in content.lines().rev() {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else { continue };

        // Look for tool_use with name "TodoWrite"
        if let Some(content) = val.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array()) {
            for item in content {
                if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    if item.get("name").and_then(|n| n.as_str()) == Some("TodoWrite") {
                        if let Some(input) = item.get("input") {
                            if let Some(todos) = input.get("todos").and_then(|t| t.as_array()) {
                                last_todos = todos.iter().filter_map(|t| {
                                    let content = t.get("content")?.as_str()?.to_string();
                                    let status = t.get("status")?.as_str()?.to_string();
                                    Some(TodoItem { content, status })
                                }).collect();
                                if !last_todos.is_empty() {
                                    return last_todos;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    last_todos
}
