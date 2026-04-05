use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tokio::sync::mpsc;
use zero_core::{AgentEvent, EngineConfig, QueryEngine};
use zero_llm::{MessagesApiProvider, ModelProvider, OpenAiConfig, OpenAiProvider};
use zero_tools::{register_core_tools, ToolRegistry};

#[derive(Parser)]
#[command(name = "zero-code", version, about = "Agent-native coding tool")]
struct Cli {
    /// Initial prompt to send to the agent
    prompt: Option<String>,

    /// LLM provider: flock, openai, messages-api
    #[arg(long, default_value = "flock")]
    provider: String,

    /// Model to use (provider-specific; empty = provider default)
    #[arg(long, default_value = "")]
    model: String,

    /// Maximum tokens per response
    #[arg(long, default_value = "16384")]
    max_tokens: u32,

    /// Launch interactive TUI mode
    #[arg(short, long)]
    interactive: bool,
}

fn build_provider(cli: &Cli) -> Result<Arc<dyn ModelProvider>> {
    match cli.provider.as_str() {
        "flock" => {
            let api_key = std::env::var("FLOCK_API_KEY").or_else(|_| std::env::var("ZERO_CODE_API_KEY"));
            let api_key = api_key.unwrap_or_default();
            if api_key.is_empty() {
                anyhow::bail!(
                    "Flock provider requires FLOCK_API_KEY (or ZERO_CODE_API_KEY).\n  \
                     export FLOCK_API_KEY=your-key"
                );
            }
            let mut p = OpenAiProvider::with_config(api_key, OpenAiConfig::flock());
            if !cli.model.is_empty() {
                p = p.with_model(&cli.model);
            }
            Ok(Arc::new(p))
        }
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY").or_else(|_| std::env::var("ZERO_CODE_API_KEY"));
            let api_key = api_key.unwrap_or_default();
            if api_key.is_empty() {
                anyhow::bail!(
                    "OpenAI provider requires OPENAI_API_KEY (or ZERO_CODE_API_KEY).\n  \
                     export OPENAI_API_KEY=your-key"
                );
            }
            let mut p = OpenAiProvider::new(api_key);
            if !cli.model.is_empty() {
                p = p.with_model(&cli.model);
            }
            Ok(Arc::new(p))
        }
        "messages-api" => {
            let api_key = std::env::var("ZERO_CODE_API_KEY").unwrap_or_default();
            if api_key.is_empty() {
                anyhow::bail!(
                    "Messages API provider requires ZERO_CODE_API_KEY.\n  \
                     export ZERO_CODE_API_KEY=your-key"
                );
            }
            let mut p = MessagesApiProvider::new(api_key);
            if !cli.model.is_empty() {
                p = p.with_model(&cli.model);
            }
            Ok(Arc::new(p))
        }
        other => {
            anyhow::bail!("Unknown provider: {other}. Use: flock, openai, messages-api");
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    if cli.interactive || cli.prompt.is_none() {
        return run_interactive(cli).await;
    }

    run_oneshot(cli).await
}

async fn run_interactive(cli: Cli) -> Result<()> {
    let provider = build_provider(&cli)?;
    let provider_name = provider.name().to_string();

    let mut registry = ToolRegistry::new();
    register_core_tools(&mut registry);
    let tools = Arc::new(registry);

    let model_name = if cli.model.is_empty() {
        provider_name.clone()
    } else {
        cli.model.clone()
    };

    let config = EngineConfig {
        model: cli.model,
        max_tokens: cli.max_tokens,
        ..Default::default()
    };

    let (user_tx, mut user_rx) = mpsc::channel::<String>(32);
    let (agent_tx, agent_rx) = mpsc::channel::<AgentEvent>(256);

    tokio::spawn(async move {
        let mut engine = QueryEngine::new(provider, tools, config);
        while let Some(msg) = user_rx.recv().await {
            let tx = agent_tx.clone();
            if let Err(e) = engine.run(&msg, tx).await {
                let _ = agent_tx.send(AgentEvent::Error(e.to_string())).await;
            }
        }
    });

    zero_tui::run_tui(model_name, agent_rx, user_tx).await?;

    Ok(())
}

async fn run_oneshot(cli: Cli) -> Result<()> {
    println!("zero-code v{}", env!("CARGO_PKG_VERSION"));

    let provider = build_provider(&cli)?;
    let prompt = cli.prompt.expect("prompt required in oneshot mode");

    let mut registry = ToolRegistry::new();
    register_core_tools(&mut registry);
    let tools = Arc::new(registry);

    let config = EngineConfig {
        model: cli.model,
        max_tokens: cli.max_tokens,
        ..Default::default()
    };

    let mut engine = QueryEngine::new(provider, tools, config);
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(256);

    let run_handle = tokio::spawn(async move { engine.run(&prompt, tx).await });

    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::TextDelta(t) => print!("{t}"),
            AgentEvent::ThinkingDelta(_) => {}
            AgentEvent::ToolStart { name, .. } => {
                println!("\n--- tool: {name} ---");
            }
            AgentEvent::ToolEnd {
                result, is_error, ..
            } => {
                if is_error {
                    eprintln!("[error] {result}");
                } else {
                    let preview: String = result.chars().take(200).collect();
                    println!("{preview}");
                }
                println!("--- end tool ---\n");
            }
            AgentEvent::TurnComplete {
                usage_input,
                usage_output,
            } => {
                println!("\n[tokens: {usage_input} in / {usage_output} out]");
            }
            AgentEvent::Error(e) => {
                eprintln!("[engine error] {e}");
            }
        }
    }

    run_handle.await??;
    Ok(())
}
