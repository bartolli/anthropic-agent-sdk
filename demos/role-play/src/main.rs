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
    ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message, OutputFormat, PermissionMode,
    SettingSource, SystemPrompt, ToolName,
};
use clap::Parser;
use console::style;
use std::path::{Path, PathBuf};
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
        "  {} - Start/continue (optional: /start N for N more turns)",
        style("/start [N]").cyan()
    );
    println!("  {}    - Pause the scene", style("/stop").cyan());
    println!();
    println!("{}", style("Mode:").bold().magenta());
    println!(
        "  {}   - Human-in-the-loop (pause after each turn)",
        style("/hitl").cyan()
    );
    println!(
        "  {}   - Auto mode (run turns continuously)",
        style("/auto").cyan()
    );
    println!();
    println!("{}", style("Scene Control:").bold().magenta());
    println!("  {} - Set remaining turns", style("/turns N").cyan());
    println!(
        "  {} - Off-camera note to agent",
        style("/say <agent> \"msg\"").cyan()
    );
    println!();
    println!("{}", style("State Overrides (HITL):").bold().magenta());
    println!("  {} - Set tension (1-10)", style("/tension N").cyan());
    println!("  {}    - Set heat (1-5)", style("/heat N").cyan());
    println!("  {}  - Set narrative beat", style("/beat X").cyan());
    println!("  {}  - Show current state", style("/status").cyan());
    println!();
    println!("  {}   - Show this help", style("/help").cyan());
    println!("  {}   - Exit the demo", style("/quit").cyan());
    println!();
    println!(
        "{}",
        style("In HITL mode: Press Enter to advance, or override Haiku's analysis").dim()
    );
    println!();
}

/// Display agent prefix with color
fn display_agent_prefix(name: &str, is_agent_a: bool) {
    println!();
    if is_agent_a {
        print!("{} ", style(format!("{name}:")).cyan().bold());
    } else {
        print!("{} ", style(format!("{name}:")).green().bold());
    }
    std::io::Write::flush(&mut std::io::stdout()).ok();
}

/// Agent turn result containing response text and `session_id`
struct TurnResult {
    response: Option<String>,
    session_id: Option<String>,
}

/// Scene state for the analyzer
#[derive(Debug, Clone)]
struct SceneState {
    tension: u32,
    heat: u32,
    beat: String,
}

// AnalysisResult is in output.rs
use output::AnalysisResult;

/// Read current scene state from files
fn read_scene_state(demo_root: &Path) -> SceneState {
    let state_dir = demo_root.join(".claude").join("scene-state");

    let tension = std::fs::read_to_string(state_dir.join("meters").join("tension.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(5);

    let heat = std::fs::read_to_string(state_dir.join("meters").join("heat.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(1);

    let beat = std::fs::read_to_string(state_dir.join("beat.txt"))
        .ok().map_or_else(|| "exposition".to_string(), |s| s.trim().to_string());

    SceneState {
        tension,
        heat,
        beat,
    }
}

/// Read director note for a specific agent (before hook consumes it)
/// Returns the note content if one exists, None otherwise
fn read_director_note(demo_root: &Path, agent_name: &str) -> Option<String> {
    let notes_dir = demo_root.join(".claude").join("scene-state").join("notes");

    if !notes_dir.exists() {
        return None;
    }

    // Match note file by agent name (same logic as hook)
    let agent_lower = agent_name.to_lowercase().replace(' ', "_");

    if let Ok(entries) = std::fs::read_dir(&notes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "txt") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let note_name = stem.to_lowercase();
                    // Match: exact, agent contains note, or note contains agent
                    if agent_lower == note_name
                        || agent_lower.contains(&note_name)
                        || note_name.contains(&agent_lower)
                    {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let trimmed = content.trim();
                            if !trimmed.is_empty() {
                                return Some(trimmed.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Spawn Haiku analyzer to evaluate scene dynamics
///
/// Runs after each agent turn to semantically analyze dialogue and update state.
/// Uses structured output for guaranteed valid JSON, then writes files ourselves.
/// Returns the analysis result and `session_id` for continuity.
#[allow(clippy::too_many_lines)] // Complex analyzer with structured output handling
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // tension (1-10) and heat (1-5) are bounded by schema
async fn spawn_analyzer(
    demo_root: &Path,
    last_dialogue: &str,
    agent_name: &str,
    before_state: &SceneState,
    resume_session: Option<&str>,
    director_note: Option<&str>,
) -> anyhow::Result<(Option<AnalysisResult>, Option<String>)> {
    let analyzer_persona = demo_root.join("personas").join("analyzer.txt");

    if !analyzer_persona.exists() {
        tracing::warn!("Analyzer persona not found, skipping analysis");
        return Ok((None, None));
    }

    // Define JSON schema for structured output - guarantees valid JSON from Haiku
    let analysis_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "tension": {
                "type": "object",
                "properties": {
                    "to": { "type": "integer", "minimum": 1, "maximum": 10 },
                    "reason": { "type": "string" }
                },
                "required": ["to", "reason"]
            },
            "heat": {
                "type": "object",
                "properties": {
                    "to": { "type": "integer", "minimum": 1, "maximum": 5 },
                    "reason": { "type": "string" }
                },
                "required": ["to", "reason"]
            },
            "beat": {
                "type": "object",
                "properties": {
                    "changed": { "type": "boolean" },
                    "current": { "type": "string", "enum": ["exposition", "confrontation", "revelation", "intimate-moment", "climax", "resolution"] }
                },
                "required": ["changed", "current"]
            },
            "director_aligned": { "type": "boolean" }
        },
        "required": ["tension", "heat", "beat", "director_aligned"]
    });

    // Build context with current state, dialogue, and optional director note
    let director_section = if let Some(note) = director_note {
        format!(
            r"
## Director's Note (given to {agent_name} before speaking)
{note}

Evaluate whether the agent's dialogue aligns with this direction.
"
        )
    } else {
        String::new()
    };

    let context = format!(
        r"
## Current Scene State
- Tension: {}/10
- Heat: {}/5
- Beat: {}
{director_section}
## Last Dialogue (by {agent_name})
{last_dialogue}

Analyze this dialogue and return your analysis as JSON.
",
        before_state.tension, before_state.heat, before_state.beat
    );

    // Create Haiku agent with structured output
    let mut env = std::collections::HashMap::new();
    env.insert(
        "CLAUDE_AGENT_NAME".to_string(),
        "haiku_analyzer".to_string(),
    );

    let output_format = OutputFormat::json_schema(analysis_schema);
    tracing::debug!(
        "Haiku output_format schema: {}",
        serde_json::to_string(&output_format.schema).unwrap_or_default()
    );

    // Build options - resume session if we have one for continuity
    let options = match resume_session {
        Some(session_id) => ClaudeAgentOptions::builder()
            .model("haiku".to_string())
            .cwd(demo_root.to_path_buf())
            .setting_sources(vec![SettingSource::Local])
            .system_prompt(SystemPrompt::File(analyzer_persona))
            .append_system_prompt(context)
            .permission_mode(PermissionMode::BypassPermissions)
            .allowed_tools(vec![ToolName::new("Read")])
            .output_format(output_format)
            .max_turns(15_u32)
            .resume(session_id.to_string())
            .env(env)
            .stderr(std::sync::Arc::new(|line| {
                tracing::warn!("[Haiku stderr] {}", line);
            }))
            .build(),
        None => ClaudeAgentOptions::builder()
            .model("haiku".to_string())
            .cwd(demo_root.to_path_buf())
            .setting_sources(vec![SettingSource::Local])
            .system_prompt(SystemPrompt::File(analyzer_persona))
            .append_system_prompt(context)
            .permission_mode(PermissionMode::BypassPermissions)
            .allowed_tools(vec![ToolName::new("Read")])
            .output_format(output_format)
            .max_turns(15_u32)
            .env(env)
            .stderr(std::sync::Arc::new(|line| {
                tracing::warn!("[Haiku stderr] {}", line);
            }))
            .build(),
    };

    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Send analysis request
    tracing::debug!("Sending analysis request to Haiku");
    client
        .send_message("Analyze the dialogue and return your analysis.")
        .await?;

    // Wait for Result with structured_output
    let mut structured_output: Option<serde_json::Value> = None;
    let mut message_count = 0;
    let mut had_error = false;

    while let Some(message) = client.next_message().await {
        message_count += 1;
        match message {
            Ok(Message::Assistant { message, .. }) => {
                for block in &message.content {
                    if let ContentBlock::Text { text } = block {
                        tracing::info!(
                            "[Haiku] Text: {}",
                            text.chars().take(200).collect::<String>()
                        );
                    }
                    if let ContentBlock::ToolUse { name, input, .. } = block {
                        tracing::info!("[Haiku] Tool: {} input: {}", name, input);
                    }
                }
            }
            Ok(Message::Result {
                structured_output: so,
                ..
            }) => {
                tracing::trace!("[Haiku] Complete - structured_output: {:?}", so);
                structured_output = so;
                break;
            }
            Ok(Message::System { subtype, data }) => {
                tracing::trace!("[Haiku] System {}: {:?}", subtype, data);
            }
            Ok(other) => {
                tracing::trace!("[Haiku] Other message: {:?}", other);
            }
            Err(e) => {
                tracing::warn!("[Haiku] Error: {}", e);
                had_error = true;
                break;
            }
        }
    }

    tracing::trace!(
        "[Haiku] Session ended - messages: {}, error: {}, structured_output: {}",
        message_count,
        had_error,
        structured_output.is_some()
    );

    // Capture session_id before closing (SDK tracks it automatically)
    let new_session_id = client.get_session_id().map(|s| s.to_string());
    client.close().await.ok();

    // Parse structured output and write files
    if let Some(analysis) = structured_output {
        tracing::info!("[Haiku] Structured output: {}", analysis);

        // Extract values from the guaranteed-valid JSON
        let tension_to = analysis["tension"]["to"]
            .as_i64()
            .unwrap_or(i64::from(before_state.tension)) as u32;
        let tension_reason = analysis["tension"]["reason"].as_str().unwrap_or("");
        let heat_to = analysis["heat"]["to"]
            .as_i64()
            .unwrap_or(i64::from(before_state.heat)) as u32;
        let heat_reason = analysis["heat"]["reason"].as_str().unwrap_or("");
        let beat_changed = analysis["beat"]["changed"].as_bool().unwrap_or(false);
        let beat_current = analysis["beat"]["current"]
            .as_str()
            .unwrap_or(&before_state.beat);
        let director_aligned = analysis["director_aligned"].as_bool().unwrap_or(true);

        // Write state files ourselves (reliable, no tool use needed)
        let state_dir = demo_root.join(".claude").join("scene-state");
        let meters_dir = state_dir.join("meters");
        std::fs::create_dir_all(&meters_dir)?;

        // Write tension
        std::fs::write(meters_dir.join("tension.txt"), tension_to.to_string())?;

        // Write heat
        std::fs::write(meters_dir.join("heat.txt"), heat_to.to_string())?;

        // Write beat if changed
        if beat_changed {
            std::fs::write(state_dir.join("beat.txt"), beat_current)?;
        }

        // Write full analysis JSON
        let full_analysis = serde_json::json!({
            "tension": { "from": before_state.tension, "to": tension_to, "reason": tension_reason },
            "heat": { "from": before_state.heat, "to": heat_to, "reason": heat_reason },
            "beat": { "changed": beat_changed, "current": beat_current },
            "director_aligned": director_aligned
        });
        std::fs::write(
            state_dir.join("analysis.json"),
            serde_json::to_string_pretty(&full_analysis)?,
        )?;

        return Ok((
            Some(AnalysisResult {
                tension_from: before_state.tension,
                tension_to,
                heat_from: before_state.heat,
                heat_to,
                beat: beat_current.to_string(),
                beat_changed,
            }),
            new_session_id,
        ));
    }

    // Fallback: no structured output, return session_id anyway for next attempt
    tracing::warn!("[Haiku] No structured output received");
    Ok((None, new_session_id))
}

/// Run a single agent turn and return the response text and `session_id`
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
                Message::Result { .. } => {
                    output::hide_thinking();
                    println!();
                    output::display_elapsed(start_time.elapsed());
                    break;
                }
                _ => {}
            },
            Err(e) => {
                output::hide_thinking();
                output::display_error(&format!("Error: {e}"));
                return Err(e.into());
            }
        }
    }

    // Use SDK's session tracking (cleaner than extracting from Result)
    Ok(TurnResult {
        response: if response_text.is_empty() {
            None
        } else {
            Some(response_text)
        },
        session_id: client.get_session_id().map(|s| s.to_string()),
    })
}

/// Create an agent with the given persona, scene, and optional session to resume
///
/// Uses the demo's .claude/ directory for hooks and settings.
/// `CLAUDE_PROJECT_DIR` will be set to the demo root, enabling scene hooks.
/// `CLAUDE_AGENT_NAME` is set so hooks can deliver agent-specific director notes.
async fn create_agent(
    persona_path: &Path,
    scene: Option<&str>,
    resume_session: Option<&str>,
    agent_name: &str,
) -> anyhow::Result<ClaudeSDKClient> {
    // Demo root directory (where .claude/ lives)
    // CARGO_MANIFEST_DIR is set at compile time to the directory containing Cargo.toml
    let demo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Environment variable to identify this agent (for agent-specific director notes)
    let mut env = std::collections::HashMap::new();
    env.insert(
        "CLAUDE_AGENT_NAME".to_string(),
        agent_name.to_lowercase().replace(' ', "_"),
    );

    // Build options with cwd and setting_sources to enable scene hooks
    // typed-builder requires full chain, so we use match for optional fields
    let options = match (scene, resume_session) {
        (Some(scene_text), Some(session_id)) => ClaudeAgentOptions::builder()
            .cwd(demo_root.clone())
            .setting_sources(vec![SettingSource::Local])
            .system_prompt(SystemPrompt::File(persona_path.to_path_buf()))
            .append_system_prompt(scene_text.to_string())
            .permission_mode(PermissionMode::BypassPermissions)
            .resume(session_id.to_string())
            .env(env)
            .build(),
        (Some(scene_text), None) => ClaudeAgentOptions::builder()
            .cwd(demo_root.clone())
            .setting_sources(vec![SettingSource::Local])
            .system_prompt(SystemPrompt::File(persona_path.to_path_buf()))
            .append_system_prompt(scene_text.to_string())
            .permission_mode(PermissionMode::BypassPermissions)
            .env(env)
            .build(),
        (None, Some(session_id)) => ClaudeAgentOptions::builder()
            .cwd(demo_root.clone())
            .setting_sources(vec![SettingSource::Local])
            .system_prompt(SystemPrompt::File(persona_path.to_path_buf()))
            .permission_mode(PermissionMode::BypassPermissions)
            .resume(session_id.to_string())
            .env(env)
            .build(),
        (None, None) => ClaudeAgentOptions::builder()
            .cwd(demo_root)
            .setting_sources(vec![SettingSource::Local])
            .system_prompt(SystemPrompt::File(persona_path.to_path_buf()))
            .permission_mode(PermissionMode::BypassPermissions)
            .env(env)
            .build(),
    };

    let client = ClaudeSDKClient::new(options, None).await?;
    Ok(client)
}

/// Handle Director commands
#[allow(clippy::too_many_lines)] // Complex command handler with clear structure
fn handle_director_command(
    cmd: &str,
    remaining_turns: &mut u32,
    running: &mut bool,
    hitl_mode: &mut bool,
    demo_root: &Path,
) -> anyhow::Result<bool> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let command = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();

    match command.as_str() {
        "/help" => {
            display_director_help();
            Ok(false)
        }
        "/start" | "/continue" | "/go" => {
            // Accept optional turns argument: /start 5
            if let Some(n) = parts.get(1).and_then(|s| s.parse::<u32>().ok()) {
                *remaining_turns = n;
                println!("{}", style(format!("Added {n} turns")).cyan());
            }

            // Check if we have turns to run
            if *remaining_turns == 0 {
                println!(
                    "{}",
                    style("No turns remaining. Use /start N to add turns (e.g., /start 5)")
                        .yellow()
                );
                return Ok(false);
            }

            *running = true;
            if *hitl_mode {
                println!(
                    "{}",
                    style(format!(
                        "▶ Scene started (HITL mode) - {remaining_turns} turns remaining"
                    ))
                    .green()
                );
            } else {
                println!(
                    "{}",
                    style(format!(
                        "▶ Conversation resumed (auto mode) - {remaining_turns} turns remaining"
                    ))
                    .green()
                );
            }
            Ok(false)
        }
        "/stop" | "/pause" => {
            *running = false;
            println!("{}", style("⏸ Conversation paused").yellow());
            Ok(false)
        }
        "/hitl" => {
            *hitl_mode = true;
            println!(
                "{}",
                style("🎬 HITL mode: pause after each turn for director input").cyan()
            );
            println!(
                "{}",
                style("   Press Enter to advance, or /say <agent> \"note\"").dim()
            );
            Ok(false)
        }
        "/auto" => {
            *hitl_mode = false;
            println!("{}", style("▶ Auto mode: run turns continuously").cyan());
            Ok(false)
        }
        "/turns" => {
            if let Some(n) = parts.get(1).and_then(|s| s.parse::<u32>().ok()) {
                *remaining_turns = n;
                println!("{}", style(format!("Set remaining turns to {n}")).cyan());
            } else {
                println!(
                    "{}",
                    style(format!("Remaining turns: {remaining_turns}")).dim()
                );
            }
            Ok(false)
        }
        "/say" | "/note" => {
            // Director's note - off-camera direction for specific agent
            // Syntax: /say <agent|all> "message"
            // Written to file, injected by UserPromptSubmit hook on next turn

            // Parse: /say <target> "quoted message"
            let rest = cmd
                .trim_start_matches("/say")
                .trim_start_matches("/note")
                .trim();

            // Extract target (first word) and message (quoted string)
            let (target, note) = if let Some(space_idx) = rest.find(' ') {
                let target = rest[..space_idx].to_lowercase();
                let msg_part = rest[space_idx..].trim();

                // Extract quoted message or use rest as-is
                let note = if let Some(stripped) =
                    msg_part.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                {
                    // Both quotes present
                    stripped.to_string()
                } else if let Some(stripped) = msg_part.strip_prefix('"') {
                    // Find closing quote
                    if let Some(end) = stripped.find('"') {
                        stripped[..end].to_string()
                    } else {
                        stripped.to_string()
                    }
                } else {
                    msg_part.to_string()
                };

                (target, note)
            } else {
                (String::new(), String::new())
            };

            if !target.is_empty() && !note.is_empty() {
                let notes_dir = demo_root.join(".claude").join("scene-state").join("notes");
                std::fs::create_dir_all(&notes_dir)?;

                if target == "all" {
                    // Write to both agents
                    let note_a = notes_dir.join("agent_a.txt");
                    let note_b = notes_dir.join("agent_b.txt");
                    std::fs::write(&note_a, &note)?;
                    std::fs::write(&note_b, &note)?;
                    println!(
                        "\n{} {}",
                        style("📢 Director's note to ALL (next turn):")
                            .magenta()
                            .bold(),
                        style(&note).magenta().dim()
                    );
                } else {
                    // Write to specific agent (normalize name to filename)
                    let agent_file = format!("{}.txt", target.replace(' ', "_"));
                    let note_path = notes_dir.join(&agent_file);
                    std::fs::write(&note_path, &note)?;
                    println!(
                        "\n{} {}",
                        style(format!(
                            "📢 Director's note queued for {} (delivered when they speak):",
                            target.to_uppercase()
                        ))
                        .magenta()
                        .bold(),
                        style(&note).magenta().dim()
                    );
                }
            } else {
                println!("{}", style("Usage: /say <agent|all> \"message\"").dim());
                println!("{}", style("  /say luna \"Show more vulnerability\"").dim());
                println!(
                    "{}",
                    style("  /say rourke \"Press harder on the alibi\"").dim()
                );
                println!(
                    "{}",
                    style("  /say all \"Increase tension, turning point\"").dim()
                );
            }
            Ok(false)
        }
        "/tension" => {
            // Override tension meter: /tension 7
            let state_dir = demo_root.join(".claude").join("scene-state");
            if let Some(n) = parts.get(1).and_then(|s| s.parse::<u32>().ok()) {
                let clamped = n.clamp(1, 10);
                let meters_dir = state_dir.join("meters");
                std::fs::create_dir_all(&meters_dir)?;
                std::fs::write(meters_dir.join("tension.txt"), clamped.to_string())?;
                println!(
                    "{}",
                    style(format!("⚡ Tension set to {clamped}/10")).yellow()
                );
            } else {
                // Show current value
                let current = std::fs::read_to_string(state_dir.join("meters").join("tension.txt"))
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(5);
                println!("{}", style(format!("Tension: {current}/10")).dim());
                println!("{}", style("Usage: /tension N (1-10)").dim());
            }
            Ok(false)
        }
        "/heat" => {
            // Override heat meter: /heat 3
            let state_dir = demo_root.join(".claude").join("scene-state");
            if let Some(n) = parts.get(1).and_then(|s| s.parse::<u32>().ok()) {
                let clamped = n.clamp(1, 5);
                let meters_dir = state_dir.join("meters");
                std::fs::create_dir_all(&meters_dir)?;
                std::fs::write(meters_dir.join("heat.txt"), clamped.to_string())?;
                println!("{}", style(format!("🔥 Heat set to {clamped}/5")).red());
            } else {
                // Show current value
                let current = std::fs::read_to_string(state_dir.join("meters").join("heat.txt"))
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(1);
                println!("{}", style(format!("Heat: {current}/5")).dim());
                println!("{}", style("Usage: /heat N (1-5)").dim());
            }
            Ok(false)
        }
        "/beat" => {
            // Override beat: /beat confrontation
            let state_dir = demo_root.join(".claude").join("scene-state");
            let valid_beats = [
                "exposition",
                "confrontation",
                "revelation",
                "intimate-moment",
                "climax",
                "resolution",
            ];
            if let Some(beat) = parts.get(1) {
                let beat_lower = beat.to_lowercase();
                if valid_beats.contains(&beat_lower.as_str()) {
                    std::fs::create_dir_all(&state_dir)?;
                    std::fs::write(state_dir.join("beat.txt"), &beat_lower)?;
                    println!(
                        "{}",
                        style(format!("🎭 Beat set to: {}", beat_lower.to_uppercase())).magenta()
                    );
                } else {
                    println!("{}", style(format!("Unknown beat: {beat}")).red());
                    println!(
                        "{}",
                        style(format!("Valid beats: {}", valid_beats.join(", "))).dim()
                    );
                }
            } else {
                // Show current value
                let current = std::fs::read_to_string(state_dir.join("beat.txt"))
                    .ok().map_or_else(|| "exposition".to_string(), |s| s.trim().to_string());
                println!("{}", style(format!("Beat: {current}")).dim());
                println!(
                    "{}",
                    style(format!("Usage: /beat <{}>", valid_beats.join("|"))).dim()
                );
            }
            Ok(false)
        }
        "/status" => {
            // Show current scene state (minimal style matching scene analysis)
            let state = read_scene_state(demo_root);
            println!(
                "\n{}",
                style("─── STATUS ────────────────────────────────").dim()
            );
            println!(
                "  {} {}/10  {} {}/5",
                style("Tension:").dim(),
                state.tension,
                style("Heat:").dim(),
                state.heat
            );
            println!("  {} {}", style("Beat:").dim(), state.beat);
            Ok(false)
        }
        "/quit" | "/exit" => Ok(true),
        _ => {
            output::display_warning(&format!("Unknown command: {cmd}"));
            display_director_help();
            Ok(false)
        }
    }
}

#[tokio::main]
#[allow(clippy::too_many_lines)] // Entry point with initialization and main loop
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Initialize tracing (quiet by default, use RUST_LOG=info to see logs)
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "warn".parse().unwrap());
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Display greeting
    display_roleplay_greeting(&args);
    display_director_help();

    // Resolve persona paths to absolute (allows running from any directory)
    let persona_a = std::fs::canonicalize(&args.persona_a).ok();
    let persona_b = std::fs::canonicalize(&args.persona_b).ok();

    // Check persona files exist
    if persona_a.is_none() {
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
    if persona_b.is_none() {
        output::display_error(&format!(
            "Persona file not found: {}",
            args.persona_b.display()
        ));
        return Ok(());
    }

    // Unwrap the canonicalized paths (safe after checks above)
    let persona_a = persona_a.unwrap();
    let persona_b = persona_b.unwrap();

    // Create input editor
    let mut editor = input::create_editor()?;
    let _ = ensure_history_dir();
    input::load_history(&mut editor, &history_path());

    // Demo root for /say command file writes
    let demo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // State
    let mut remaining_turns = args.turns;
    let mut turn_number: u32 = 0; // Tracks total turns completed
    let mut running = false;
    let mut hitl_mode = false; // Human-in-the-loop: pause after each turn
    let mut last_response: Option<String> = args.opening.clone();

    // Track which agent speaks next (A starts)
    let mut agent_a_turn = true;

    // Session IDs for conversation continuity - each agent maintains its own context
    let mut session_id_a: Option<String> = None;
    let mut session_id_b: Option<String> = None;
    let mut analyzer_session_id: Option<String> = None; // Haiku analyzer session

    println!(
        "{}",
        style("Type /start to begin, or /help for commands").dim()
    );
    println!();

    // Main loop
    'main: loop {
        // If running and turns remain, continue conversation
        if running && remaining_turns > 0 {
            // Create agent for this turn - use saved session_id for continuity
            let (persona_path, name, is_agent_a, session_id) = if agent_a_turn {
                (&persona_a, &args.name_a, true, session_id_a.as_deref())
            } else {
                (&persona_b, &args.name_b, false, session_id_b.as_deref())
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
                    "{other_name} said: \"{prev}\"\n\nRespond naturally, staying in character."
                )
            } else {
                // Opening - first message
                "Start the conversation. Introduce yourself and begin the dialogue.".to_string()
            };

            // Create and run agent - resume session if we have one
            // Display turn progress (turn_number is 0-indexed, so +1 for display)
            println!(
                "\n{}",
                style(format!(
                    "── Turn {} ({} remaining) ──",
                    turn_number + 1,
                    remaining_turns
                ))
                .dim()
            );

            // Capture state BEFORE agent turn (for analyzer comparison)
            let before_state = read_scene_state(&demo_root);

            // Capture director note BEFORE agent turn (hook will delete it)
            // This lets us pass it to Haiku for director_aligned evaluation
            let director_note = read_director_note(&demo_root, name);

            // Track if we got a response and what it was (for analyzer)
            let mut turn_response: Option<String> = None;
            let mut turn_succeeded = false;

            match create_agent(persona_path, args.scene.as_deref(), session_id, name).await {
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
                                turn_response = Some(response.clone());
                                last_response = Some(response);
                                remaining_turns -= 1;
                                turn_number += 1;
                                agent_a_turn = !agent_a_turn; // Switch turns
                                turn_succeeded = true;
                            } else {
                                output::display_warning("Agent returned no response");
                            }
                        }
                        Err(e) => {
                            output::display_error(&format!("Agent error: {e}"));
                        }
                    }
                    client.close().await.ok();
                }
                Err(e) => {
                    output::display_error(&format!("Failed to create agent: {e}"));
                    running = false;
                }
            }

            // Spawn Haiku analyzer after successful agent turn
            let mut analysis_result: Option<AnalysisResult> = None;
            if turn_succeeded {
                if let Some(ref dialogue) = turn_response {
                    output::show_analyzing();
                    match spawn_analyzer(
                        &demo_root,
                        dialogue,
                        name,
                        &before_state,
                        analyzer_session_id.as_deref(),
                        director_note.as_deref(),
                    )
                    .await
                    {
                        Ok((result, new_session_id)) => {
                            analysis_result = result;
                            // Preserve session for continuity - Haiku remembers previous analyses
                            if new_session_id.is_some() {
                                analyzer_session_id = new_session_id;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Analyzer failed: {}", e);
                        }
                    }
                    output::hide_analyzing();
                }
            }

            // Always show analysis summary after a turn (if available)
            if let Some(ref result) = analysis_result {
                output::display_scene_analysis(result);
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
            } else if hitl_mode {
                // HITL mode: pause for director input
                let next_agent = if agent_a_turn {
                    &args.name_a
                } else {
                    &args.name_b
                };
                println!(
                    "\n{}",
                    style(format!(
                        "── HITL: {next_agent} speaks next. Enter=continue, /say, /tension N, /heat N, or /stop ──"
                    ))
                    .dim()
                );
                // Fall through to get director input
            } else {
                // Auto mode: continue to next turn
                continue;
            }
        }

        // Get Director input (loop until we get a turn-advancing action)
        // Commands like /status, /help loop back; Enter or /start advance turns
        'input: loop {
            match input::get_input(&mut editor)? {
                input::InputResult::Command(cmd) => {
                    if handle_director_command(
                        &cmd,
                        &mut remaining_turns,
                        &mut running,
                        &mut hitl_mode,
                        &demo_root,
                    )? {
                        break 'main; // Quit command
                    }
                    // Check if this command should advance the turn
                    let cmd_lower = cmd.to_lowercase();
                    if cmd_lower.starts_with("/start")
                        || cmd_lower.starts_with("/continue")
                        || cmd_lower.starts_with("/go")
                    {
                        break 'input; // Advance to next turn
                    }
                    // Other commands (/status, /help, /say, etc.) loop back for more input
                }
                input::InputResult::Message(msg) => {
                    if msg.is_empty() {
                        // Empty Enter in HITL mode = start or advance one turn
                        if hitl_mode && remaining_turns > 0 && !running {
                            running = true;
                            println!("{}", style("▶ Scene started (HITL mode)").green());
                        }
                        break 'input; // Advance to next turn
                    }
                    // Director interjection - inject into conversation
                    println!(
                        "\n{} {}",
                        style("[Director]:").magenta().bold(),
                        style(&msg).magenta()
                    );
                    last_response = Some(format!("[Director interjects]: {msg}"));
                    break 'input; // Advance with interjection
                }
                input::InputResult::Exit => break 'main,
                input::InputResult::Interrupt => {
                    running = false;
                    println!();
                    println!("{}", style("⏸ Interrupted - conversation paused").yellow());
                    break 'input;
                }
            }
        }
    }

    // Save history
    let _ = input::save_history(&mut editor, &history_path());

    output::display_goodbye();
    Ok(())
}
