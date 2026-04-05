//! Handoff package builder — compresses session state into a prompt
//! that any agent can pick up and continue from.

use crate::SessionSnapshot;
use anyhow::Result;

/// Build a handoff prompt from a session snapshot.
/// This is the formatted text that gets sent to the fallback agent.
pub fn build_handoff(
    snapshot: &SessionSnapshot,
    target_agent: &str,
    max_tokens: usize,
) -> Result<String> {
    let mut sections: Vec<String> = Vec::new();

    // ── Header ─────────────────────────────────────────────────
    let urgency = if let Some(ref deadline) = snapshot.deadline {
        format!("\n  DEADLINE     : {deadline}")
    } else {
        String::new()
    };

    sections.push(format!(
        "══ RELAY HANDOFF ══════════════════════════════
  Original agent : Claude Code
  Handed off at  : {}
  Target agent   : {}
  Project        : {}{}
══════════════════════════════════════════════",
        snapshot.timestamp,
        target_agent,
        snapshot.project_dir,
        urgency
    ));

    // ── Current Task ───────────────────────────────────────────
    sections.push(format!(
        "## CURRENT TASK\n\n{}",
        snapshot.current_task
    ));

    // ── Todos ──────────────────────────────────────────────────
    if !snapshot.todos.is_empty() {
        let mut todo_text = String::from("## PROGRESS\n\n");
        for t in &snapshot.todos {
            let icon = match t.status.as_str() {
                "completed"   => "done",
                "in_progress" => "IN PROGRESS",
                _             => "pending",
            };
            todo_text.push_str(&format!("  [{}] {}\n", icon, t.content));
        }
        sections.push(todo_text);
    }

    // ── Last Error ─────────────────────────────────────────────
    if let Some(ref err) = snapshot.last_error {
        sections.push(format!(
            "## LAST ERROR\n\n```\n{}\n```",
            truncate_smart(err, 500)
        ));
    }

    // ── Decisions ──────────────────────────────────────────────
    if !snapshot.decisions.is_empty() {
        let mut dec = String::from("## KEY DECISIONS\n\n");
        for d in &snapshot.decisions {
            dec.push_str(&format!("  - {d}\n"));
        }
        sections.push(dec);
    }

    // ── Git State ──────────────────────────────────────────────
    if let Some(ref git) = snapshot.git_state {
        let mut git_text = format!(
            "## GIT STATE\n\n  Branch: {}\n  Status: {}",
            git.branch, git.status_summary
        );
        if !git.recent_commits.is_empty() {
            git_text.push_str("\n\n  Recent commits:\n");
            for c in git.recent_commits.iter().take(5) {
                git_text.push_str(&format!("    {c}\n"));
            }
        }
        if !git.diff_summary.is_empty() {
            git_text.push_str(&format!(
                "\n  Diff summary:\n    {}",
                truncate_smart(&git.diff_summary, 500)
            ));
        }
        sections.push(git_text);
    }

    // ── Recent Files ───────────────────────────────────────────
    if !snapshot.recent_files.is_empty() {
        let mut files = String::from("## RECENTLY CHANGED FILES\n\n");
        for f in snapshot.recent_files.iter().take(20) {
            files.push_str(&format!("  {f}\n"));
        }
        sections.push(files);
    }

    // ── Full Conversation Context ──────────────────────────────
    if !snapshot.conversation.is_empty() {
        let mut convo = String::from("## CONVERSATION CONTEXT\n\nBelow is the full conversation from the Claude session (most recent turns).\nThis is the actual context that was in Claude's window when it was interrupted.\n\n");
        for turn in &snapshot.conversation {
            let prefix = match turn.role.as_str() {
                "user"           => "USER",
                "assistant"      => "CLAUDE",
                "assistant_tool" => "CLAUDE_TOOL",
                "tool_result"    => "TOOL_OUTPUT",
                _                => &turn.role,
            };
            convo.push_str(&format!("[{prefix}] {}\n\n", turn.content));
        }
        sections.push(convo);
    }

    // ── Instructions for agent ─────────────────────────────────
    let instructions = format!(
        "## INSTRUCTIONS\n\n\
        You are continuing work that was started in a Claude Code session.\n\
        The session was interrupted by a rate limit.\n\
        Pick up EXACTLY where it left off. Do NOT re-explain context.\n\
        The user is waiting — be efficient and direct.\n\
        \n\
        Working directory: {}\n\
        {}",
        snapshot.project_dir,
        if snapshot.deadline.is_some() {
            "THERE IS A DEADLINE. Prioritise completing the current task."
        } else { "" }
    );
    sections.push(instructions);

    let mut full = sections.join("\n\n");

    // Truncate if too long (rough token estimate: chars / 3.5)
    let estimated_tokens = full.len() as f64 / 3.5;
    if estimated_tokens > max_tokens as f64 {
        let max_chars = (max_tokens as f64 * 3.5) as usize;
        full.truncate(max_chars);
        full.push_str("\n\n[...truncated to fit context limit]");
    }

    Ok(full)
}

/// Save handoff to a file for reference/debugging.
pub fn save_handoff(handoff: &str, project_dir: &std::path::Path) -> Result<std::path::PathBuf> {
    let relay_dir = project_dir.join(".relay");
    std::fs::create_dir_all(&relay_dir)?;

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let path = relay_dir.join(format!("handoff_{ts}.md"));
    std::fs::write(&path, handoff)?;

    Ok(path)
}

fn truncate_smart(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    // Try to cut at a line boundary
    let cut = &s[..max];
    if let Some(last_nl) = cut.rfind('\n') {
        format!("{}\n[...truncated]", &s[..last_nl])
    } else {
        format!("{}...", &s[..max])
    }
}
