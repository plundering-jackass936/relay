# Relay Codebase Interview Guide

> **What Relay is in one line:** a Rust CLI that captures the *real* working context from a Claude Code session, compresses it into a safe handoff package, and transfers you to another coding agent (Codex, Gemini, OpenAI, Ollama, etc.) so work continues without re-explaining everything.

---

## 1) Product story (what it does)

Relay solves a concrete workflow failure: Claude Code can hit rate limits mid-task, and context continuity breaks. Instead of restarting from scratch, Relay:

1. Reads Claude transcript data from local `.jsonl` session files.
2. Extracts task state (current task, tool output, errors, decisions, todos, recent conversation).
3. Adds git context (branch, status, commits, changed files).
4. Builds a structured markdown handoff prompt.
5. Compresses the prompt by relevance score to fit token budgets.
6. Scans for secret leakage.
7. Sends handoff to a fallback agent (or chain of agents).
8. Stores handoff artifacts and analytics locally.

This is positioned as a continuity layer between coding agents, not a model itself.

---

## 2) How the project is structured

## Top-level

- `core/` → main Rust application (all CLI/business logic).
- `vscode-extension/` → VS Code integration that shells out to `relay`.
- `scripts/` + `package.json` + `bin/relay` → npm distribution wrapper that installs prebuilt binaries.
- `docs/` and `assets/` → static docs/media.

In interviews, say the architecture is **"Rust core + JS packaging + optional IDE frontend"**.

---

## 3) Runtime architecture (Rust core)

### 3.1 Entry point and command surface

`core/src/main.rs` uses `clap` to define a multi-command CLI (`handoff`, `watch`, `status`, `agents`, `sessions`, `history`, `diff`, `resume`, `replay`, etc.).

Operationally:

- Loads config from `~/.relay/config.toml`.
- Resolves project directory (`--project` or current working dir).
- Routes to command handlers.
- For `handoff`, runs a 3-step pipeline: capture → build package → launch agent.

### 3.2 Shared types and config

`core/src/lib.rs` defines:

- Config types (`GeneralConfig`, `AgentsConfig`, per-agent configs).
- Persistent paths (`config_path()`, `data_dir()`).
- Core domain structs (`SessionSnapshot`, `ConversationTurn`, `GitState`, `HandoffResult`, `AgentStatus`).

This gives a stable “contract” across modules.

---

## 4) End-to-end handoff flow (the most important part)

### Step A — Capture session snapshot

`capture::capture_snapshot()` aggregates from three sources:

- `capture/session.rs` → Claude transcript parsing.
- `capture/git.rs` → repo state (branch/status/commits/diff/files).
- `capture/todos.rs` → todo state.

Transcript parser details (`capture/session.rs`):

- Locates Claude session directory under `~/.claude/projects/...`.
- Picks most recent `.jsonl` transcript.
- Parses each JSON line and classifies user text, assistant text, tool calls, and tool results.
- Extracts:
  - current task (last substantive user prompt)
  - last error / last tool output
  - decisions (heuristically from assistant lines)
  - bounded conversation turn list (via `MAX_CONVERSATION_TURNS`)

### Step B — Build handoff package

`handoff::build_handoff()` creates sectioned markdown:

- current task
- progress / todos
- last error
- decisions
- git state
- changed files
- conversation context
- instructions for receiving agent

Then it applies token-budget compression:

- converts token budget to char budget,
- scores snapshot sections via `scoring::score_snapshot()`,
- keeps highest value sections first,
- may trim older conversation text,
- appends a compression notice if content dropped.

### Step C — Safety and delivery

Before launch, `secrets::scan_for_secrets()` runs regex checks for likely credentials (AWS/OpenAI/GitHub tokens, private keys, connection strings, generic api keys/secrets).

Then Relay:

- saves handoff markdown to `.relay/handoff_*.md`,
- estimates cost for API-backed targets,
- executes selected agent or priority-chain fallback.

Agent dispatch lives in `agents/mod.rs` through a shared `Agent` trait:

- `check_available()`
- `execute(handoff_prompt, project_dir)`

Built-ins include codex/claude/aider/gemini/copilot/opencode/ollama/openai, plus plugin agents loaded at runtime.

---

## 5) Reliability features

### Chain fallback

If a chosen agent is unavailable/fails, Relay can automatically try the next agent in configured priority order (`handoff_to_named(..., chain=true)` and `handoff_to_first_available*` helpers).

### Watch mode (daemon)

`watch.rs` runs a polling loop:

- checks latest Claude `.jsonl` growth,
- tail-scans for rate-limit signals from `detect::is_rate_limited()`,
- respects cooldown,
- auto-captures and auto-handoffs,
- records chain depth + result.

This is essentially an event loop with lightweight file-tail semantics.

### Replay/History/Diff/Resume

- `replay.rs`: replay any saved handoff against any agent.
- `history.rs`: enumerate saved handoff files and extract metadata.
- `diff.rs`: report git changes since last handoff.
- `resume.rs`: summarize what happened while user was away.

These features make handoff auditable and debuggable.

---

## 6) Data and storage model

Relay stores runtime data in `~/.relay`:

- `config.toml` for settings.
- `handoff_*.md` artifacts under each project’s `.relay/` folder.
- `analytics.db` (SQLite) for events and aggregate stats.

`analytics.rs` creates two tables:

- `handoffs` (event log with success, duration, context size, template, task, chain depth).
- `agent_stats` (per-agent rollups).

This design is local-first and privacy-friendly (no central server dependency).

---

## 7) Extensibility model

### Plugin agents

`plugins.rs` discovers `~/.relay/plugins/<name>/plugin.toml` and wires each plugin into the same `Agent` trait.

Plugin contract:

- optional availability check script,
- executable handoff command that receives prompt on stdin,
- inherits project working directory.

This avoids recompiling Rust to support new agent endpoints.

### VS Code extension

`vscode-extension/` exposes commands + session panel and delegates execution to `relay` CLI. The extension acts as UX layer, while core behavior remains in Rust.

---

## 8) How it was made (engineering choices)

### Why Rust core

- Strong fit for a fast local CLI.
- Safe file/process handling.
- Portable binaries across macOS/Linux/Windows.

### Why npm wrapper

`package.json` + `scripts/postinstall.js` let users install from npm while still running native Rust binaries. Postinstall downloads platform asset from GitHub Releases and drops it into `bin/`.

### Why markdown handoff format

- Human-readable artifacts for debugging.
- Easy for any target LLM/agent input channel.
- Versionable and inspectable in repo/project history.

---

## 9) Interview talking points (what to emphasize)

## High-level pitch

“Relay is a context continuity orchestrator for coding agents. It converts volatile in-session state into a structured, compressed, and safe transfer package, then routes execution to the best available fallback agent.”

## Interesting technical decisions

- **Heuristic transcript extraction** from Claude JSONL rather than relying only on git state.
- **Score-based context compression** with explicit relevance ranking.
- **Cross-agent abstraction** using a trait-based adapter layer.
- **Chain-of-responsibility fallback** for reliability.
- **Local observability** via SQLite analytics and handoff artifacts.

## Tradeoffs to mention

- Heuristic parsing can miss semantics if transcript formats evolve.
- Regex secret scanning may have false positives/false negatives.
- Character-budget approximation is a rough proxy for real tokenizer counts.
- Polling-based watch mode is simple but less elegant than event-driven hooks.

## Improvements you can suggest

- Pluggable tokenizers per model for accurate budgeting.
- Richer semantic summarization (embedding-aware chunking).
- Incremental transcript parsing with persisted cursor offsets.
- Stronger secret scanning (entropy + allowlist + policy engine).
- OpenTelemetry-style traces for command execution paths.

---

## 10) 90-second interview walkthrough script

“Relay is a Rust CLI that protects coding momentum when an AI agent gets rate-limited. The core command is `relay handoff`. Internally it captures session state from Claude transcript JSONL plus git metadata and todos, normalizes this into a `SessionSnapshot`, and builds a markdown handoff package. Because prompts can exceed context limits, Relay scores sections by importance and compresses to budget, prioritizing active task and recent errors. It runs secret detection before delivery, persists a handoff artifact to disk, and then dispatches to an adapter-based agent layer. If the chosen agent fails, Relay can chain to the next available one. There’s also daemon mode (`relay watch`) that auto-detects rate-limit signals and performs this flow automatically. The system is local-first, auditable through saved handoff files, and measurable through SQLite analytics.”

---

## 11) Quick file map for interview prep

- CLI routing: `core/src/main.rs`
- Core types/config: `core/src/lib.rs`
- Snapshot capture: `core/src/capture/mod.rs`
- Transcript parser: `core/src/capture/session.rs`
- Handoff builder: `core/src/handoff/mod.rs`
- Compression/scoring: `core/src/scoring.rs`
- Agent abstraction/routing: `core/src/agents/mod.rs`
- Auto-watch daemon: `core/src/watch.rs`
- Secret scan: `core/src/secrets.rs`
- Analytics DB: `core/src/analytics.rs`
- Plugin system: `core/src/plugins.rs`
- npm install pipeline: `scripts/postinstall.js`, `package.json`
- VS Code activation entry: `vscode-extension/src/extension.ts`

---

## 12) If interviewer asks “what’s hard here?”

Answer: **preserving useful context density under strict token limits while keeping safety and reliability**.

That combines:

- noisy transcript parsing,
- prioritization/compression,
- multi-agent compatibility,
- secure handoff handling,
- robust fallback behavior.

That’s the core engineering value in this codebase.
