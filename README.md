# Relay

**When Claude Code hits its rate limit, another agent picks up exactly where you left off — with full conversation context.**

[![CI](https://github.com/Manavarya09/relay/actions/workflows/ci.yml/badge.svg)](https://github.com/Manavarya09/relay/actions)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![npm](https://img.shields.io/npm/v/@masyv/relay)](https://www.npmjs.com/package/@masyv/relay)
[![GitHub Release](https://img.shields.io/github/v/release/Manavarya09/relay)](https://github.com/Manavarya09/relay/releases)
[![Tests](https://img.shields.io/badge/tests-48_passing-brightgreen.svg)](https://github.com/Manavarya09/relay/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Features

- **Full conversation capture** — Reads Claude's actual `.jsonl` transcript, not just git
- **8 agent adapters** — Codex, Claude, Aider, Gemini, Copilot, OpenCode, Ollama, OpenAI
- **Interactive TUI** — Spinners, progress steps, fuzzy agent picker, color-coded output
- **Smart context compression** — Priority-based truncation keeps critical info, drops low-value context
- **Secret detection** — Scans handoff for API keys, tokens, passwords before sending
- **Retry with backoff** — API agents retry transient failures (429, 5xx) automatically
- **Shell completions** — `relay completions bash|zsh|fish|powershell`
- **Config validation** — `relay validate` tests all agents before you need them
- **Handoff cleanup** — `relay clean --keep 5` purges old handoff files
- **Duration tracking** — Each handoff step shows elapsed time in milliseconds
- **`relay resume`** — When Claude comes back, see what the fallback agent did
- **`relay history`** — Browse past handoffs with `--format json|csv|table`
- **`relay diff`** — Show exactly what changed during the handoff
- **Clipboard mode** — `--clipboard` copies handoff for pasting into any tool
- **Handoff templates** — `--template minimal|full|raw` for different formats
- **Rate limit auto-detection** — PostToolUse hook triggers handoff automatically
- **Context control** — `--turns 10 --include git,todos` to customize
- **Zero network capture** — Pure local file parsing, < 100ms
- **5 MB binary** — Rust, no runtime, no GC

## The Problem

It's 6:20 PM. Your submission is at 7 PM. You're deep in a Claude Code session — 45 minutes of context, decisions, half-finished code. Then:

> **Rate limit reached. Please wait.**

Your entire session context is gone. You open Codex or Gemini and spend 20 minutes re-explaining everything. By the time you're set up, it's 6:50.

## The Solution

```bash
relay handoff --to codex
```

Relay reads your **actual Claude Code session** — the full conversation, every tool call, every file edit, every error — compresses it into a handoff package, and opens Codex (or Gemini, Aider, Ollama, etc.) with complete context. The new agent knows exactly what you were doing and waits for your instructions.

## What Relay Captures

This is NOT just git state. Relay reads Claude's actual `.jsonl` session transcript:

```
  ════════════════════════════════════════════════════════
  📋  Session Snapshot
  ════════════════════════════════════════════════════════

  📁  /Users/dev/myproject
  🕐  2026-04-05 14:46

  🎯 Current Task
  ──────────────────────────────────────────────────
  Fix the mobile/desktop page separation in the footer

  📝 Progress
  ──────────────────────────────────────────────────
  ✅  Database schema + REST API
  ✅  Landing page overhaul
  🔄  Footer link separation (IN PROGRESS)
  ⏳  Auth system

  🚨 Last Error
  ──────────────────────────────────────────────────
  Error: Next.js couldn't find the package from project directory

  💡 Key Decisions
  ──────────────────────────────────────────────────
  • Using Socket.io instead of raw WebSockets
  • Clean reinstall fixed the @next/swc-darwin-arm64 issue

  💬 Conversation (25 turns)
  ──────────────────────────────────────────────────
  🤖 AI   Now update the landing page footer too.
  🔧 TOOL [Edit] pages/index.tsx (replacing 488 chars)
  📤 OUT  File updated successfully.
  🤖 AI   Add /mobile to the Layout bypass list.
  🔧 TOOL [Edit] components/Layout.tsx (replacing 99 chars)
  🔧 TOOL [Bash] npx next build
  📤 OUT  ✓ Build passed — 12 pages compiled
```

## 8 Supported Agents

| Agent | Type | How it launches |
|-------|------|-----------------|
| **Codex** | CLI (OpenAI) | Opens interactive TUI with context |
| **Claude** | CLI (Anthropic) | New Claude session with context |
| **Aider** | CLI (open source) | Opens with --message handoff |
| **Gemini** | API / CLI | Gemini CLI or REST API |
| **Copilot** | CLI (GitHub) | Opens with context |
| **OpenCode** | CLI (Go) | Opens with context |
| **Ollama** | Local API | REST call to local model |
| **OpenAI** | API | GPT-4o / GPT-5.4 API call |

## Quick Start

```bash
# Install
git clone https://github.com/Manavarya09/relay
cd relay && ./scripts/build.sh

# Symlink to PATH
ln -sf $(pwd)/core/target/release/relay ~/.cargo/bin/relay

# Generate config
relay init

# Validate your setup
relay validate

# Check what agents you have
relay agents

# See your current session snapshot
relay status

# Hand off to Codex (interactive — opens TUI)
relay handoff --to codex

# Interactive agent picker
relay handoff

# With deadline urgency
relay handoff --to codex --deadline "7:00 PM"

# Copy to clipboard instead
relay handoff --clipboard

# Minimal handoff (just task + error + git)
relay handoff --template minimal --to codex

# When Claude comes back — see what happened
relay resume

# List all past handoffs
relay history

# Export history as CSV
relay history --format csv

# What changed since handoff?
relay diff

# Clean up old handoff files (keep last 5)
relay clean

# Clean files older than 7 days
relay clean --older-than 7d --dry-run
```

## Context Control

```bash
# Default: last 25 conversation turns + everything
relay handoff --to codex

# Light: 10 turns only
relay handoff --to codex --turns 10

# Only git state + todos (no conversation)
relay handoff --to codex --include git,todos

# Only conversation
relay handoff --to codex --include conversation

# Dry run — see what gets sent without launching
relay handoff --dry-run

# Skip secret detection warning
relay handoff --to codex --force
```

## Shell Completions

```bash
# Bash
relay completions bash > /etc/bash_completion.d/relay

# Zsh
relay completions zsh > ~/.zfunc/_relay

# Fish
relay completions fish > ~/.config/fish/completions/relay.fish
```

## Secret Detection

Before sending a handoff to any agent, Relay scans for:
- AWS access keys and secret keys
- OpenAI API keys (`sk-...`)
- GitHub tokens (`ghp_...`)
- Private keys (`-----BEGIN...PRIVATE KEY-----`)
- Database connection strings
- Generic API keys, tokens, and passwords

If secrets are detected, the handoff is blocked with a warning. Use `--force` to override.

## How It Works

1. **Reads** `~/.claude/projects/<project>/<session>.jsonl` — Claude's actual transcript
2. **Extracts** user messages, assistant responses, tool calls, tool results, errors
3. **Reads** TodoWrite state from the JSONL (your live todo list)
4. **Captures** git branch, diff summary, uncommitted files, recent commits
5. **Compresses** with smart priority — critical context always kept, low-value dropped first
6. **Scans** for secrets and warns before sending sensitive data
7. **Retries** API calls with exponential backoff on transient failures
8. **Launches** the agent interactively with inherited stdin/stdout

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

[agents.openai]
api_key = "your-key"

[agents.ollama]
url = "http://localhost:11434"
model = "llama3"
```

## Auto-Handoff (PostToolUse Hook)

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

Relay detects rate limit signals in tool output and automatically hands off.

## All Commands

| Command | Description |
|---------|-------------|
| `relay handoff` | Hand off to a fallback agent |
| `relay status` | Show current session snapshot |
| `relay agents` | List configured agents and availability |
| `relay resume` | Show what happened during handoff |
| `relay history` | List past handoffs (--format json/csv/table) |
| `relay diff` | Show changes since last handoff |
| `relay init` | Generate default config |
| `relay validate` | Test agent connectivity and API keys |
| `relay clean` | Remove old handoff files |
| `relay completions` | Generate shell completions |
| `relay hook` | PostToolUse hook for auto-detection |

## Performance

- **5 MB** binary (Rust, stripped, LTO)
- **< 100ms** session capture
- **Zero network calls** for capture
- **48 tests** passing
- **Rust** — no runtime, no GC

## License

MIT

---

Built by [@masyv](https://github.com/Manavarya09)
