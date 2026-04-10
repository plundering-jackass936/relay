# Changelog

## [1.2.0] - 2026-04-10

### Added
- **`relay watch`** — Daemon mode: monitors Claude session, auto-handoffs on rate limit. Configurable poll interval (--interval) and cooldown (--cooldown)
- **`relay replay`** — Re-send saved handoffs to any agent for testing/comparison. Supports index (0=most recent) or file path
- **`relay stats`** — TUI analytics dashboard: success rates, agent breakdown, time-saved estimates, recent handoffs
- **`relay plugin-new`** — Scaffold custom agent plugins with TOML config + shell script template
- **Handoff chains** — Auto-cascade through agents in priority order when first fails. Shows chain progress
- **SQLite analytics** — Every handoff tracked: agent, duration, tokens, success/fail, chain depth. Powers `relay stats`
- **Cost estimation** — Token count + USD pricing shown before API handoffs (GPT-4o, Gemini, etc.)
- **Context scoring engine** — Relevance-based section scoring: recency, error proximity, in-progress boost, importance heuristics
- **Plugin system** — Custom agents via `~/.relay/plugins/` with TOML metadata + executable scripts. Auto-discovered
- **GitHub Action** — `action/action.yml` for CI/CD rate limit resilience
- `rusqlite` dependency for analytics database

### Changed
- Agent discovery now also loads plugin agents automatically
- Handoff flow now records to analytics DB and shows cost estimate
- Test count: 48 → 62

## [1.1.0] - 2026-04-10

### Added
- `relay validate` — Test all configured agents' connectivity and API key validity
- `relay clean` — Remove old handoff files with `--keep N`, `--older-than DURATION`, `--dry-run`
- `relay completions` — Generate shell completions for bash, zsh, fish, powershell
- **Secret detection** — Scans handoff text for API keys, tokens, passwords, private keys before sending; blocks unless `--force`
- **Retry with exponential backoff** — API agents (Gemini, OpenAI, Ollama) retry on 429/5xx/network errors
- **Smart context compression** — Priority-based truncation: critical info always kept, conversation trimmed from old turns
- **Duration tracking** — Each handoff step shows elapsed time in milliseconds
- **History export** — `relay history --format json|csv|table`
- **Build metadata** — `relay --version` shows git hash; `relay -V` shows commit, date, rustc, target
- **`--force` flag** — Skip secret detection warning on handoff
- Cross-platform clipboard support (macOS, Linux, Windows)
- Cross-platform CI (macOS, Ubuntu, Windows)

### Changed
- DRY refactor: `capture/todos.rs` reuses `session.rs` path-finding
- Smart compression replaces naive end-truncation
- Test count: 32 → 48

## [1.0.0] - 2026-04-05

### Added
- `relay resume` — Show what happened during handoff
- `relay history` — List past handoffs with timestamp, agent, task
- `relay diff` — Show files changed since last handoff
- `--clipboard` flag — Copy handoff to clipboard
- `--template` flag — Choose format: `full`, `minimal`, `raw`
- CHANGELOG.md, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md
- GitHub issue templates, PR template
- GitHub Release with pre-built binary

### Changed
- Bumped to v1.0.0 — production ready

## [0.5.0] - 2026-04-05

### Added
- Beautiful TUI with animated spinners, colored output, progress steps
- Interactive fuzzy-select agent picker
- Box-drawn sections with emoji headers

## [0.4.0] - 2026-04-05

### Added
- 8 agents: Codex, Claude, Aider, Gemini, Copilot, OpenCode, Ollama, OpenAI
- `--turns N` and `--include` flags for context control
- Agents confirm context and wait for user

### Fixed
- Ollama timeout hang, Copilot --version hang

## [0.3.0] - 2026-04-05

### Added
- Context control flags

### Fixed
- Codex prompt overflow

## [0.2.0] - 2026-04-05

### Added
- Full Claude conversation context capture from .jsonl transcripts
- Reads user messages, assistant responses, tool calls, tool results
- Extracts decisions, errors, TodoWrite state

## [0.1.0] - 2026-04-05

### Added
- Initial release
- Session capture, handoff builder, 4 agent adapters
- Rate limit detection via PostToolUse hook
- CLI and config system
