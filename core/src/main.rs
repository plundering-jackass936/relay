use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use std::time::Instant;

use relay::{agents, capture, handoff, tui, Config};

#[derive(Parser)]
#[command(
    name = "relay",
    about = "Relay — When Claude's rate limit hits, another agent picks up where you left off.",
    version = build_version_string(),
    long_version = build_long_version_string(),
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON (no TUI)
    #[arg(long, global = true)]
    json: bool,

    /// Verbose logging
    #[arg(long, short, global = true)]
    verbose: bool,

    /// Project directory
    #[arg(long, global = true)]
    project: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Hand off current session to a fallback agent
    Handoff {
        /// Target agent (codex, claude, aider, gemini, copilot, opencode, ollama, openai)
        #[arg(long)]
        to: Option<String>,

        /// Set deadline urgency (e.g. "7pm", "30min")
        #[arg(long)]
        deadline: Option<String>,

        /// Just print the handoff — don't launch agent
        #[arg(long)]
        dry_run: bool,

        /// Skip secret detection warning
        #[arg(long)]
        force: bool,

        /// How many conversation turns to include (default: 25)
        #[arg(long, default_value = "25")]
        turns: usize,

        /// What to include: all, conversation, git, todos (comma-separated)
        #[arg(long, default_value = "all")]
        include: String,

        /// Copy handoff to clipboard instead of launching agent
        #[arg(long)]
        clipboard: bool,

        /// Handoff template: full (default), minimal, raw
        #[arg(long, default_value = "full")]
        template: String,

        /// Target a specific session by ID (or prefix). Use 'relay sessions' to list.
        #[arg(long)]
        session: Option<String>,

        /// Disable chain fallback when using --to (don't try other agents on failure)
        #[arg(long)]
        no_chain: bool,
    },

    /// List available Claude Code sessions
    Sessions {
        /// Maximum number of sessions to show
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Filter by project path substring
        #[arg(long)]
        filter: Option<String>,
    },

    /// Show current session snapshot
    Status,

    /// List configured agents and availability
    Agents,

    /// Resume after rate limit resets — show what happened during handoff
    Resume,

    /// List past handoffs
    History {
        /// Number of entries to show
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Output format: table (default), json, csv
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Show what changed since the last handoff
    Diff,

    /// Generate default config at ~/.relay/config.toml
    Init,

    /// Validate config and test agent connectivity
    Validate,

    /// Remove old handoff files from .relay/
    Clean {
        /// Number of recent handoffs to keep (default: 5)
        #[arg(long, default_value = "5")]
        keep: usize,

        /// Remove handoffs older than duration (e.g., "7d", "30d", "24h")
        #[arg(long)]
        older_than: Option<String>,

        /// Show what would be removed without actually deleting
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for (bash, zsh, fish, powershell)
        shell: String,
    },

    /// Watch for rate limits and auto-handoff (daemon mode)
    Watch {
        /// Poll interval in seconds (default: 5)
        #[arg(long, default_value = "5")]
        interval: u64,

        /// Cooldown between auto-handoffs in seconds (default: 120)
        #[arg(long, default_value = "120")]
        cooldown: u64,
    },

    /// Replay a saved handoff against any agent
    Replay {
        /// Handoff file path or index (0 = most recent)
        #[arg(default_value = "0")]
        handoff: String,

        /// Target agent
        #[arg(long)]
        to: Option<String>,

        /// Just show what would be replayed
        #[arg(long)]
        dry_run: bool,
    },

    /// Show handoff analytics and stats dashboard
    Stats,

    /// Create a new plugin scaffold
    PluginNew {
        /// Plugin name
        name: String,
    },

    /// PostToolUse hook (auto-detect rate limits)
    Hook {
        #[arg(long, default_value = "unknown")]
        session: String,
    },
}

fn build_version_string() -> &'static str {
    concat!(env!("CARGO_PKG_VERSION"), " (", env!("RELAY_GIT_HASH"), ")")
}

fn build_long_version_string() -> &'static str {
    concat!(
        env!("CARGO_PKG_VERSION"),
        "\n  commit:  ", env!("RELAY_GIT_HASH"),
        "\n  built:   ", env!("RELAY_BUILD_DATE"),
        "\n  rustc:   ", env!("RELAY_RUST_VERSION"),
        "\n  target:  ", env!("RELAY_TARGET"),
    )
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    let project_dir = cli.project
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let config = Config::load().unwrap_or_else(|_| Config {
        general: Default::default(),
        agents: Default::default(),
    });

    match cli.command {
        // ═══════════════════════════════════════════════════════════════
        // HANDOFF
        // ═══════════════════════════════════════════════════════════════
        Commands::Handoff { to, deadline, dry_run, force, turns, include, clipboard, template, session, no_chain } => {
            if !cli.json {
                tui::print_banner();
            }

            // Resolve project directory from --session if provided
            let project_dir = if let Some(ref sid) = session {
                match relay::sessions::find_session(sid)? {
                    Some(entry) => {
                        if !cli.json {
                            eprintln!("  {} Targeting session {} ({})",
                                "📂".to_string(), &entry.session_id[..8], entry.project_path.dimmed());
                            eprintln!();
                        }
                        PathBuf::from(&entry.project_path)
                    }
                    None => {
                        eprintln!("  No session matching '{}'. Run 'relay sessions' to list.", sid);
                        return Ok(());
                    }
                }
            } else {
                project_dir
            };

            let handoff_start = Instant::now();

            // Step 1: Capture
            let sp = if !cli.json { Some(tui::step(1, 3, "Capturing session state...")) } else { None };
            let step1_start = Instant::now();

            relay::capture::session::MAX_CONVERSATION_TURNS
                .store(turns, std::sync::atomic::Ordering::Relaxed);

            let mut snapshot = capture::capture_snapshot(&project_dir, deadline.as_deref())?;

            // Apply include filter
            let includes: Vec<&str> = include.split(',').map(|s| s.trim()).collect();
            if !includes.contains(&"all") {
                if !includes.contains(&"conversation") { snapshot.conversation.clear(); }
                if !includes.contains(&"git") { snapshot.git_state = None; snapshot.recent_files.clear(); }
                if !includes.contains(&"todos") { snapshot.todos.clear(); }
            }

            let capture_ms = step1_start.elapsed().as_millis();
            if let Some(sp) = sp { sp.finish_with_message(format!("Session captured ({capture_ms}ms)")); }

            // Step 2: Build handoff
            let step2_start = Instant::now();
            let sp = if !cli.json { Some(tui::step(2, 3, "Building handoff package...")) } else { None };

            // Resolve target agent
            let target_name = if let Some(ref name) = to {
                name.clone()
            } else if !cli.json && !dry_run {
                // Interactive agent selection
                if let Some(sp) = sp.as_ref() { sp.finish_with_message("Handoff built"); }

                let statuses = agents::check_all_agents(&config);
                let agent_list: Vec<(String, bool, String)> = statuses
                    .iter()
                    .map(|s| (s.name.clone(), s.available, s.reason.clone()))
                    .collect();

                match tui::select_agent(&agent_list) {
                    Some(name) => name,
                    None => {
                        eprintln!("  No agent selected.");
                        return Ok(());
                    }
                }
            } else {
                "auto".into()
            };

            // Build handoff using selected template
            let handoff_text = match handoff::templates::Template::parse(&template) {
                handoff::templates::Template::Minimal => {
                    handoff::templates::build_minimal(&snapshot, &target_name)
                }
                handoff::templates::Template::Raw => {
                    handoff::templates::build_raw(&snapshot)
                }
                handoff::templates::Template::Full => {
                    handoff::build_handoff(&snapshot, &target_name, config.general.max_context_tokens)?
                }
            };
            let handoff_path = handoff::save_handoff(&handoff_text, &project_dir)?;

            let build_ms = step2_start.elapsed().as_millis();
            if let Some(sp) = sp { sp.finish_with_message(format!("Handoff built ({build_ms}ms)")); }

            // Secret detection
            if !force {
                let secrets = relay::secrets::scan_for_secrets(&handoff_text);
                if !secrets.is_empty() && !cli.json {
                    eprintln!();
                    eprintln!("  \u{26a0}\u{fe0f}  {} potential secret(s) detected in handoff:", secrets.len());
                    for s in secrets.iter().take(5) {
                        eprintln!("    - {} (line {}): {}", s.pattern_name, s.line_number, s.redacted_match);
                    }
                    if secrets.len() > 5 {
                        eprintln!("    ... and {} more", secrets.len() - 5);
                    }
                    eprintln!();
                    eprintln!("  Use --force to skip this warning, or review the handoff file:");
                    eprintln!("    {}", handoff_path.display());
                    eprintln!();
                    if !dry_run && !clipboard {
                        return Ok(());
                    }
                }
            }

            // Cost estimation (show for API agents)
            if !cli.json && !dry_run {
                let model = match target_name.as_str() {
                    "gemini" => &config.agents.gemini.model,
                    "openai" => &config.agents.openai.model,
                    "ollama" => &config.agents.ollama.model,
                    _ => "unknown",
                };
                let estimate = relay::cost::estimate_cost(&handoff_text, &target_name, model);
                eprintln!("  \u{1f4b0} {}", relay::cost::format_cost(&estimate));
            }

            // JSON / dry-run output
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "snapshot": snapshot,
                    "handoff_text": handoff_text,
                    "handoff_file": handoff_path.to_string_lossy(),
                    "target_agent": target_name,
                    "timing": {
                        "capture_ms": capture_ms,
                        "build_ms": build_ms,
                    }
                }))?);
                return Ok(());
            }
            // Clipboard mode
            if clipboard {
                #[cfg(target_os = "macos")]
                {
                    use std::process::{Command, Stdio};
                    let mut child = Command::new("pbcopy")
                        .stdin(Stdio::piped())
                        .spawn()?;
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        stdin.write_all(handoff_text.as_bytes())?;
                    }
                    child.wait()?;
                    eprintln!("  📋 Handoff copied to clipboard!");
                    eprintln!("  📄 Also saved: {}", handoff_path.display());
                }
                #[cfg(target_os = "windows")]
                {
                    use std::process::{Command, Stdio};
                    let mut child = Command::new("clip")
                        .stdin(Stdio::piped())
                        .spawn()?;
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        stdin.write_all(handoff_text.as_bytes())?;
                    }
                    child.wait()?;
                    eprintln!("  📋 Handoff copied to clipboard!");
                    eprintln!("  📄 Also saved: {}", handoff_path.display());
                }
                #[cfg(all(unix, not(target_os = "macos")))]
                {
                    use std::process::{Command, Stdio};
                    // Try xclip first (X11), then wl-copy (Wayland)
                    let clipboard_result = if let Ok(mut child) = Command::new("xclip")
                        .arg("-selection")
                        .arg("clipboard")
                        .stdin(Stdio::piped())
                        .spawn()
                    {
                        if let Some(mut stdin) = child.stdin.take() {
                            use std::io::Write;
                            let _ = stdin.write_all(handoff_text.as_bytes());
                        }
                        child.wait().ok()
                    } else if let Ok(mut child) = Command::new("wl-copy")
                        .stdin(Stdio::piped())
                        .spawn()
                    {
                        if let Some(mut stdin) = child.stdin.take() {
                            use std::io::Write;
                            let _ = stdin.write_all(handoff_text.as_bytes());
                        }
                        child.wait().ok()
                    } else {
                        None
                    };

                    if clipboard_result.is_some() {
                        eprintln!("  📋 Handoff copied to clipboard!");
                        eprintln!("  📄 Also saved: {}", handoff_path.display());
                    } else {
                        eprintln!("  Clipboard tools not available (xclip or wl-copy required)");
                        eprintln!("  📄 Saved to: {}", handoff_path.display());
                    }
                }
                return Ok(());
            }
            if dry_run {
                println!("{handoff_text}");
                eprintln!();
                eprintln!("  📄 Saved: {}", handoff_path.display());
                return Ok(());
            }

            // Step 3: Launch agent
            let step3_start = Instant::now();
            let sp = tui::step(3, 3, &format!("Launching {}...", target_name));

            let result = if to.is_some() {
                agents::handoff_to_named(&config, &target_name, &handoff_text, &project_dir.to_string_lossy(), !no_chain)
            } else {
                agents::handoff_to_first_available(&config, &handoff_text, &project_dir.to_string_lossy())
            }?;

            let launch_ms = step3_start.elapsed().as_millis();
            let total_ms = handoff_start.elapsed().as_millis();

            sp.finish_with_message(if result.success {
                format!("{} launched ({launch_ms}ms)", target_name)
            } else {
                "Failed".into()
            });

            // Record in analytics
            if let Ok(db) = relay::analytics::open_db() {
                let _ = relay::analytics::record_handoff(
                    &db, &result.agent, result.success, total_ms,
                    handoff_text.len(), handoff_text.len() / 4,
                    &template, &project_dir.to_string_lossy(),
                    &snapshot.current_task,
                    if result.success { None } else { Some(&result.message) },
                    0,
                );
            }

            if result.success {
                tui::print_handoff_success(&result.agent, &handoff_path.to_string_lossy());
                eprintln!("  \u{23f1}\u{fe0f}  Total: {}ms (capture: {}ms, build: {}ms, launch: {}ms)",
                    total_ms, capture_ms, build_ms, launch_ms);
                eprintln!();
            } else {
                tui::print_handoff_fail(&result.message, &handoff_path.to_string_lossy());
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // SESSIONS
        // ═══════════════════════════════════════════════════════════════
        Commands::Sessions { limit, filter: filter_project } => {
            let sp = if !cli.json { Some(tui::spinner("Scanning sessions...")) } else { None };
            let mut sessions = relay::sessions::list_sessions()?;
            if let Some(sp) = sp { sp.finish_and_clear(); }

            // Apply project filter
            if let Some(ref filter) = filter_project {
                sessions.retain(|s| s.project_path.contains(filter));
            }

            sessions.truncate(limit);

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&sessions)?);
            } else {
                tui::print_sessions(&sessions);
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // STATUS
        // ═══════════════════════════════════════════════════════════════
        Commands::Status => {
            let sp = if !cli.json { Some(tui::spinner("Reading session state...")) } else { None };
            let snapshot = capture::capture_snapshot(&project_dir, None)?;
            if let Some(sp) = sp { sp.finish_and_clear(); }

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                tui::print_snapshot(&snapshot);
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // AGENTS
        // ═══════════════════════════════════════════════════════════════
        Commands::Agents => {
            let sp = if !cli.json { Some(tui::spinner("Checking agents...")) } else { None };
            let statuses = agents::check_all_agents(&config);
            if let Some(sp) = sp { sp.finish_and_clear(); }

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&statuses)?);
            } else {
                tui::print_agents(&config.general.priority, &statuses);
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // RESUME
        // ═══════════════════════════════════════════════════════════════
        Commands::Resume => {
            let report = relay::resume::build_resume(&project_dir)?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                tui::print_section("🔄", "Resume — What Happened During Handoff");
                eprintln!("  Handoff at: {}", report.handoff_time);
                eprintln!("  Task: {}", report.original_task);
                eprintln!();

                if !report.new_commits.is_empty() {
                    tui::print_section("📝", &format!("New Commits ({})", report.new_commits.len()));
                    for c in &report.new_commits {
                        eprintln!("  {c}");
                    }
                }

                if !report.changes_since.is_empty() {
                    tui::print_section("📄", &format!("Changed Files ({})", report.changes_since.len()));
                    for f in &report.changes_since {
                        eprintln!("  {f}");
                    }
                }

                if !report.diff_stat.is_empty() {
                    tui::print_section("📊", "Diff Summary");
                    for line in report.diff_stat.lines() {
                        eprintln!("  {line}");
                    }
                }

                eprintln!();
                eprintln!("  📋 Resume prompt ready. Use --json to get the full prompt.");
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // HISTORY
        // ═══════════════════════════════════════════════════════════════
        Commands::History { limit, format: fmt } => {
            let entries = relay::history::list_handoffs(&project_dir, limit)?;

            if cli.json || fmt == "json" {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }

            if fmt == "csv" {
                println!("timestamp,agent,task,filename");
                for e in &entries {
                    // Escape commas in task field
                    let task = e.task.replace('"', "\"\"");
                    println!("{},{}.\"{}\",{}", e.timestamp, e.agent, task, e.filename);
                }
                return Ok(());
            }

            // Default: table format
            if entries.is_empty() {
                eprintln!("  No handoffs recorded yet.");
                return Ok(());
            }

            tui::print_section("📜", &format!("Handoff History ({} entries)", entries.len()));
            eprintln!();
            for e in &entries {
                eprintln!(
                    "  {}  → {:<10}  {}",
                    e.timestamp, e.agent, e.task
                );
            }
            eprintln!();
        }

        // ═══════════════════════════════════════════════════════════════
        // DIFF
        // ═══════════════════════════════════════════════════════════════
        Commands::Diff => {
            let report = relay::diff::diff_since_handoff(&project_dir)?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
                return Ok(());
            }

            tui::print_section("📊", "Changes Since Last Handoff");
            eprintln!("  Handoff at: {}", report.handoff_time);
            eprintln!(
                "  {} added, {} modified, {} deleted",
                report.files_added, report.files_modified, report.files_deleted
            );

            if !report.new_commits.is_empty() {
                eprintln!();
                eprintln!("  Commits:");
                for c in &report.new_commits {
                    eprintln!("    {c}");
                }
            }

            if !report.diff_stat.is_empty() {
                eprintln!();
                for line in report.diff_stat.lines() {
                    eprintln!("  {line}");
                }
            }
            eprintln!();
        }

        // ═══════════════════════════════════════════════════════════════
        // INIT
        // ═══════════════════════════════════════════════════════════════
        Commands::Init => {
            let path = relay::config_path();
            if path.exists() {
                eprintln!("  Config exists: {}", path.display());
                eprintln!("  Edit to add API keys and customize priority.");
            } else {
                Config::save_default(&path)?;
                eprintln!("  ✅ Config created: {}", path.display());
                eprintln!();
                eprintln!("  Add API keys:");
                eprintln!("    [agents.gemini]");
                eprintln!("    api_key = \"your-key\"");
                eprintln!();
                eprintln!("    [agents.openai]");
                eprintln!("    api_key = \"your-key\"");
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // VALIDATE
        // ═══════════════════════════════════════════════════════════════
        Commands::Validate => {
            if !cli.json {
                eprintln!();
                eprintln!("  {}  {}", "🔍", "Validating Relay Configuration".bold());
                eprintln!("  {}", "─".repeat(50).dimmed());
                eprintln!();
            }

            let results = relay::validate::validate_config(&config);

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&results)?);
                return Ok(());
            }

            let mut all_ok = true;
            for r in &results {
                let (icon, status_str) = match r.status.as_str() {
                    "ok" => ("✅", r.message.green().to_string()),
                    "warn" => { all_ok = false; ("⚠️ ", r.message.yellow().to_string()) },
                    _ => { all_ok = false; ("❌", r.message.red().to_string()) },
                };
                eprintln!("  {} {:<12} {}", icon, r.agent.bold(), status_str);
            }

            eprintln!();
            if all_ok {
                eprintln!("  {} All agents validated successfully!", "🚀".to_string());
            } else {
                eprintln!("  {} Some agents need attention. Run 'relay init' to configure.", "💡".to_string());
            }
            eprintln!();
        }

        // ═══════════════════════════════════════════════════════════════
        // CLEAN
        // ═══════════════════════════════════════════════════════════════
        Commands::Clean { keep, older_than, dry_run } => {
            let older_secs = older_than.as_deref().and_then(relay::clean::parse_duration);
            let result = relay::clean::clean_handoffs(&project_dir, keep, older_secs, dry_run)?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
                return Ok(());
            }

            if result.removed.is_empty() {
                eprintln!("  No handoff files to clean.");
            } else {
                let action = if dry_run { "Would remove" } else { "Removed" };
                eprintln!("  {} {} handoff file(s), freed ~{} KB",
                    action, result.removed.len(), result.bytes_freed / 1024);
                for f in &result.removed {
                    eprintln!("    - {f}");
                }
            }
            if !result.kept.is_empty() {
                eprintln!("  Kept {} file(s)", result.kept.len());
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // COMPLETIONS
        // ═══════════════════════════════════════════════════════════════
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            use clap_complete::{generate, Shell};

            let shell = match shell.to_lowercase().as_str() {
                "bash" => Shell::Bash,
                "zsh" => Shell::Zsh,
                "fish" => Shell::Fish,
                "powershell" | "pwsh" => Shell::PowerShell,
                _ => {
                    eprintln!("  Unknown shell: {shell}. Supported: bash, zsh, fish, powershell");
                    return Ok(());
                }
            };

            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "relay", &mut std::io::stdout());
        }

        // ═══════════════════════════════════════════════════════════════
        // WATCH (daemon mode)
        // ═══════════════════════════════════════════════════════════════
        Commands::Watch { interval, cooldown } => {
            if !cli.json {
                tui::print_banner();
            }
            let watch_config = relay::watch::WatchConfig {
                poll_interval: std::time::Duration::from_secs(interval),
                cooldown: std::time::Duration::from_secs(cooldown),
            };
            relay::watch::run_watch(&project_dir, &config, &watch_config)?;
        }

        // ═══════════════════════════════════════════════════════════════
        // REPLAY
        // ═══════════════════════════════════════════════════════════════
        Commands::Replay { handoff, to, dry_run } => {
            let path = relay::replay::resolve_handoff_path(&project_dir, &handoff)?;
            eprintln!("  Replaying: {}", path.display());

            let result = relay::replay::replay_handoff(&path, &config, to.as_deref(), dry_run)?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
                return Ok(());
            }

            if result.success {
                eprintln!("  \u{2705} Replay to {} succeeded ({} bytes)", result.agent, result.handoff_size);
            } else {
                eprintln!("  \u{274c} Replay failed: {}", result.message);
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // STATS
        // ═══════════════════════════════════════════════════════════════
        Commands::Stats => {
            let db = relay::analytics::open_db()?;
            let stats = relay::analytics::get_stats(&db)?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&stats)?);
                return Ok(());
            }

            eprintln!();
            eprintln!("  {}", "═".repeat(50).cyan());
            eprintln!("  {}  {}", "📊", "Relay Analytics".bold().cyan());
            eprintln!("  {}", "═".repeat(50).cyan());
            eprintln!();

            eprintln!("  Total handoffs:     {}", stats.total_handoffs.to_string().bold());
            eprintln!("  Successful:         {} ({}%)", stats.successful.to_string().green(), format!("{:.0}", stats.success_rate).green());
            eprintln!("  Failed:             {}", stats.failed.to_string().red());
            eprintln!("  Avg duration:       {}ms", stats.avg_duration_ms);
            eprintln!("  Est. time saved:    {} min", format!("{:.0}", stats.total_time_saved_est_min).green().bold());

            if !stats.agents.is_empty() {
                eprintln!();
                eprintln!("  {}", "Agent Breakdown".bold());
                eprintln!("  {}", "─".repeat(50).dimmed());
                for a in &stats.agents {
                    eprintln!("  {:<12} {:>3} handoffs ({} ok, {} fail) avg {}ms",
                        a.agent.bold(), a.total, a.successful.to_string().green(),
                        a.failed.to_string().red(), a.avg_duration_ms);
                }
            }

            if !stats.recent.is_empty() {
                eprintln!();
                eprintln!("  {}", "Recent Handoffs".bold());
                eprintln!("  {}", "─".repeat(50).dimmed());
                for h in stats.recent.iter().take(5) {
                    let icon = if h.success { "\u{2705}" } else { "\u{274c}" };
                    let task = if h.task.len() > 40 {
                        format!("{}...", &h.task[..37])
                    } else {
                        h.task.clone()
                    };
                    eprintln!("  {} {} → {:<10} {}", icon, h.timestamp.dimmed(), h.agent, task);
                }
            }

            if stats.total_handoffs == 0 {
                eprintln!();
                eprintln!("  {}", "No handoffs recorded yet. Run 'relay handoff' to get started.".dimmed());
            }
            eprintln!();
        }

        // ═══════════════════════════════════════════════════════════════
        // PLUGIN-NEW
        // ═══════════════════════════════════════════════════════════════
        Commands::PluginNew { name } => {
            let path = relay::plugins::scaffold_plugin(&name)?;
            eprintln!("  \u{2705} Plugin scaffolded: {}", path.display());
            eprintln!();
            eprintln!("  Edit these files:");
            eprintln!("    {}/plugin.toml    — metadata", path.display());
            eprintln!("    {}/handoff.sh     — your agent logic", path.display());
            eprintln!();
            eprintln!("  The handoff text is piped to stdin of your script.");
        }

        // ═══════════════════════════════════════════════════════════════
        // HOOK
        // ═══════════════════════════════════════════════════════════════
        Commands::Hook { session: _ } => {
            use std::io::Read;
            let mut raw = String::new();
            std::io::stdin().read_to_string(&mut raw)?;

            if let Some(detection) = relay::detect::check_hook_output(&raw) {
                eprintln!(
                    "  🚨 Rate limit detected in {} (signal: {})",
                    detection.tool_name, detection.signal
                );
                if config.general.auto_handoff {
                    let snapshot = capture::capture_snapshot(&project_dir, None)?;
                    let handoff_text = handoff::build_handoff(&snapshot, "auto", config.general.max_context_tokens)?;
                    let handoff_path = handoff::save_handoff(&handoff_text, &project_dir)?;
                    let result = agents::handoff_to_first_available(
                        &config, &handoff_text, &project_dir.to_string_lossy(),
                    )?;
                    if result.success {
                        eprintln!("  ✅ Auto-handed off to {}", result.agent);
                    } else {
                        eprintln!("  📄 Saved: {}", handoff_path.display());
                    }
                }
            }
            print!("{raw}");
        }
    }

    Ok(())
}
