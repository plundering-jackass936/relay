use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use relay::{agents, capture, handoff, Config};

#[derive(Parser)]
#[command(
    name = "relay",
    about = "Relay — When Claude's rate limit hits, another agent picks up where you left off.",
    long_about = "Captures your Claude Code session state (task, todos, git diff, decisions,\nerrors) and hands it off to Codex, Gemini, Ollama, or GPT-4 — so your\nwork never stops.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON
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
    /// Hand off current session to a fallback agent right now
    Handoff {
        /// Force a specific agent (codex, gemini, ollama, openai)
        #[arg(long)]
        to: Option<String>,

        /// Set deadline urgency (e.g. "7pm", "19:00", "30min")
        #[arg(long)]
        deadline: Option<String>,

        /// Don't execute — just print the handoff package
        #[arg(long)]
        dry_run: bool,
    },

    /// Show current session snapshot (what would be handed off)
    Status,

    /// List configured agents and their availability
    Agents,

    /// Generate default config file at ~/.relay/config.toml
    Init,

    /// PostToolUse hook mode (auto-detect rate limits from stdin)
    Hook {
        /// Session ID
        #[arg(long, default_value = "unknown")]
        session: String,
    },
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
        Commands::Handoff { to, deadline, dry_run } => {
            eprintln!("{}", "⚡ Relay — capturing session state...".yellow().bold());

            let snapshot = capture::capture_snapshot(
                &project_dir,
                deadline.as_deref(),
            )?;

            let target = to.as_deref().unwrap_or("auto");
            let handoff_text = handoff::build_handoff(
                &snapshot,
                target,
                config.general.max_context_tokens,
            )?;

            // Save handoff file for reference
            let handoff_path = handoff::save_handoff(&handoff_text, &project_dir)?;

            if dry_run || cli.json {
                if cli.json {
                    let result = serde_json::json!({
                        "snapshot": snapshot,
                        "handoff_text": handoff_text,
                        "handoff_file": handoff_path.to_string_lossy(),
                        "target_agent": target,
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{handoff_text}");
                    eprintln!();
                    eprintln!("{}", format!("📄 Saved to: {}", handoff_path.display()).dimmed());
                }
                return Ok(());
            }

            eprintln!("{}", format!("📄 Handoff saved: {}", handoff_path.display()).dimmed());
            eprintln!();

            // Execute handoff
            let result = if let Some(ref agent_name) = to {
                agents::handoff_to_named(&config, agent_name, &handoff_text, &project_dir.to_string_lossy())
            } else {
                agents::handoff_to_first_available(&config, &handoff_text, &project_dir.to_string_lossy())
            }?;

            if result.success {
                eprintln!("{}", format!("✅ Handed off to {}", result.agent).green().bold());
                eprintln!("   {}", result.message);
            } else {
                eprintln!("{}", format!("❌ Handoff failed: {}", result.message).red());
                eprintln!();
                eprintln!("💡 The handoff context was saved to:");
                eprintln!("   {}", handoff_path.display());
                eprintln!("   You can copy-paste it into any AI assistant manually.");
            }
        }

        Commands::Status => {
            let snapshot = capture::capture_snapshot(&project_dir, None)?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
                return Ok(());
            }

            println!("{}", "═══ Relay Session Snapshot ═══".bold());
            println!();
            println!("{}: {}", "Project".bold(), snapshot.project_dir);
            println!("{}: {}", "Captured".bold(), snapshot.timestamp);
            println!();

            println!("{}", "── Current Task ──".cyan());
            println!("  {}", snapshot.current_task);
            println!();

            if !snapshot.todos.is_empty() {
                println!("{}", "── Todos ──".cyan());
                for t in &snapshot.todos {
                    let icon = match t.status.as_str() {
                        "completed"   => "✅",
                        "in_progress" => "🔄",
                        _             => "⏳",
                    };
                    println!("  {icon} [{}] {}", t.status, t.content);
                }
                println!();
            }

            if let Some(ref err) = snapshot.last_error {
                println!("{}", "── Last Error ──".red());
                println!("  {err}");
                println!();
            }

            if !snapshot.decisions.is_empty() {
                println!("{}", "── Decisions ──".cyan());
                for d in &snapshot.decisions {
                    println!("  • {d}");
                }
                println!();
            }

            if let Some(ref git) = snapshot.git_state {
                println!("{}", "── Git ──".cyan());
                println!("  Branch: {}", git.branch);
                println!("  {}", git.status_summary);
                if !git.recent_commits.is_empty() {
                    println!("  Recent:");
                    for c in git.recent_commits.iter().take(3) {
                        println!("    {c}");
                    }
                }
                println!();
            }

            if !snapshot.recent_files.is_empty() {
                println!("{}", "── Changed Files ──".cyan());
                for f in snapshot.recent_files.iter().take(10) {
                    println!("  {f}");
                }
                println!();
            }
        }

        Commands::Agents => {
            let statuses = agents::check_all_agents(&config);

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&statuses)?);
                return Ok(());
            }

            println!("{}", "═══ Relay Agents ═══".bold());
            println!();
            println!("Priority order: {}", config.general.priority.join(" → "));
            println!();

            for s in &statuses {
                let icon = if s.available { "✅" } else { "❌" };
                let name = if s.available {
                    s.name.green().bold().to_string()
                } else {
                    s.name.dimmed().to_string()
                };
                println!(
                    "  {icon}  {:<10}  {}",
                    name,
                    s.reason
                );
                if let Some(ref v) = s.version {
                    println!("              Version: {v}");
                }
            }
            println!();

            let available = statuses.iter().filter(|s| s.available).count();
            if available == 0 {
                eprintln!("{}", "⚠️  No agents available. Run 'relay init' to configure.".yellow());
            } else {
                println!(
                    "  {} agent{} ready for handoff.",
                    available,
                    if available == 1 { "" } else { "s" }
                );
            }
        }

        Commands::Init => {
            let path = relay::config_path();
            if path.exists() {
                println!("Config already exists at: {}", path.display());
                println!("Edit it to add API keys and customize agent priority.");
            } else {
                Config::save_default(&path)?;
                println!("{}", "✅ Config created at:".green());
                println!("   {}", path.display());
                println!();
                println!("Edit it to add API keys:");
                println!("  [agents.gemini]");
                println!("  api_key = \"your-gemini-key\"");
                println!();
                println!("  [agents.openai]");
                println!("  api_key = \"your-openai-key\"");
            }
        }

        Commands::Hook { session: _ } => {
            use std::io::Read;
            let mut raw = String::new();
            std::io::stdin().read_to_string(&mut raw)?;

            // Check for rate limit signals
            if let Some(detection) = relay::detect::check_hook_output(&raw) {
                eprintln!(
                    "{}",
                    format!(
                        "🚨 [relay] Rate limit detected in {} output (signal: {})",
                        detection.tool_name, detection.signal
                    ).red().bold()
                );

                if config.general.auto_handoff {
                    // Auto-handoff
                    let snapshot = capture::capture_snapshot(&project_dir, None)?;
                    let handoff_text = handoff::build_handoff(
                        &snapshot,
                        "auto",
                        config.general.max_context_tokens,
                    )?;

                    let handoff_path = handoff::save_handoff(&handoff_text, &project_dir)?;
                    eprintln!(
                        "📄 Handoff saved: {}",
                        handoff_path.display()
                    );

                    let result = agents::handoff_to_first_available(
                        &config,
                        &handoff_text,
                        &project_dir.to_string_lossy(),
                    )?;

                    if result.success {
                        eprintln!(
                            "{}",
                            format!("✅ Auto-handed off to {}", result.agent).green()
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format!("⚠️  No agents available. Handoff saved to: {}",
                                handoff_path.display()
                            ).yellow()
                        );
                    }
                }
            }

            // Always pass through the original output
            print!("{raw}");
        }
    }

    Ok(())
}
