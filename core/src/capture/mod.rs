pub mod git;
pub mod session;
pub mod todos;

use crate::SessionSnapshot;
use anyhow::Result;
use std::path::Path;

/// Capture a full session snapshot from the current working directory.
pub fn capture_snapshot(
    project_dir: &Path,
    deadline: Option<&str>,
) -> Result<SessionSnapshot> {
    let git_state = git::capture_git_state(project_dir).ok();

    // Try to read Claude session state
    let session_info = session::read_latest_session(project_dir);

    // Try to read todo state
    let todos = todos::read_todos(project_dir);

    let recent_files = git_state
        .as_ref()
        .map(|g| g.uncommitted_files.clone())
        .unwrap_or_default();

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    Ok(SessionSnapshot {
        current_task: session_info.current_task,
        todos,
        decisions: session_info.decisions,
        last_error: session_info.last_error,
        last_output: session_info.last_output,
        git_state,
        project_dir: project_dir.to_string_lossy().to_string(),
        recent_files,
        timestamp,
        deadline: deadline.map(String::from),
    })
}
