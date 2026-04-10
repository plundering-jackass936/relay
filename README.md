# Relay

**When Claude Code hits its rate limit, another agent picks up exactly where you left off — with full conversation context.**

[![CI](https://github.com/Manavarya09/relay/actions/workflows/ci.yml/badge.svg)](https://github.com/Manavarya09/relay/actions)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![npm](https://img.shields.io/npm/v/@masyv/relay)](https://www.npmjs.com/package/@masyv/relay)
[![GitHub Release](https://img.shields.io/github/v/release/Manavarya09/relay)](https://github.com/Manavarya09/relay/releases)
[![Tests](https://img.shields.io/badge/tests-62_passing-brightgreen.svg)](https://github.com/Manavarya09/relay/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## The Problem

You're deep in a Claude Code session — 45 minutes of context, decisions, half-finished code. Then:

> **Rate limit reached. Please wait.**

Your entire session context is gone. You open Codex or Gemini and spend 20 minutes re-explaining everything.

## The Solution

```bash
relay handoff --to codex
```

Relay reads your **actual Claude Code session transcript** — every conversation turn, tool call, file edit, error — compresses it with smart priority scoring, scans for leaked secrets, estimates the API cost, and launches the fallback agent with complete context. If that agent fails, it automatically chains to the next one.

Or just run the daemon:

```bash
relay watch
```

Zero intervention. Relay monitors your session, detects rate limits, and hands off automatically.

## Features

### Core
- **Full conversation capture** — Reads Claude's `.jsonl` transcript, not just git state
- **8 built-in agents** — Codex, Claude, Aider, Gemini, Copilot, OpenCode, Ollama, OpenAI
- **Plugin system** — Add custom agents via TOML + shell script (no Rust needed)
- **Handoff chains** — If first agent fails, auto-cascades to the next in priority

### Intelligence
- **Context scoring engine** — Relevance-based scoring: recency, error proximity, decision importance
- **Smart compression** — Priority-based truncation keeps critical info, drops low-value context
- **Secret detection** — Scans for API keys, tokens, passwords, private keys before sending
- **Cost estimation** — Shows token count + USD estimate before API handoffs

### Operations
- **Daemon mode** (`relay watch`) — Background monitoring with auto-handoff
- **SQLite analytics** — Tracks every handoff: success rate, duration, agent performance
- **Stats dashboard** (`relay stats`) — TUI dashboard with time-saved estimates
- **Rate limit auto-detection** — PostToolUse hook for zero-config automation

### Developer Experience
- **16 CLI commands** — handoff, watch, replay, stats, validate, clean, history, diff, and more
- **Shell completions** — bash, zsh, fish, powershell
- **GitHub Action** — `uses: masyv/relay-action@v1` for CI/CD resilience
- **JSON output** — Every command supports `--json` for scripting
- **62 tests** — Comprehensive coverage across all modules

## Quick Start

```bash
# Install
git clone https://github.com/Manavarya09/relay
cd relay && ./scripts/build.sh

# Symlink to PATH
ln -sf $(pwd)/core/target/release/relay ~/.cargo/bin/relay

# Generate config + validate setup
relay init
relay validate

# See what agents you have
relay agents

# Hand off (interactive agent picker)
relay handoff

# Hand off to specific agent
relay handoff --to codex

# Or just run the daemon — zero intervention
relay watch
```

## What Relay Captures

```
  ════════════════════════════════════════════════════════
  📋  Session Snapshot
  ════════════════════════════════════════════════════════

  📁  /Users/dev/myproject
  🕐  2026-04-10 14:46

  🎯 Current Task
  ──────────────────────────────────────────────────
  Fix the mobile/desktop page separation in the footer

  📝 Progress
  ──────────────────────────────────────────────────
  ✅  Database schema + REST API
  🔄  Footer link separation (IN PROGRESS)
  ⏳  Auth system

  🚨 Last Error
  ──────────────────────────────────────────────────
  error[E0499]: cannot borrow as mutable

  💬 Conversation (25 turns)
  ──────────────────────────────────────────────────
  🤖 AI   Now update the landing page footer too.
  🔧 TOOL [Edit] pages/index.tsx (replacing 488 chars)
  📤 OUT  File updated successfully.
  🤖 AI   Add /mobile to the Layout bypass list.
  🔧 TOOL [Bash] npx next build
  📤 OUT  ✓ Build passed — 12 pages compiled
```

## All Commands

| Command | Description |
|---------|-------------|
| `relay handoff` | Hand off to a fallback agent (with cost estimation + secret scanning) |
| `relay watch` | Daemon mode — auto-detects rate limits, hands off automatically |
| `relay replay` | Re-send a saved handoff to any agent for testing |
| `relay stats` | TUI dashboard — success rates, time saved, agent breakdown |
| `relay status` | Show current session snapshot |
| `relay agents` | List configured agents + plugins and availability |
| `relay resume` | Show what the fallback agent did |
| `relay history` | List past handoffs (`--format json\|csv\|table`) |
| `relay diff` | Show changes since last handoff |
| `relay validate` | Test all agent connectivity and API keys |
| `relay clean` | Remove old handoff files (`--keep N`, `--older-than 7d`) |
| `relay completions` | Generate shell completions (bash/zsh/fish/powershell) |
| `relay plugin-new` | Scaffold a custom agent plugin |
| `relay init` | Generate default config |
| `relay hook` | PostToolUse hook for auto-detection |

## 8+ Supported Agents

| Agent | Type | How it launches |
|-------|------|-----------------|
| **Codex** | CLI (OpenAI) | Opens interactive TUI with context |
| **Claude** | CLI (Anthropic) | New Claude session with context |
| **Aider** | CLI (open source) | Opens with --message handoff |
| **Gemini** | API / CLI | Gemini CLI or REST API (with retry) |
| **Copilot** | CLI (GitHub) | Opens with context |
| **OpenCode** | CLI (Go) | Opens with context |
| **Ollama** | Local API | REST call to local model (with retry) |
| **OpenAI** | API | GPT-4o / GPT-5.4 API call (with retry) |
| **Plugins** | Custom | Your own agents via TOML + shell script |

## Handoff Chains

When you run `relay handoff`, if the first agent fails, Relay automatically tries the next one:

```
  [1] Trying codex... ❌ Not available
  [2] Trying gemini... ❌ API error (429)
  [3] Trying ollama... ✅ Handed off to ollama
```

This also works in daemon mode (`relay watch`) — complete resilience.

## Plugin System

Create custom agents without writing Rust:

```bash
relay plugin-new my-agent
```

This creates `~/.relay/plugins/my-agent/`:

```
plugin.toml     # Metadata + config
handoff.sh      # Your agent logic (receives handoff on stdin)
```

Example `plugin.toml`:
```toml
[plugin]
name = "my-agent"
description = "Custom internal agent"
version = "0.1.0"
command = "./handoff.sh"
```

Plugins are auto-discovered and appear in `relay agents`.

## Analytics Dashboard

Every handoff is tracked in a local SQLite database:

```bash
relay stats
```

```
  ════════════════════════════════════════════════════════
  📊  Relay Analytics
  ════════════════════════════════════════════════════════

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

## Secret Detection

Before sending a handoff, Relay scans for:
- AWS access keys and secret keys
- OpenAI API keys (`sk-...`)
- GitHub tokens (`ghp_...`, `gho_...`)
- Private keys (`-----BEGIN...PRIVATE KEY-----`)
- Database connection strings (`postgres://`, `mongodb://`)
- Slack tokens, bearer tokens, generic API keys/passwords

If detected, the handoff is **blocked** with a redacted warning. Use `--force` to override.

## Cost Estimation

For API agents, Relay shows the estimated cost before sending:

```
  💰 ~2,400 tokens (~$0.006 on gpt-4o)
```

Free agents (CLI-based like Codex, Aider, Ollama) show `(free — local/CLI agent)`.

## Context Control

```bash
# Default: last 25 conversation turns + everything
relay handoff --to codex

# Light: 10 turns only
relay handoff --to codex --turns 10

# Only git state + todos
relay handoff --to codex --include git,todos

# Minimal handoff (just task + error + git)
relay handoff --template minimal --to codex

# Copy to clipboard
relay handoff --clipboard

# Dry run
relay handoff --dry-run
```

## Daemon Mode

```bash
# Start watching (polls every 5s, 2min cooldown between handoffs)
relay watch

# Custom intervals
relay watch --interval 10 --cooldown 300
```

The daemon:
1. Polls Claude's JSONL transcript for new content
2. Checks last 5 lines for rate limit signals
3. Auto-captures session state
4. Chains through agents until one succeeds
5. Records result in analytics

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

## GitHub Action

```yaml
- uses: masyv/relay-action@v1
  with:
    agent: auto
    gemini-api-key: ${{ secrets.GEMINI_API_KEY }}
```

## Shell Completions

```bash
relay completions bash > /etc/bash_completion.d/relay
relay completions zsh > ~/.zfunc/_relay
relay completions fish > ~/.config/fish/completions/relay.fish
```

## Architecture

```
Claude .jsonl → capture → score → compress → scan secrets → estimate cost
                                                     ↓
                                              chain handoff (agent1 → agent2 → agent3)
                                                     ↓
                                              record analytics → SQLite
                                                     ↓
                                              relay stats dashboard
```

## Performance

- **5 MB** binary (Rust, stripped, LTO)
- **< 100ms** session capture
- **Zero network calls** for capture
- **62 tests** passing
- **Rust** — no runtime, no GC

## License

MIT

---

Built by [@masyv](https://github.com/Manavarya09)
