//! Knowledge Builder Example
//!
//! Demonstrates knowledge accumulation through iterative multi-source research:
//! - `SubagentStart` hooks inject accumulated knowledge into sub-agents
//! - `PostToolUse` hooks extract findings from tool responses
//! - Shared state accumulates knowledge across the session
//!
//! This is a Rust port of the TypeScript patterns from codanna-agent:
//! - knowledgeBuilder `AgentDefinition`
//! - createKnowledgeHooks with knowledge accumulation
//!
//! Run with: cargo run --example `knowledge_builder`
//! Run with debug: `RUST_LOG=debug` cargo run --example `knowledge_builder`

use anthropic_agent_sdk::ClaudeSDKClient;
use anthropic_agent_sdk::hooks::{HookManager, HookMatcherBuilder};
use anthropic_agent_sdk::types::{
    AgentDefinition, ClaudeAgentOptions, ContentBlock, HookEvent, HookOutput, Message,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

/// Shared knowledge state - accumulated across hooks
#[derive(Debug, Default)]
struct KnowledgeState {
    /// Accumulated findings from tools
    accumulated: String,
    /// Sources of findings (file paths, URLs)
    sources: Vec<String>,
    /// Tool invocations tracked
    tool_calls: Vec<(String, String)>, // (tool_name, summary)
}

impl KnowledgeState {
    fn add_finding(&mut self, tool: &str, finding: &str, source: Option<&str>) {
        if !finding.is_empty() {
            self.accumulated
                .push_str(&format!("\n---\n[{tool}] {finding}"));
            if let Some(src) = source {
                self.sources.push(src.to_string());
            }
        }
    }
}

/// Extract key findings from tool responses
/// Note: `tool_response` is typically a plain string, not a nested JSON object
fn extract_findings(
    tool_name: &str,
    response: &serde_json::Value,
) -> Option<(String, Option<String>)> {
    // First, try to get the response as a direct string (most common case)
    let response_str = response.as_str();

    match tool_name {
        // Bash command output - response is a direct string
        "Bash" => {
            if let Some(output) = response_str {
                if output.len() > 10 {
                    let preview = if output.len() > 300 {
                        format!("{}...", &output[..300])
                    } else {
                        output.to_string()
                    };
                    return Some((preview, None));
                }
            }
            None
        }
        // Read file results - response is a direct string
        "Read" => {
            if let Some(content) = response_str {
                if content.len() > 10 {
                    let preview = if content.len() > 300 {
                        format!("{}...", &content[..300])
                    } else {
                        content.to_string()
                    };
                    return Some((preview, None));
                }
            }
            None
        }
        // Grep results
        "Grep" => {
            if let Some(output) = response_str {
                if !output.is_empty() {
                    let preview = if output.len() > 300 {
                        format!("{}...", &output[..300])
                    } else {
                        output.to_string()
                    };
                    return Some((preview, None));
                }
            }
            None
        }
        // WebSearch/WebFetch results
        "WebSearch" | "WebFetch" => {
            if let Some(output) = response_str {
                if !output.is_empty() {
                    let preview = if output.len() > 300 {
                        format!("{}...", &output[..300])
                    } else {
                        output.to_string()
                    };
                    return Some((preview, None));
                }
            }
            None
        }
        // Codanna semantic search
        name if name.contains("semantic_search") || name.contains("codanna") => {
            if let Some(results) = response_str {
                let lines: Vec<&str> = results.lines().take(5).collect();
                if !lines.is_empty() {
                    return Some((lines.join("\n"), None));
                }
            }
            None
        }
        // Any other tool - try to extract string content
        _ => {
            if let Some(output) = response_str {
                if output.len() > 20 {
                    let preview = if output.len() > 200 {
                        format!("{}...", &output[..200])
                    } else {
                        output.to_string()
                    };
                    return Some((preview, None));
                }
            }
            None
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_ansi(true)
        .init();

    info!("=== Knowledge Builder Example ===");

    // Shared knowledge state - this is the key to accumulation
    let knowledge = Arc::new(Mutex::new(KnowledgeState::default()));

    // SubagentStart hook - injects accumulated knowledge into sub-agents
    let knowledge_start = knowledge.clone();
    let subagent_start_hook = HookManager::callback(move |input, tool_name, ctx| {
        let knowledge = knowledge_start.clone();
        async move {
            // Log context to verify session info is passed (TypeScript SDK parity)
            info!(
                ctx_session_id = ?ctx.session_id,
                ctx_cwd = ?ctx.cwd,
                ctx_has_cancel_token = ctx.cancellation_token.is_some(),
                "HookContext received"
            );

            // Log raw input for debugging
            debug!(
                raw_input = %serde_json::to_string_pretty(&input).unwrap_or_default(),
                tool_name = ?tool_name,
                "SubagentStart hook invoked"
            );

            let agent_id = input
                .get("agent_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let agent_type = input
                .get("agent_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            info!(
                agent_id = %agent_id,
                agent_type = %agent_type,
                "Subagent starting"
            );

            // Check if we have accumulated knowledge to inject
            let state = knowledge.lock().unwrap();
            if !state.accumulated.is_empty() {
                info!(
                    findings_len = state.accumulated.len(),
                    sources = state.sources.len(),
                    "Injecting prior findings into subagent"
                );

                // Return hookSpecificOutput with additionalContext
                // This injects our accumulated knowledge into the subagent
                return Ok(HookOutput {
                    decision: None,
                    system_message: None,
                    hook_specific_output: Some(serde_json::json!({
                        "hookEventName": "SubagentStart",
                        "additionalContext": format!(
                            "Prior findings from this research session:\n{}",
                            state.accumulated
                        )
                    })),
                });
            }

            Ok(HookOutput::default())
        }
    });

    // PostToolUse hook - extracts and accumulates findings
    let knowledge_post = knowledge.clone();
    let post_tool_hook = HookManager::callback(move |input, tool_name, ctx| {
        let knowledge = knowledge_post.clone();
        async move {
            // Check cancellation before processing
            if ctx.is_cancelled() {
                return Ok(HookOutput::default());
            }
            let tool = tool_name.clone().unwrap_or_else(|| "unknown".to_string());

            // Log raw input for debugging (use char-safe truncation)
            let input_str = serde_json::to_string_pretty(&input).unwrap_or_default();
            let preview: String = if input_str.chars().count() > 400 {
                format!("{}...", input_str.chars().take(400).collect::<String>())
            } else {
                input_str
            };
            debug!(
                raw_input = %preview,
                tool_name = ?tool_name,
                "PostToolUse hook invoked"
            );

            // Get tool response
            let response = input
                .get("tool_response")
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            // Extract findings from the response
            if let Some((finding, source)) = extract_findings(&tool, &response) {
                let mut state = knowledge.lock().unwrap();
                state.add_finding(&tool, &finding, source.as_deref());
                state.tool_calls.push((tool.clone(), finding.clone()));

                info!(
                    tool = %tool,
                    finding_len = finding.len(),
                    "Accumulated finding from tool"
                );
            } else {
                debug!(tool = %tool, "No findings extracted from tool response");
            }

            Ok(HookOutput::default())
        }
    });

    // SubagentStop hook - logs what subagent discovered
    let knowledge_stop = knowledge.clone();
    let subagent_stop_hook = HookManager::callback(move |input, tool_name, ctx| {
        let knowledge = knowledge_stop.clone();
        async move {
            // Log session context
            debug!(
                session_id = ?ctx.session_id,
                cwd = ?ctx.cwd,
                "SubagentStop context"
            );

            // Log raw input for debugging - shows subagent result (char-safe)
            let input_str = serde_json::to_string_pretty(&input).unwrap_or_default();
            let preview: String = if input_str.chars().count() > 800 {
                format!("{}...", input_str.chars().take(800).collect::<String>())
            } else {
                input_str
            };
            debug!(
                raw_input = %preview,
                tool_name = ?tool_name,
                "SubagentStop hook invoked"
            );

            let agent_id = input
                .get("agent_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            // Try to get the subagent's result/output
            let result_preview = input
                .get("result")
                .or_else(|| input.get("output"))
                .map(|v| {
                    let s = serde_json::to_string(v).unwrap_or_default();
                    if s.len() > 200 {
                        format!("{}...", &s[..200])
                    } else {
                        s
                    }
                })
                .unwrap_or_else(|| "no result field".to_string());

            let state = knowledge.lock().unwrap();
            info!(
                agent_id = %agent_id,
                total_findings = state.accumulated.len(),
                tool_calls = state.tool_calls.len(),
                result_preview = %result_preview,
                "Subagent completed"
            );

            Ok(HookOutput::default())
        }
    });

    // Build hooks configuration
    let mut hooks = HashMap::new();
    hooks.insert(
        HookEvent::SubagentStart,
        vec![
            HookMatcherBuilder::new(None::<String>)
                .add_hook(subagent_start_hook)
                .build(),
        ],
    );
    hooks.insert(
        HookEvent::SubagentStop,
        vec![
            HookMatcherBuilder::new(None::<String>)
                .add_hook(subagent_stop_hook)
                .build(),
        ],
    );
    // Match knowledge-gathering tools: WebSearch, WebFetch, Bash, Read
    hooks.insert(
        HookEvent::PostToolUse,
        vec![
            HookMatcherBuilder::new(Some("WebSearch|WebFetch|Bash|Read|SlashCommand"))
                .add_hook(post_tool_hook)
                .build(),
        ],
    );

    // Define the knowledgeBuilder agent (from TypeScript blueprint)
    let mut agents = HashMap::new();
    agents.insert(
        "knowledge-builder".to_string(),
        AgentDefinition {
            description: "Build deep understanding through iterative multi-source research"
                .to_string(),
            prompt: r#"You iteratively build knowledge until complete understanding.

SOURCES AVAILABLE:
- Code: Bash("codanna mcp semantic_search_with_context query:...")
- Files: Read with file paths from search results
- Grep: Search for specific patterns

WORKFLOW:
1. STATE current understanding
2. IDENTIFY gaps - what questions remain?
3. DISPATCH queries to fill gaps (parallel when independent)
4. UPDATE understanding with new findings
5. REPEAT until gaps filled OR 3 iterations with no new info

OUTPUT:
Structured findings with:
- What you learned
- Where each fact came from (code location)
- How pieces connect
- Remaining unknowns"#
                .to_string(),
            tools: Some(vec![
                "Bash".to_string(),
                "Read".to_string(),
                "Grep".to_string(),
            ]),
            model: Some("sonnet".to_string()),
        },
    );

    // Create client options
    let options = ClaudeAgentOptions::builder()
        .max_turns(8)
        .system_prompt(
            "You are a research orchestrator. Use the 'knowledge-builder' agent via Task tool \
             to build comprehensive understanding of topics. The agent iteratively researches \
             using code search and file reading until it has complete knowledge.",
        )
        .agents(agents)
        .hooks(hooks)
        .build();

    match ClaudeSDKClient::new(options, None).await {
        Ok(mut client) => {
            info!("Client connected with knowledge accumulation hooks");

            // Simple 3-hop task to verify hook flow:
            // 1. List files in src/
            // 2. Read src/error.rs
            // 3. Summarize what was found
            let query = "Use the Task tool with subagent_type='knowledge-builder'. \
                        The agent should: 1) run 'ls src/' to list files, \
                        2) read src/error.rs, 3) summarize the error types found. \
                        Keep it to exactly 3 tool calls.";

            client.send_message(query).await?;

            // Read responses
            loop {
                match tokio::time::timeout(Duration::from_secs(120), client.next_message()).await {
                    Ok(Some(message)) => match message? {
                        Message::System { subtype, data } => {
                            // Log system messages to understand initialization
                            info!(
                                subtype = %subtype,
                                session_id = %data.get("session_id").and_then(|v| v.as_str()).unwrap_or("none"),
                                cwd = %data.get("cwd").and_then(|v| v.as_str()).unwrap_or("none"),
                                "System message received"
                            );
                        }
                        Message::Assistant { message, .. } => {
                            for block in &message.content {
                                if let ContentBlock::Text { text } = block {
                                    let preview = if text.len() > 300 {
                                        format!("{}...", &text[..300])
                                    } else {
                                        text.clone()
                                    };
                                    info!("Response: {}", preview);
                                }
                            }
                        }
                        Message::Result {
                            session_id,
                            duration_ms,
                            num_turns,
                            ..
                        } => {
                            info!(
                                session = %session_id,
                                duration_ms = duration_ms,
                                turns = num_turns,
                                "Session complete"
                            );
                            break;
                        }
                        _ => {}
                    },
                    Ok(None) => {
                        debug!("Stream ended");
                        break;
                    }
                    Err(_) => {
                        warn!("Timeout after 120s");
                        break;
                    }
                }
            }

            client.close().await?;

            // Final knowledge summary
            let state = knowledge.lock().unwrap();
            info!("=== Knowledge Accumulation Summary ===");
            info!(
                total_findings_chars = state.accumulated.len(),
                unique_sources = state.sources.len(),
                tool_invocations = state.tool_calls.len(),
                "Statistics"
            );

            if !state.sources.is_empty() {
                info!("Sources discovered:");
                for (i, src) in state.sources.iter().take(10).enumerate() {
                    info!(index = i + 1, source = %src, "Source");
                }
            }

            if !state.tool_calls.is_empty() {
                info!("Tool calls that yielded findings:");
                for (i, (tool, summary)) in state.tool_calls.iter().take(5).enumerate() {
                    let preview = if summary.len() > 60 {
                        format!("{}...", &summary[..60])
                    } else {
                        summary.clone()
                    };
                    info!(index = i + 1, tool = %tool, summary = %preview, "Finding");
                }
            }
        }
        Err(e) => {
            warn!(error = %e, "Could not connect to Claude CLI");
        }
    }

    info!("=== Knowledge Builder Complete ===");
    Ok(())
}
