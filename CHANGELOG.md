# Changelog

## [1.1.0] - 2026-04-10

### Added
- `relay validate` — Test all configured agents' connectivity and API key validity
- `relay clean` — Remove old handoff files with `--keep N`, `--older-than DURATION`, `--dry-run`
- `relay completions` — Generate shell completions for bash, zsh, fish, powershell
- **Secret detection** — Scans handoff text for API keys, tokens, passwords, private keys before sending; blocks unless `--force`
- **Retry with exponential backoff** — API agents (Gemini, OpenAI, Ollama) retry on 429/5xx/network errors (3 retries, 1s/2s/4s)
- **Smart context compression** — Priority-based truncation: critical info always kept, conversation trimmed from old turns, dropped sections noted
- **Duration tracking** — Each handoff step shows elapsed time (capture, build, launch) in milliseconds
- **History export** — `relay history --format json|csv|table`
- **Build metadata** — `relay --version` shows git hash; `relay -V` shows commit, date, rustc, target
- **`--force` flag** — Skip secret detection warning on handoff
- Cross-platform clipboard support (macOS pbcopy, Linux xclip/wl-copy, Windows clip)
- Cross-platform CI (macOS, Ubuntu, Windows)

### Changed
- **DRY refactor** — `capture/todos.rs` now reuses `session.rs` path-finding instead of duplicating it
- Smart handoff compression replaces naive end-truncation
- Test count: 32 → 48 tests

### Fixed
- Validate test no longer depends on local CLI availability

## [1.0.0] - 2026-04-05

### Added
- `relay resume` — Show what happened during handoff, generate resume prompt for Claude
- `relay history` — List all past handoffs with timestamp, agent, task
- `relay diff` — Show files changed, new commits, diff stats since last handoff
- `--clipboard` flag — Copy handoff to clipboard (macOS pbcopy)
- `--template` flag — Choose handoff format: `full`, `minimal`, `raw`
- CHANGELOG.md, CONTRIBUTING.md
- GitHub issue templates (bug report, feature request)
- GitHub Release with pre-built binary
- Git tags for all versions

### Changed
- Bumped to v1.0.0 — production ready

## [0.5.0] - 2026-04-05

### Added
- Beautiful TUI with animated spinners, colored output, progress steps
- Interactive fuzzy-select agent picker when no `--to` specified
- Box-drawn sections with emoji headers

## [0.4.0] - 2026-04-05

### Added
- 8 agents: Codex, Claude, Aider, Gemini, Copilot, OpenCode, Ollama, OpenAI
- `--turns N` flag to control conversation context size
- `--include` flag to filter what context is sent
- Agents don't auto-execute — confirm context and wait for user

### Fixed
- Ollama timeout hang (switched to curl with --max-time)
- Copilot --version hang (skip version check)

## [0.3.0] - 2026-04-05

### Added
- Context control flags (--turns, --include)

### Fixed
- Codex prompt overflow from too many conversation turns
- Reduced per-turn content size

## [0.2.0] - 2026-04-05

### Added
- Full Claude conversation context capture from .jsonl transcripts
- Reads user messages, assistant responses, tool calls, tool results
- Extracts decisions, errors, TodoWrite state
- 80 conversation turns captured from live session

### Changed
- Session capture no longer git-only — reads actual Claude context

## [0.1.0] - 2026-04-05

### Added
- Initial release
- Session capture: git state, branch, commits, diff
- Handoff builder with structured prompt format
- 4 agent adapters: Codex, Gemini, Ollama, OpenAI
- Rate limit detection via PostToolUse hook
- CLI: handoff, status, agents, init, hook
- Config system via ~/.relay/config.toml
