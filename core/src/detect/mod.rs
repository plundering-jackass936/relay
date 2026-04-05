//! Rate limit detection — identifies when Claude Code has hit its limit.

use anyhow::Result;

/// Signals from tool output that suggest a rate limit.
static RATE_LIMIT_SIGNALS: &[&str] = &[
    "rate limit",
    "rate_limit",
    "quota exceeded",
    "too many requests",
    "429",
    "capacity",
    "overloaded",
    "try again later",
    "usage limit",
    "limit reached",
    "context window full",
    "maximum context",
];

/// Check if a tool output string contains rate limit signals.
pub fn is_rate_limited(text: &str) -> bool {
    let lower = text.to_lowercase();
    RATE_LIMIT_SIGNALS.iter().any(|sig| lower.contains(sig))
}

/// Hook handler — reads PostToolUse JSON, checks for rate limit,
/// triggers handoff if detected.
pub fn check_hook_output(raw: &str) -> Option<RateLimitDetection> {
    let val: serde_json::Value = serde_json::from_str(raw).ok()?;

    let tool_output = val.get("tool_output").and_then(|v| v.as_str()).unwrap_or("");
    let tool_name = val.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");

    if is_rate_limited(tool_output) {
        return Some(RateLimitDetection {
            tool_name: tool_name.to_string(),
            signal: extract_signal(tool_output),
            full_output: tool_output.to_string(),
        });
    }

    None
}

/// Watch the Claude Code process for exit with rate limit.
pub fn watch_claude_process() -> Result<WatchResult> {
    // Check if Claude Code is running
    let output = std::process::Command::new("pgrep")
        .args(["-f", "claude"])
        .output()?;

    let running = output.status.success();

    Ok(WatchResult {
        claude_running: running,
        rate_limited: false, // Can't detect from outside without hook
    })
}

pub struct RateLimitDetection {
    pub tool_name: String,
    pub signal: String,
    pub full_output: String,
}

pub struct WatchResult {
    pub claude_running: bool,
    pub rate_limited: bool,
}

fn extract_signal(text: &str) -> String {
    let lower = text.to_lowercase();
    for sig in RATE_LIMIT_SIGNALS {
        if lower.contains(sig) {
            return sig.to_string();
        }
    }
    "unknown".into()
}
