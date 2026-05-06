#![deny(unused)]
#![warn(clippy::all, clippy::pedantic)]

use clap::Parser;
use std::sync::atomic::Ordering;

use coder::shutdown_notifier;
use coder::SHUTDOWN_REQUESTED;

/// 🦀 Coder - AI-powered development tool
///
/// Integrates features from Claude Code and `OpenCode`.
/// Chat with AI, execute tools, manage teams, and more.
#[derive(Parser, Debug)]
#[command(name = "coder", version, about, long_about = None)]
struct Cli {
    /// AI provider to use
    #[arg(long, env = "CODER_PROVIDER")]
    provider: Option<String>,

    /// Model name override
    #[arg(long, env = "CODER_MODEL")]
    model: Option<String>,

    /// Config file path
    #[arg(long, short = 'c', env = "CODER_CONFIG")]
    config: Option<String>,

    /// Session ID to resume
    #[arg(long, short = 's')]
    session: Option<String>,

    /// Run in headless mode (no TUI)
    #[arg(long)]
    headless: bool,

    /// Print mode: one-shot query then exit
    #[arg(long)]
    print: Option<String>,

    /// Directory to work in
    #[arg(long, short = 'd', default_value = ".")]
    directory: String,

    /// Enable verbose logging
    #[arg(long, short = 'v')]
    verbose: bool,

    /// Start the HTTP server (requires 'server' feature)
    #[arg(long)]
    serve: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Set up signal handlers for graceful shutdown (Ctrl+C, SIGTERM).
    // Instead of calling process::exit() directly (which races with TUI rendering),
    // they set a flag and notify the main loop to exit cleanly.
    setup_signal_handler();

    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("coder={log_level}"))
        .init();

    // Initialize core systems
    coder::core::automation::init_automation_manager();
    coder::core::audit::AuditLogger::init(None);
    tracing::info!("Audit logger initialized");

    // Load config
    let config_path = cli.config.clone();
    let config = coder::config::Settings::load(config_path.as_deref())?;

    // If --serve flag is set, start the HTTP server
    if cli.serve {
        #[cfg(feature = "server")]
        {
            let provider = create_provider(&config, &cli)?;
            let tools = std::sync::Arc::new(coder::tool::ToolRegistry::default());
            let state = std::sync::Arc::new(coder::server::AppState::new(
                coder::session::manager::SessionManager::new(),
                tools,
                provider,
            ));
            let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3000).into();
            coder::server::serve(&addr, state).await?;
            return Ok(());
        }
        #[cfg(not(feature = "server"))]
        {
            anyhow::bail!("Server feature not enabled. Build with --features server");
        }
    }

    // Handle --print mode (one-shot)
    if let Some(ref query) = cli.print {
        return run_print_mode(&config, query, &cli).await;
    }

    // Handle headless mode
    if cli.headless {
        return run_headless_mode(&config, &cli).await;
    }

    // Full TUI mode — pass the shutdown notifier so the event loop can
    // break out when SIGTERM/SIGINT arrives
    run_tui_mode(config, &cli).await
}

/// Set up signal handlers that set a global flag + notify the main loop.
/// They do NOT call process::exit() directly to avoid racing with TUI rendering.
#[cfg(unix)]
fn setup_signal_handler() {
    let notify = shutdown_notifier();
    tokio::spawn(async move {
        let mut term_signal =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to create SIGTERM signal handler");

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl+C, shutting down gracefully...");
            }
            _ = term_signal.recv() => {
                tracing::info!("Received SIGTERM, shutting down gracefully...");
            }
        }

        SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
        notify.notify_waiters();
    });
}

/// Windows-compatible signal handler.
#[cfg(not(unix))]
fn setup_signal_handler() {
    let notify = shutdown_notifier();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Received Ctrl+C, shutting down gracefully...");
        SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
        notify.notify_waiters();
    });
}

async fn run_print_mode(
    config: &coder::config::Settings,
    query: &str,
    cli: &Cli,
) -> anyhow::Result<()> {
    let provider = create_provider(config, cli)?;
    let tools = coder::tool::ToolRegistry::default();
    let agent = coder::agent::Agent::new(provider, tools);
    let response = agent.run_simple(query).await?;
    println!("{response}");
    Ok(())
}

async fn run_headless_mode(config: &coder::config::Settings, cli: &Cli) -> anyhow::Result<()> {
    let provider = create_provider(config, cli)?;
    let tools = coder::tool::ToolRegistry::default();
    let agent = coder::agent::Agent::new(provider, tools);
    agent.run_interactive().await?;
    Ok(())
}

async fn run_tui_mode(mut config: coder::config::Settings, cli: &Cli) -> anyhow::Result<()> {
    // Check if any provider has an API key configured; if not, show setup dialog
    let has_api_key = config.ai.providers.values().any(|p| p.api_key.is_some());
    if !has_api_key {
        use coder::tui::dialog_provider_setup::{run_provider_setup_dialog, ProviderSetupResult};

        let result = run_provider_setup_dialog();

        match result {
            ProviderSetupResult::FreeTier => {
                tracing::info!("User selected OpenCode free tier (anonymous)");
                save_opencode_config(&mut config, "")?;
            }
            ProviderSetupResult::OAuth => {
                tracing::info!("User selected OAuth flow");
                #[cfg(feature = "ai-opencode")]
                {
                    match coder::oauth::opencode::run_oauth_flow().await {
                        coder::oauth::opencode::OAuthResult::Success(key) => {
                            save_opencode_config(&mut config, &key)?;
                        }
                        coder::oauth::opencode::OAuthResult::Cancelled => {
                            anyhow::bail!("OAuth cancelled.");
                        }
                        coder::oauth::opencode::OAuthResult::Error(e) => {
                            anyhow::bail!("OAuth failed: {e}");
                        }
                    }
                }
                #[cfg(not(feature = "ai-opencode"))]
                anyhow::bail!("OAuth requires 'ai-opencode' feature");
            }
            ProviderSetupResult::ManualKey(key) => {
                tracing::info!("User entered API key manually");
                save_opencode_config(&mut config, &key)?;
            }
            ProviderSetupResult::Skipped | ProviderSetupResult::Quit => {
                anyhow::bail!("No AI provider configured. Run with --help for options.");
            }
        }
    }

    let provider = create_provider(&config, cli)?;
    let tools = coder::tool::ToolRegistry::default();
    let mut agent = coder::agent::Agent::new(provider, tools);
    let shutdown = shutdown_notifier();

    // Restore session if --session flag was provided
    if let Some(session_id) = &cli.session {
        let session_manager = coder::session::manager::SessionManager::new();
        match session_manager.load(session_id) {
            Ok(Some(session)) => {
                for msg in &session.messages {
                    agent.context_mut().add_message(msg.clone());
                }
                *agent.session_mut() = session;
                println!(
                    "Restored session: {} ({} messages)",
                    session_id,
                    agent.session().message_count()
                );
            }
            Ok(None) => {
                println!("Session not found: {session_id}");
            }
            Err(e) => {
                eprintln!(
                    "Failed to load session '{session_id}': {e}. Starting new session."
                );
            }
        }
    }

    // Extract model & provider info for the welcome screen
    let provider_name = cli
        .provider
        .clone()
        .unwrap_or_else(|| config.ai.default_provider.clone());
    let model_name = cli
        .model
        .clone()
        .or_else(|| {
            config
                .ai
                .providers
                .get(&provider_name)
                .and_then(|p| p.model.clone())
        })
        .unwrap_or_else(|| "unknown".to_string());
    let working_dir = std::fs::canonicalize(&cli.directory).map_or_else(|_| cli.directory.clone(), |p| p.display().to_string());

    let mut terminal = coder::tui::init_terminal()?;
    let mut app = coder::tui::App::new(agent, model_name, provider_name, working_dir);
    let result = coder::tui::ui::run_app(&mut app, &mut terminal, &config.ui, shutdown).await;
    coder::tui::restore_terminal()?;

    // If shutdown was requested, use exit code 130 (same as SIGINT convention)
    if SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
        tracing::info!("Exiting due to shutdown signal (exit code 130)");
        std::process::exit(130);
    }

    result
}

fn create_provider(
    config: &coder::config::Settings,
    cli: &Cli,
) -> anyhow::Result<Box<dyn coder::ai::Provider>> {
    let provider_name = cli
        .provider
        .clone()
        .unwrap_or_else(|| config.ai.default_provider.clone());

    let provider_config = config
        .ai
        .providers
        .get(&provider_name)
        .cloned()
        .unwrap_or_default();

    let model_override = cli.model.clone();

    coder::ai::create_provider(&provider_name, provider_config, model_override)
}

/// Save `OpenCode` API key to config file and reload settings.
fn save_opencode_config(config: &mut coder::config::Settings, key: &str) -> anyhow::Result<()> {
    let config_path = coder::util::path::coder_dir().join("config.toml");

    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let existing_content = if config_path.exists() {
        std::fs::read_to_string(&config_path).unwrap_or_default()
    } else {
        String::new()
    };

    let mut root: toml::Value = existing_content
        .parse()
        .unwrap_or(toml::Value::Table(toml::value::Table::new()));

    {
        let ai_table = root
            .as_table_mut()
            .unwrap()
            .entry("ai")
            .or_insert_with(|| toml::Value::Table(toml::value::Table::new()))
            .as_table_mut()
            .unwrap();
        ai_table.insert(
            "default_provider".to_string(),
            toml::Value::String("opencode".to_string()),
        );

        let providers = ai_table
            .entry("providers")
            .or_insert_with(|| toml::Value::Table(toml::value::Table::new()))
            .as_table_mut()
            .unwrap();
        providers.insert(
            "opencode".to_string(),
            toml::Value::Table({
                let mut p = toml::value::Table::new();
                p.insert(
                    "provider_type".to_string(),
                    toml::Value::String("opencode".to_string()),
                );
                p.insert("api_key".to_string(), toml::Value::String(key.to_string()));
                p.insert(
                    "base_url".to_string(),
                    toml::Value::String("https://opencode.ai/zen/v1".to_string()),
                );
                p
            }),
        );
    }

    let serialized = toml::to_string_pretty(&root)
        .map_err(|e| anyhow::anyhow!("Failed to serialize config: {e}"))?;
    std::fs::write(&config_path, serialized)
        .map_err(|e| anyhow::anyhow!("Failed to write config: {e}"))?;
    tracing::info!("OpenCode config saved to {:?}", config_path);

    *config = coder::config::Settings::load(Some(config_path.to_str().unwrap()))?;
    Ok(())
}
