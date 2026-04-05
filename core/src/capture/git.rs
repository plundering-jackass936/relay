//! Capture git state: branch, diff, status, recent commits.

use crate::GitState;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn capture_git_state(project_dir: &Path) -> Result<GitState> {
    let branch = run_git(project_dir, &["branch", "--show-current"])?
        .trim()
        .to_string();

    let status = run_git(project_dir, &["status", "--short"])?;
    let uncommitted_files: Vec<String> = status
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let status_summary = if uncommitted_files.is_empty() {
        "Clean working tree".to_string()
    } else {
        format!("{} uncommitted changes", uncommitted_files.len())
    };

    // Recent commits (last 5, one-line)
    let log = run_git(
        project_dir,
        &["log", "--oneline", "-5", "--no-decorate"],
    )
    .unwrap_or_default();
    let recent_commits: Vec<String> = log
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    // Diff summary (stat, not full diff — keeps handoff small)
    let diff_summary = run_git(project_dir, &["diff", "--stat", "HEAD"])
        .unwrap_or_default()
        .trim()
        .to_string();

    Ok(GitState {
        branch,
        status_summary,
        recent_commits,
        diff_summary,
        uncommitted_files,
    })
}

fn run_git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .context("failed to run git")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
