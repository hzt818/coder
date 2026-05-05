#![deny(unused)]
#![warn(clippy::all, clippy::pedantic)]

use clap::Parser;

/// 🦀 Coder - AI-powered development tool
///
/// Integrates features from Claude Code and OpenCode.
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
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Set up signal handlers for graceful shutdown (Ctrl+C, SIGTERM)
    setup_signal_handler();

    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("coder={}", log_level))
        .init();

    // Initialize core systems
    coder::core::automation::init_automation_manager();

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
        return run_print_mode(&config, &query, &cli).await;
    }

    // Handle headless mode
    if cli.headless {
        return run_headless_mode(&config, &cli).await;
    }

    // Full TUI mode
    run_tui_mode(config, &cli).await
}

/// Set up signal handlers to restore terminal on Ctrl+C / SIGTERM.
#[cfg(unix)]
fn setup_signal_handler() {
    tokio::spawn(async move {
        let mut term_signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to create SIGTERM signal handler");

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl+C, shutting down...");
            }
            _ = term_signal.recv() => {
                tracing::info!("Received SIGTERM, shutting down...");
            }
        }

        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        std::process::exit(130);
    });
}

/// Set up signal handler to restore terminal on Ctrl+C (Windows-compatible).
#[cfg(not(unix))]
fn setup_signal_handler() {
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Received Ctrl+C, shutting down...");
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        std::process::exit(130);
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
    println!("{}", response);
    Ok(())
}

async fn run_headless_mode(
    config: &coder::config::Settings,
    cli: &Cli,
) -> anyhow::Result<()> {
    let provider = create_provider(config, cli)?;
    let tools = coder::tool::ToolRegistry::default();
    let agent = coder::agent::Agent::new(provider, tools);
    agent.run_interactive().await?;
    Ok(())
}

async fn run_tui_mode(
    mut config: coder::config::Settings,
    cli: &Cli,
) -> anyhow::Result<()> {
    // Check if any provider has an API key configured; if not, show setup dialog
    let has_api_key = config.ai.providers.values().any(|p| p.api_key.is_some());
    if !has_api_key {
        use coder::tui::dialog_provider_setup::{run_provider_setup_dialog, ProviderSetupResult};

        let result = run_provider_setup_dialog();

        match result {
            ProviderSetupResult::FreeTier => {
                tracing::info!("User selected OpenCode free tier (anonymous)");
                config.ai.default_provider = "opencode".to_string();
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
                            anyhow::bail!("OAuth failed: {}", e);
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

    // Restore session if --session flag was provided
    if let Some(session_id) = &cli.session {
        let session_manager = coder::session::manager::SessionManager::new();
        match session_manager.load(session_id) {
            Ok(Some(session)) => {
                for msg in &session.messages {
                    agent.context_mut().add_message(msg.clone());
                }
                // Replace the agent's session so auto-save uses the same ID
                *agent.session_mut() = session;
                println!(
                    "Restored session: {} ({} messages)",
                    session_id,
                    agent.session().message_count()
                );
            }
            Ok(None) => {
                println!("Session not found: {}", session_id);
            }
            Err(e) => {
                eprintln!("Failed to load session '{}': {}. Starting new session.", session_id, e);
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
    let working_dir = std::fs::canonicalize(&cli.directory)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| cli.directory.clone());

    let mut terminal = coder::tui::init_terminal()?;
    let mut app = coder::tui::App::new(agent, model_name, provider_name, working_dir);
    let result = coder::tui::ui::run_app(&mut app, &mut terminal, &config.ui).await;
    coder::tui::restore_terminal()?;
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

/// Save OpenCode API key to config file and reload settings.
fn save_opencode_config(config: &mut coder::config::Settings, key: &str) -> anyhow::Result<()> {
    let config_path = coder::util::path::coder_dir().join("config.toml");

    let opencode_config = coder::config::ProviderConfig {
        provider_type: "opencode".to_string(),
        api_key: Some(key.to_string()),
        base_url: Some("https://opencode.ai/zen/v1".to_string()),
        ..Default::default()
    };

    config.ai.providers.insert("opencode".to_string(), opencode_config);
    config.ai.default_provider = "opencode".to_string();

    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let toml_str = toml::to_string(&config)
        .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
    std::fs::write(&config_path, toml_str)
        .map_err(|e| anyhow::anyhow!("Failed to write config: {}", e))?;
    tracing::info!("OpenCode API key saved to {:?}", config_path);

    // Reload from the saved file so all env vars are resolved
    *config = coder::config::Settings::load(Some(config_path.to_str().unwrap()))?;
    Ok(())
}
