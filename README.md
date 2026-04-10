# Relay

**When Claude Code hits its rate limit, another agent picks up exactly where you left off.**

[![CI](https://github.com/Manavarya09/relay/actions/workflows/ci.yml/badge.svg)](https://github.com/Manavarya09/relay/actions)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![GitHub Release](https://img.shields.io/github/v/release/Manavarya09/relay)](https://github.com/Manavarya09/relay/releases)
[![Tests](https://img.shields.io/badge/tests-62_passing-brightgreen.svg)](https://github.com/Manavarya09/relay/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

<br/>

https://github.com/Manavarya09/relay/releases/download/v1.2.0/relay-launch-final.mp4

<br/>

---

## The Problem

You're 45 minutes into a Claude Code session. Debugging, editing files, running tests. Claude has full context on everything. Then:

```
Error: Rate limit reached. Your request has been throttled.
```

All that context is gone. You open Codex or Gemini and spend 20 minutes re-explaining everything from scratch.

## The Solution

```bash
relay handoff --to codex
```

Relay reads Claude's actual `.jsonl` session transcript -- every conversation turn, tool call, file edit, error, decision -- compresses it with relevance-based scoring, scans for leaked secrets, estimates the API cost, and launches the fallback agent with complete context.

If that agent fails, it automatically chains to the next one in your priority list.

Or just run the daemon and never think about it:

```bash
relay watch
```

Zero intervention. Relay monitors your session, detects the rate limit, and hands off automatically.

---

## How It Works

https://github.com/Manavarya09/relay/releases/download/v1.2.0/relay-workflow-final.mp4

<br/>

1. **Reads** `~/.claude/projects/<project>/<session>.jsonl` -- Claude's actual session transcript
2. **Extracts** conversation turns, tool calls with results, errors, decisions, TodoWrite state
3. **Captures** git branch, diff summary, uncommitted files, recent commits
4. **Scores** each context section by relevance -- recency, error proximity, decision importance
5. **Compresses** with priority-based truncation -- critical info always kept, low-value dropped first
6. **Scans** for API keys, tokens, passwords, private keys before sending
7. **Estimates** token count and cost for API agents
8. **Chains** through agents in priority order until one succeeds
9. **Records** the result in a local SQLite database for analytics

---

## What Gets Captured

This is not just git state. Relay reads the actual Claude transcript:

```
  manav@mbp ~/myproject $ relay status

  Session Snapshot
  ══════════════════════════════════════════════════

  /Users/dev/myproject
  2026-04-10 14:46

  Current Task
  ──────────────────────────────────────────────────
  Fix the JWT validation in auth middleware

  Progress
  ──────────────────────────────────────────────────
  [done]        Database schema + REST API
  [done]        Landing page overhaul
  [IN PROGRESS] Auth middleware
  [pending]     Frontend dashboard

  Last Error
  ──────────────────────────────────────────────────
  error[E0499]: cannot borrow `state` as mutable more than once

  Key Decisions
  ──────────────────────────────────────────────────
  - Using JWT instead of session cookies
  - Redis for token blacklist

  Conversation (25 turns)
  ──────────────────────────────────────────────────
  AI   Now update the route handlers to use the new middleware.
  TOOL [Edit] src/middleware/auth.rs (replacing 234 chars)
  OUT  File updated successfully.
  TOOL [Bash] cargo test -- auth
  OUT  test result: ok. 6 passed; 0 failed
```

The fallback agent gets everything. No re-explaining.

---

## Quick Start

```bash
git clone https://github.com/Manavarya09/relay
cd relay && ./scripts/build.sh

ln -sf $(pwd)/core/target/release/relay ~/.cargo/bin/relay

relay init
relay validate
relay agents
```

```bash
# Interactive agent picker
relay handoff

# Specific agent
relay handoff --to codex

# With deadline urgency
relay handoff --to codex --deadline "7:00 PM"

# Minimal context (just task + error + git)
relay handoff --template minimal --to codex

# Copy to clipboard instead of launching
relay handoff --clipboard

# Dry run -- see what gets sent
relay handoff --dry-run

# Daemon mode -- auto-detects and hands off
relay watch
```

---

## Agents

8 built-in adapters plus a plugin system for custom agents.

| Agent | Type | Launch Method |
|-------|------|---------------|
| Codex | CLI (OpenAI) | Opens interactive TUI with context |
| Claude | CLI (Anthropic) | New Claude session with handoff |
| Aider | CLI (open source) | Opens with `--message` handoff |
| Gemini | API / CLI | CLI or REST API with retry |
| Copilot | CLI (GitHub) | Opens with context |
| OpenCode | CLI (Go) | Opens with context |
| Ollama | Local API | REST call to local model with retry |
| OpenAI | API | Chat completions API with retry |
| Plugins | Custom | Your own agents via TOML + shell script |

### Handoff Chains

If the first agent fails, Relay cascades to the next:

```
  [1] Trying codex... not available
  [2] Trying gemini... API error (429)
  [3] Trying ollama... done
       Handed off to ollama
```

Works in daemon mode too. Complete resilience.

---

## Plugin System

Create custom agents without writing Rust:

```bash
relay plugin-new my-agent
```

Creates `~/.relay/plugins/my-agent/`:

```
plugin.toml     -- metadata and config
handoff.sh      -- your agent logic (receives handoff on stdin)
```

```toml
[plugin]
name = "my-agent"
description = "Custom internal agent"
version = "0.1.0"
command = "./handoff.sh"
```

Plugins are auto-discovered and show up in `relay agents`.

---

## Analytics

Every handoff is tracked in a local SQLite database.

```bash
relay stats
```

```
  Relay Analytics
  ══════════════════════════════════════════════════

  Total handoffs:     47
  Successful:         43 (91%)
  Failed:             4
  Avg duration:       1,250ms
  Est. time saved:    645 min

  Agent Breakdown
  ──────────────────────────────────────────────────
  codex          28 handoffs (27 ok, 1 fail) avg 890ms
  gemini         12 handoffs (11 ok, 1 fail) avg 2100ms
  ollama          7 handoffs (5 ok, 2 fail) avg 1800ms
```

---

## Secret Detection

Before sending a handoff, Relay scans for:

- AWS access keys and secret keys
- OpenAI API keys (`sk-...`)
- GitHub tokens (`ghp_...`, `gho_...`)
- Private keys (`-----BEGIN...PRIVATE KEY-----`)
- Database connection strings
- Slack tokens, bearer tokens, generic passwords

If detected, the handoff is blocked with a redacted warning. Use `--force` to override.

---

## Cost Estimation

For API agents, Relay shows the estimated cost before sending:

```
  ~2,400 tokens (~$0.006 on gpt-4o)
```

CLI agents show `(free -- local/CLI agent)`.

---

## Context Control

```bash
# Default: last 25 conversation turns + everything
relay handoff --to codex

# Light: 10 turns only
relay handoff --to codex --turns 10

# Only git state + todos
relay handoff --to codex --include git,todos

# Templates: full (default), minimal, raw
relay handoff --template minimal --to codex

# Skip secret detection
relay handoff --to codex --force
```

---

## Daemon Mode

```bash
relay watch
relay watch --interval 10 --cooldown 300
```

The daemon polls Claude's transcript for new content, checks for rate limit signals, captures session state, chains through agents, and records the result. Set it and forget it.

---

## All Commands

| Command | Description |
|---------|-------------|
| `relay handoff` | Hand off to a fallback agent |
| `relay watch` | Daemon mode -- auto-detect and hand off |
| `relay replay` | Re-send a saved handoff to any agent |
| `relay stats` | Analytics dashboard |
| `relay status` | Current session snapshot |
| `relay agents` | List agents and plugins |
| `relay resume` | Show what the fallback agent did |
| `relay history` | Past handoffs (json, csv, table) |
| `relay diff` | Changes since last handoff |
| `relay validate` | Test agent connectivity |
| `relay clean` | Remove old handoff files |
| `relay completions` | Shell completions (bash, zsh, fish) |
| `relay plugin-new` | Scaffold a custom agent |
| `relay init` | Generate default config |
| `relay hook` | PostToolUse hook for auto-detection |

---

## Config

`~/.relay/config.toml`:

```toml
[general]
priority = ["codex", "claude", "aider", "gemini", "copilot", "opencode", "ollama", "openai"]
max_context_tokens = 8000
auto_handoff = true

[agents.codex]
model = "o4-mini"

[agents.gemini]
api_key = "your-key"
model = "gemini-2.5-pro"

[agents.openai]
api_key = "your-key"
model = "gpt-4o"

[agents.ollama]
url = "http://localhost:11434"
model = "llama3"
```

---

## Auto-Handoff Hook

Add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "PostToolUse": [
      { "matcher": "*", "hooks": [{ "type": "command", "command": "relay hook" }] }
    ]
  }
}
```

---

## Shell Completions

```bash
relay completions bash > /etc/bash_completion.d/relay
relay completions zsh > ~/.zfunc/_relay
relay completions fish > ~/.config/fish/completions/relay.fish
```

---

## GitHub Action

```yaml
- uses: masyv/relay-action@v1
  with:
    agent: auto
    gemini-api-key: ${{ secrets.GEMINI_API_KEY }}
```

---

## Architecture

```
Claude .jsonl --> capture --> score --> compress --> scan secrets --> estimate cost
                                                         |
                                                  chain handoff (agent1 --> agent2 --> agent3)
                                                         |
                                                  record analytics --> SQLite
                                                         |
                                                  relay stats dashboard
```

---

## Performance

| Metric | Value |
|--------|-------|
| Binary size | 5 MB |
| Session capture | < 100ms |
| Network calls for capture | Zero |
| Tests | 62 passing |
| Runtime | Rust, no GC |

---

## License

MIT

---

Built by [@masyv](https://github.com/Manavarya09)
