//! Context scoring engine — assigns relevance scores to each handoff section.
//! Higher scores = more important = kept during compression.

use crate::SessionSnapshot;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScoredSection {
    pub name: String,
    pub score: f64,
    pub bytes: usize,
    pub reason: String,
}

/// Score all sections of a snapshot for priority-based compression.
pub fn score_snapshot(snapshot: &SessionSnapshot) -> Vec<ScoredSection> {
    let mut sections = Vec::new();

    // Current task — always critical
    sections.push(ScoredSection {
        name: "current_task".into(),
        score: 100.0,
        bytes: snapshot.current_task.len(),
        reason: "Active task — always needed".into(),
    });

    // Last error — very high if present (likely the reason for handoff)
    if let Some(ref err) = snapshot.last_error {
        sections.push(ScoredSection {
            name: "last_error".into(),
            score: 95.0,
            bytes: err.len(),
            reason: "Error context — likely cause of handoff".into(),
        });
    }

    // Git state
    if let Some(ref git) = snapshot.git_state {
        let git_bytes = git.branch.len() + git.status_summary.len() + git.diff_summary.len();
        sections.push(ScoredSection {
            name: "git_state".into(),
            score: 80.0,
            bytes: git_bytes,
            reason: "Repository context — branch, changes, commits".into(),
        });
    }

    // Decisions
    if !snapshot.decisions.is_empty() {
        let bytes: usize = snapshot.decisions.iter().map(|d| d.len()).sum();
        let recency_boost = (snapshot.decisions.len() as f64).min(5.0) * 2.0;
        sections.push(ScoredSection {
            name: "decisions".into(),
            score: 70.0 + recency_boost,
            bytes,
            reason: format!("{} decisions — architectural context", snapshot.decisions.len()),
        });
    }

    // Conversation turns — score by recency
    if !snapshot.conversation.is_empty() {
        let total = snapshot.conversation.len();
        let recent_count = total.min(5);
        let recent_bytes: usize = snapshot.conversation[total - recent_count..]
            .iter().map(|t| t.content.len()).sum();
        let old_bytes: usize = snapshot.conversation[..total - recent_count]
            .iter().map(|t| t.content.len()).sum();

        // Recent turns score high
        sections.push(ScoredSection {
            name: "conversation_recent".into(),
            score: 85.0,
            bytes: recent_bytes,
            reason: format!("Last {} conversation turns — most relevant", recent_count),
        });

        // Older turns score lower
        if old_bytes > 0 {
            let age_penalty = ((total - recent_count) as f64 / 10.0).min(30.0);
            sections.push(ScoredSection {
                name: "conversation_old".into(),
                score: 40.0 - age_penalty,
                bytes: old_bytes,
                reason: format!("{} older turns — decreasing relevance", total - recent_count),
            });
        }
    }

    // Todos
    if !snapshot.todos.is_empty() {
        let in_progress = snapshot.todos.iter().filter(|t| t.status == "in_progress").count();
        let bytes: usize = snapshot.todos.iter().map(|t| t.content.len() + t.status.len()).sum();
        let progress_boost = in_progress as f64 * 10.0;
        sections.push(ScoredSection {
            name: "todos".into(),
            score: 50.0 + progress_boost,
            bytes,
            reason: format!("{} items ({} in progress)", snapshot.todos.len(), in_progress),
        });
    }

    // Recent files
    if !snapshot.recent_files.is_empty() {
        let bytes: usize = snapshot.recent_files.iter().map(|f| f.len()).sum();
        sections.push(ScoredSection {
            name: "recent_files".into(),
            score: 30.0,
            bytes,
            reason: format!("{} changed files", snapshot.recent_files.len()),
        });
    }

    // Last output
    if let Some(ref out) = snapshot.last_output {
        sections.push(ScoredSection {
            name: "last_output".into(),
            score: 25.0,
            bytes: out.len(),
            reason: "Last tool output".into(),
        });
    }

    // Sort by score descending
    sections.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    sections
}

/// Given a token budget, return which sections to keep and which to drop.
pub fn budget_allocation(sections: &[ScoredSection], max_chars: usize) -> (Vec<String>, Vec<String>) {
    let mut budget = max_chars;
    let mut keep = Vec::new();
    let mut drop = Vec::new();

    for section in sections {
        if section.bytes < budget {
            budget -= section.bytes;
            keep.push(section.name.clone());
        } else {
            drop.push(format!("{} ({} bytes, score {:.0})", section.name, section.bytes, section.score));
        }
    }

    (keep, drop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn test_snapshot() -> SessionSnapshot {
        SessionSnapshot {
            current_task: "Fix authentication bug".into(),
            todos: vec![
                TodoItem { content: "Review auth flow".into(), status: "in_progress".into() },
                TodoItem { content: "Write tests".into(), status: "pending".into() },
            ],
            decisions: vec!["Using JWT tokens".into()],
            last_error: Some("401 Unauthorized".into()),
            last_output: Some("test failed".into()),
            git_state: Some(GitState {
                branch: "fix/auth".into(),
                status_summary: "2 changes".into(),
                recent_commits: vec!["abc Fix login".into()],
                diff_summary: "2 files".into(),
                uncommitted_files: vec!["src/auth.rs".into()],
            }),
            project_dir: "/tmp/test".into(),
            recent_files: vec!["src/auth.rs".into()],
            timestamp: "2026-04-10".into(),
            deadline: None,
            conversation: vec![
                ConversationTurn { role: "user".into(), content: "fix auth".into() },
                ConversationTurn { role: "assistant".into(), content: "checking".into() },
            ],
        }
    }

    #[test]
    fn task_always_highest_score() {
        let scores = score_snapshot(&test_snapshot());
        assert_eq!(scores[0].name, "current_task");
        assert_eq!(scores[0].score, 100.0);
    }

    #[test]
    fn error_scores_very_high() {
        let scores = score_snapshot(&test_snapshot());
        let error = scores.iter().find(|s| s.name == "last_error").unwrap();
        assert!(error.score >= 90.0);
    }

    #[test]
    fn in_progress_todos_score_higher() {
        let scores = score_snapshot(&test_snapshot());
        let todos = scores.iter().find(|s| s.name == "todos").unwrap();
        assert!(todos.score > 50.0); // boosted by in_progress item
    }

    #[test]
    fn budget_drops_lowest_scored() {
        let scores = score_snapshot(&test_snapshot());
        let (keep, drop) = budget_allocation(&scores, 100); // very tight budget
        // Should keep high-score items and drop low-score ones
        assert!(keep.contains(&"current_task".to_string()));
        assert!(!drop.is_empty());
    }
}
