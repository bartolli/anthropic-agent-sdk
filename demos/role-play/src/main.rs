//! Claude Role-Play Demo
//!
//! Two Claude agents with different personas engage in conversation,
//! controlled by a Director (you) who can start, stop, and guide the scene.
//!
//! Features:
//! - Load personas from files via --system-prompt-file
//! - Scene settings via --append-system-prompt
//! - Director commands: /start, /stop, /pause, /turns, /quit
//! - Configurable number of conversation turns
//!
//! Run with:
//!   cargo run -p claude-role-play -- \
//!     --persona-a personas/scientist.txt \
//!     --persona-b personas/philosopher.txt \
//!     --scene "Discuss the nature of consciousness" \
//!     --turns 5

mod config;
mod input;
mod output;
mod thinking;

use anthropic_agent_sdk::{
    ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message, PermissionMode, SystemPrompt,
};
use clap::Parser;
use console::style;
use std::path::PathBuf;
use std::time::Instant;

/// Two-agent role-play orchestrator
#[derive(Parser, Debug)]
#[command(name = "claude-role-play")]
#[command(about = "Two Claude agents with different personas engage in conversation")]
struct Args {
    /// Path to persona file for Agent A
    #[arg(long, default_value = "personas/agent_a.txt")]
    persona_a: PathBuf,

    /// Path to persona file for Agent B
    #[arg(long, default_value = "personas/agent_b.txt")]
    persona_b: PathBuf,

    /// Name for Agent A (for display)
    #[arg(long, default_value = "Alice")]
    name_a: String,

    /// Name for Agent B (for display)
    #[arg(long, default_value = "Bob")]
    name_b: String,

    /// Scene description / context to append to both agents
    #[arg(long, short = 's')]
    scene: Option<String>,

    /// Maximum number of conversation turns (exchanges between agents)
    #[arg(long, short = 't', default_value = "5")]
    turns: u32,

    /// Opening line to start the conversation (given to Agent A)
    #[arg(long, short = 'o')]
    opening: Option<String>,
}

/// Get the history file path
fn history_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claude-role-play")
        .join("history.txt")
}

/// Ensure history directory exists
fn ensure_history_dir() -> anyhow::Result<()> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Display the role-play greeting
fn display_roleplay_greeting(args: &Args) {
    println!();
    println!(
        "{}",
        style("╔════════════════════════════════════════════════════════╗").magenta()
    );
    println!(
        "{}",
        style("║            Claude Role-Play Demo                       ║").magenta()
    );
    println!(
        "{}",
        style("╠════════════════════════════════════════════════════════╣").magenta()
    );
    println!(
        "{}",
        style("║  Two agents will converse based on their personas      ║").magenta()
    );
    println!(
        "{}",
        style("║  You are the Director - control the scene!             ║").magenta()
    );
    println!(
        "{}",
        style("╚════════════════════════════════════════════════════════╝").magenta()
    );
    println!();
    println!(
        "  {} {} ({})",
        style("Agent A:").cyan().bold(),
        args.name_a,
        args.persona_a.display()
    );
    println!(
        "  {} {} ({})",
        style("Agent B:").green().bold(),
        args.name_b,
        args.persona_b.display()
    );
    if let Some(ref scene) = args.scene {
        println!("  {} {}", style("Scene:").yellow().bold(), scene);
    }
    println!("  {} {}", style("Turns:").dim(), args.turns);
    println!();
}

/// Display help for Director commands
fn display_director_help() {
    println!();
    println!("{}", style("Director Commands:").bold().magenta());
    println!(
        "  {}  - Start/continue the conversation",
        style("/start").cyan()
    );
    println!("  {}   - Stop the conversation", style("/stop").cyan());
    println!("  {}  - Pause and wait for input", style("/pause").cyan());
    println!(
        "  {} - Set remaining turns (e.g., /turns 3)",
        style("/turns N").cyan()
    );
    println!(
        "  {}  - Send a message as Director to both agents",
        style("/say").cyan()
    );
    println!("  {}   - Show this help", style("/help").cyan());
    println!("  {}   - Exit the demo", style("/quit").cyan());
    println!();
    println!(
        "{}",
        style("Or type a message to interject as Director").dim()
    );
    println!();
}

/// Display agent prefix with color
fn display_agent_prefix(name: &str, is_agent_a: bool) {
    println!();
    if is_agent_a {
        print!("{} ", style(format!("{}:", name)).cyan().bold());
    } else {
        print!("{} ", style(format!("{}:", name)).green().bold());
    }
    std::io::Write::flush(&mut std::io::stdout()).ok();
}

/// Agent turn result containing response text and session_id
struct TurnResult {
    response: Option<String>,
    session_id: Option<String>,
}

/// Run a single agent turn and return the response text and session_id
async fn run_agent_turn(
    client: &mut ClaudeSDKClient,
    prompt: &str,
    name: &str,
    is_agent_a: bool,
) -> anyhow::Result<TurnResult> {
    client.send_message(prompt).await?;

    output::show_thinking();
    let start_time = Instant::now();

    let mut response_text = String::new();
    let mut first_text = true;
    let mut session_id: Option<String> = None;

    while let Some(message) = client.next_message().await {
        match message {
            Ok(msg) => match &msg {
                Message::Assistant { message, .. } => {
                    for block in &message.content {
                        if let ContentBlock::Text { text } = block {
                            output::hide_thinking();

                            if first_text {
                                display_agent_prefix(name, is_agent_a);
                                first_text = false;
                            }

                            output::print_markdown(text);
                            response_text.push_str(text);
                        }
                    }
                }
                Message::Result {
                    session_id: sid, ..
                } => {
                    output::hide_thinking();
                    println!();
                    output::display_elapsed(start_time.elapsed());
                    session_id = Some(sid.to_string());
                    break;
                }
                _ => {}
            },
            Err(e) => {
                output::hide_thinking();
                output::display_error(&format!("Error: {}", e));
                return Err(e.into());
            }
        }
    }

    Ok(TurnResult {
        response: if response_text.is_empty() {
            None
        } else {
            Some(response_text)
        },
        session_id,
    })
}

/// Create an agent with the given persona, scene, and optional session to resume
async fn create_agent(
    persona_path: &PathBuf,
    scene: Option<&str>,
    resume_session: Option<&str>,
) -> anyhow::Result<ClaudeSDKClient> {
    let options = match (scene, resume_session) {
        (Some(scene_text), Some(session_id)) => ClaudeAgentOptions::builder()
            .system_prompt(SystemPrompt::File(persona_path.clone()))
            .append_system_prompt(scene_text.to_string())
            .permission_mode(PermissionMode::BypassPermissions)
            .resume(session_id.to_string())
            .build(),
        (Some(scene_text), None) => ClaudeAgentOptions::builder()
            .system_prompt(SystemPrompt::File(persona_path.clone()))
            .append_system_prompt(scene_text.to_string())
            .permission_mode(PermissionMode::BypassPermissions)
            .build(),
        (None, Some(session_id)) => ClaudeAgentOptions::builder()
            .system_prompt(SystemPrompt::File(persona_path.clone()))
            .permission_mode(PermissionMode::BypassPermissions)
            .resume(session_id.to_string())
            .build(),
        (None, None) => ClaudeAgentOptions::builder()
            .system_prompt(SystemPrompt::File(persona_path.clone()))
            .permission_mode(PermissionMode::BypassPermissions)
            .build(),
    };

    let client = ClaudeSDKClient::new(options, None).await?;
    Ok(client)
}

/// Handle Director commands
async fn handle_director_command(
    cmd: &str,
    remaining_turns: &mut u32,
    running: &mut bool,
) -> anyhow::Result<bool> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let command = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();

    match command.as_str() {
        "/help" => {
            display_director_help();
            Ok(false)
        }
        "/start" | "/continue" => {
            *running = true;
            println!("{}", style("▶ Conversation resumed").green());
            Ok(false)
        }
        "/stop" | "/pause" => {
            *running = false;
            println!("{}", style("⏸ Conversation paused").yellow());
            Ok(false)
        }
        "/turns" => {
            if let Some(n) = parts.get(1).and_then(|s| s.parse::<u32>().ok()) {
                *remaining_turns = n;
                println!("{}", style(format!("Set remaining turns to {}", n)).cyan());
            } else {
                println!(
                    "{}",
                    style(format!("Remaining turns: {}", remaining_turns)).dim()
                );
            }
            Ok(false)
        }
        "/quit" | "/exit" => Ok(true),
        _ => {
            output::display_warning(&format!("Unknown command: {}", cmd));
            display_director_help();
            Ok(false)
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Initialize tracing
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "claude_role_play=info,warn".parse().unwrap());
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Display greeting
    display_roleplay_greeting(&args);
    display_director_help();

    // Check persona files exist
    if !args.persona_a.exists() {
        output::display_error(&format!(
            "Persona file not found: {}",
            args.persona_a.display()
        ));
        println!();
        println!("{}", style("Create persona files first. Example:").dim());
        println!("{}", style("  mkdir -p personas").dim());
        println!(
            "{}",
            style("  echo 'You are a curious scientist.' > personas/agent_a.txt").dim()
        );
        println!(
            "{}",
            style("  echo 'You are a wise philosopher.' > personas/agent_b.txt").dim()
        );
        return Ok(());
    }
    if !args.persona_b.exists() {
        output::display_error(&format!(
            "Persona file not found: {}",
            args.persona_b.display()
        ));
        return Ok(());
    }

    // Create input editor
    let mut editor = input::create_editor()?;
    let _ = ensure_history_dir();
    let _ = input::load_history(&mut editor, &history_path());

    // State
    let mut remaining_turns = args.turns;
    let mut running = false;
    let mut last_response: Option<String> = args.opening.clone();

    // Track which agent speaks next (A starts)
    let mut agent_a_turn = true;

    // Session IDs for conversation continuity - each agent maintains its own context
    let mut session_id_a: Option<String> = None;
    let mut session_id_b: Option<String> = None;

    println!(
        "{}",
        style("Type /start to begin, or /help for commands").dim()
    );
    println!();

    // Main loop
    loop {
        // If running and turns remain, continue conversation
        if running && remaining_turns > 0 {
            // Create agent for this turn - use saved session_id for continuity
            let (persona_path, name, is_agent_a, session_id) = if agent_a_turn {
                (&args.persona_a, &args.name_a, true, session_id_a.as_deref())
            } else {
                (
                    &args.persona_b,
                    &args.name_b,
                    false,
                    session_id_b.as_deref(),
                )
            };

            // Build the prompt for this agent - pass other agent's response as "user message"
            let prompt = if let Some(ref prev) = last_response {
                // The other agent's response becomes our input
                let other_name = if agent_a_turn {
                    &args.name_b
                } else {
                    &args.name_a
                };
                format!(
                    "{} said: \"{}\"\n\nRespond naturally, staying in character.",
                    other_name, prev
                )
            } else {
                // Opening - first message
                "Start the conversation. Introduce yourself and begin the dialogue.".to_string()
            };

            // Create and run agent - resume session if we have one
            println!(
                "\n{}",
                style(format!(
                    "── Turn {} of {} ──",
                    args.turns - remaining_turns + 1,
                    args.turns
                ))
                .dim()
            );

            match create_agent(persona_path, args.scene.as_deref(), session_id).await {
                Ok(mut client) => {
                    match run_agent_turn(&mut client, &prompt, name, is_agent_a).await {
                        Ok(result) => {
                            // Save session_id for next turn with this agent
                            if let Some(sid) = result.session_id {
                                if agent_a_turn {
                                    session_id_a = Some(sid);
                                } else {
                                    session_id_b = Some(sid);
                                }
                            }

                            if let Some(response) = result.response {
                                last_response = Some(response);
                                remaining_turns -= 1;
                                agent_a_turn = !agent_a_turn; // Switch turns
                            } else {
                                output::display_warning("Agent returned no response");
                            }
                        }
                        Err(e) => {
                            output::display_error(&format!("Agent error: {}", e));
                        }
                    }
                    client.close().await.ok();
                }
                Err(e) => {
                    output::display_error(&format!("Failed to create agent: {}", e));
                    running = false;
                }
            }

            // Check if conversation ended
            if remaining_turns == 0 {
                running = false;
                println!();
                println!(
                    "{}",
                    style("═══ Conversation Complete ═══").magenta().bold()
                );
                println!(
                    "{}",
                    style("Type /start to continue, or /quit to exit").dim()
                );
            } else {
                // Auto-continue to next turn
                continue;
            }
        }

        // Get Director input (only when paused or conversation ended)
        match input::get_input(&mut editor)? {
            input::InputResult::Command(cmd) => {
                if handle_director_command(&cmd, &mut remaining_turns, &mut running).await? {
                    break;
                }
            }
            input::InputResult::Message(msg) => {
                if msg.is_empty() {
                    continue;
                }
                // Director interjection - inject into conversation
                println!(
                    "\n{} {}",
                    style("[Director]:").magenta().bold(),
                    style(&msg).magenta()
                );
                last_response = Some(format!("[Director interjects]: {}", msg));
            }
            input::InputResult::Exit => break,
            input::InputResult::Interrupt => {
                running = false;
                println!();
                println!("{}", style("⏸ Interrupted - conversation paused").yellow());
            }
        }
    }

    // Save history
    let _ = input::save_history(&mut editor, &history_path());

    output::display_goodbye();
    Ok(())
}
