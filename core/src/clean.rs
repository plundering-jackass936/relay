//! `relay clean` — Remove old handoff files from .relay/ directory.

use anyhow::Result;
use std::path::Path;

#[derive(Debug, serde::Serialize)]
pub struct CleanResult {
    pub removed: Vec<String>,
    pub kept: Vec<String>,
    pub bytes_freed: u64,
}

/// Clean old handoff files, keeping the N most recent.
pub fn clean_handoffs(project_dir: &Path, keep: usize, older_than_secs: Option<u64>, dry_run: bool) -> Result<CleanResult> {
    let relay_dir = project_dir.join(".relay");
    if !relay_dir.exists() {
        return Ok(CleanResult { removed: Vec::new(), kept: Vec::new(), bytes_freed: 0 });
    }

    let mut entries: Vec<(String, std::path::PathBuf, std::time::SystemTime, u64)> = Vec::new();

    for entry in std::fs::read_dir(&relay_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("handoff_") && name.ends_with(".md") {
            if let Ok(meta) = entry.metadata() {
                let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                entries.push((name, entry.path(), modified, meta.len()));
            }
        }
    }

    // Sort newest first
    entries.sort_by(|a, b| b.2.cmp(&a.2));

    let mut removed = Vec::new();
    let mut kept = Vec::new();
    let mut bytes_freed = 0u64;
    let now = std::time::SystemTime::now();

    for (i, (name, path, modified, size)) in entries.iter().enumerate() {
        let mut should_remove = i >= keep;

        // Also check age if --older-than specified
        if let Some(max_age) = older_than_secs {
            if let Ok(age) = now.duration_since(*modified) {
                if age.as_secs() > max_age {
                    should_remove = true;
                }
            }
        }

        // Never remove if within the keep count and no age filter
        if i < keep && older_than_secs.is_none() {
            should_remove = false;
        }

        if should_remove {
            if !dry_run {
                let _ = std::fs::remove_file(path);
            }
            removed.push(name.clone());
            bytes_freed += size;
        } else {
            kept.push(name.clone());
        }
    }

    Ok(CleanResult { removed, kept, bytes_freed })
}

/// Parse a duration string like "7d", "30d", "24h" into seconds.
pub fn parse_duration(s: &str) -> Option<u64> {
    let s = s.trim().to_lowercase();
    if s.ends_with('d') {
        s[..s.len()-1].parse::<u64>().ok().map(|d| d * 86400)
    } else if s.ends_with('h') {
        s[..s.len()-1].parse::<u64>().ok().map(|h| h * 3600)
    } else if s.ends_with('m') {
        s[..s.len()-1].parse::<u64>().ok().map(|m| m * 60)
    } else {
        s.parse::<u64>().ok().map(|d| d * 86400) // default to days
    }
}
