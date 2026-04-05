# Relay

**When Claude's rate limit hits, another agent picks up exactly where you left off.**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![npm](https://img.shields.io/npm/v/@masyv/relay)](https://www.npmjs.com/package/@masyv/relay)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## The Problem

You're building a feature. It's 6:20 PM. You need to submit by 7 PM. Claude hits its rate limit.

Your entire session context — what you were building, your todos, the last error you were debugging, the architectural decisions you made — all gone. You have to re-explain everything to a new tool. By the time you're set up, it's 6:45 PM.

**Relay fixes this.** It captures your full session state and hands it to Codex, Gemini, Ollama, or GPT-4 — automatically, with complete context — so work never stops.

## How It Works

```
Claude Code session running...
   | (rate limit hit)
   v
Relay captures session state:
  - Current task (from conversation)
  - Todo list + status (from TodoWrite)
  - Git branch, diff, recent commits
  - Last error / last tool output
  - Key decisions made
  - Deadline (if set)
   |
   v
Relay dispatches to fallback agent:
  -> Codex CLI (if installed)
  -> Gemini (if API key set)
  -> Ollama (if running locally)
  -> GPT-4 (if API key set)
   |
   v
Agent picks up EXACTLY where you left off.
```

## Quick Start

```bash
# Install
git clone https://github.com/Manavarya09/relay
cd relay && ./scripts/build.sh && ./scripts/install.sh

# Generate config
relay init

# Check available agents
relay agents

# See what would be handed off
relay status

# Manual handoff (now)
relay handoff

# Handoff to specific agent with deadline
relay handoff --to codex --deadline "7:00 PM"

# Dry run — just print the handoff package
relay handoff --dry-run
```

## What Relay Captures

```
═══ Relay Session Snapshot ═══

Project: /Users/dev/myproject
Captured: 2026-04-05 13:32:02

── Current Task ──
  Building WebSocket handler in src/server/ws.rs

── Todos ──
  ✅ [completed] Database schema + REST API
  🔄 [in_progress] WebSocket handler (60% done)
  ⏳ [pending] Frontend charts
  ⏳ [pending] Auth

── Last Error ──
  error[E0499]: cannot borrow `state` as mutable...

── Decisions ──
  • Using Socket.io instead of raw WebSockets
  • Redis pub/sub for cross-server events

── Git ──
  Branch: feature/websocket
  3 uncommitted changes
  Recent: abc1234 Add WebSocket route skeleton
```

## Agent Priority

Configure in `~/.relay/config.toml`:

```toml
[general]
priority = ["codex", "gemini", "ollama", "openai"]
auto_handoff = true
max_context_tokens = 8000

[agents.codex]
model = "o4-mini"

[agents.gemini]
api_key = "your-key"
model = "gemini-2.5-pro"

[agents.ollama]
url = "http://localhost:11434"
model = "llama3"

[agents.openai]
api_key = "your-key"
model = "gpt-4o"
```

Relay tries agents in priority order and uses the first available one.

## CLI

```
COMMANDS:
  handoff   Hand off to fallback agent (--to, --deadline, --dry-run)
  status    Show current session snapshot
  agents    List agents and availability
  init      Generate default config
  hook      PostToolUse hook (auto-detect rate limits)

OPTIONS:
  --json       Output as JSON
  --project    Project directory (default: cwd)
  -v           Verbose logging
```

## Auto-Handoff via Hook

Add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "*",
        "hooks": [{ "type": "command", "command": "relay hook" }]
      }
    ]
  }
}
```

Relay will detect rate limit signals in tool output and automatically hand off.

## Performance

- **4.6 MB** binary (release, stripped)
- **< 100ms** to capture full session snapshot
- **Zero network calls** for capture (git + file reads only)

## License

MIT
