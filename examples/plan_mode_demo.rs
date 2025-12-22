//! Plan Mode Demo
//!
//! This example demonstrates using Plan mode, where Claude is instructed to
//! analyze and plan rather than execute. Plan mode is useful for:
//! - Getting implementation plans before committing to changes
//! - Understanding what Claude would do without side effects
//! - Reviewing proposed changes before approval
//!
//! Note: Plan mode is a behavioral mode that instructs Claude to plan rather
//! than hard-blocking tools. Claude may still use Write to create plan files
//! and uses `ExitPlanMode` tool when ready for user approval.

use anthropic_agent_sdk::{
    ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message, PermissionMode, SettingSource,
};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Plan Mode Demo");
    println!("===============\n");
    println!("Starting Claude in PLAN mode - it will analyze but not execute.\n");

    // Start session in Plan mode
    let options = ClaudeAgentOptions::builder()
        .permission_mode(PermissionMode::Plan)
        .setting_sources(vec![
            SettingSource::User,
            SettingSource::Project,
            SettingSource::Local,
        ])
        .max_turns(5)
        .build();

    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Ask Claude to plan a task
    println!("─────────────────────────────────────────────────────────────────");
    println!("REQUEST: Plan how to add a new error type to the SDK");
    println!("─────────────────────────────────────────────────────────────────\n");

    client
        .send_message(
            "Plan how I would add a new error type called 'RateLimitError' to this SDK. \
             Explore the codebase to understand the error handling patterns, then provide \
             a step-by-step implementation plan. Don't implement it - just plan.",
        )
        .await?;

    let mut tool_uses: Vec<String> = Vec::new();
    let mut exit_plan_detected = false;

    {
        let mut messages = Box::pin(client.receive_response());
        while let Some(msg) = messages.next().await {
            match msg {
                Ok(Message::Assistant { message, .. }) => {
                    for block in &message.content {
                        match block {
                            ContentBlock::ToolUse { name, input, .. } => {
                                tool_uses.push(name.clone());

                                // Check for ExitPlanMode tool
                                if name == "ExitPlanMode" {
                                    exit_plan_detected = true;
                                    println!("\n  [ExitPlanMode detected!]");
                                    if let Some(plan) = input.get("plan") {
                                        println!("  Plan content:");
                                        println!("  {plan}");
                                    }
                                } else {
                                    let input_str = format!("{input}");
                                    let preview = if input_str.len() > 80 {
                                        format!("{}...", &input_str[..80])
                                    } else {
                                        input_str
                                    };
                                    println!("  [Tool: {name}] {preview}");
                                }
                            }
                            ContentBlock::Text { text } => {
                                let trimmed = text.trim();
                                if !trimmed.is_empty() {
                                    // Show Claude's analysis/planning text
                                    println!("\n  Claude:");
                                    for line in trimmed.lines().take(20) {
                                        println!("    {line}");
                                    }
                                    if trimmed.lines().count() > 20 {
                                        println!("    ... (truncated)");
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Message::User { .. }) => {
                    // Tool results - just acknowledge
                }
                Ok(Message::Result { .. }) => {
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    println!("  [Error] {e}");
                    break;
                }
            }
        }
    } // Drop messages stream

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("Plan Mode Summary");
    println!("═══════════════════════════════════════════════════════════════");

    // Show session permission mode (captured from init message)
    if let Some(info) = client.session_info() {
        println!("  Permission Mode: {:?}", info.permission_mode());
        println!("  Is Plan Mode: {}", info.is_plan_mode());
    }

    println!("  Tools used: {tool_uses:?}");
    println!("  ExitPlanMode detected: {exit_plan_detected}");

    // Categorize tools
    let read_only: Vec<_> = tool_uses
        .iter()
        .filter(|t| matches!(t.as_str(), "Read" | "Glob" | "Grep" | "Task"))
        .collect();
    let write_tools: Vec<_> = tool_uses
        .iter()
        .filter(|t| matches!(t.as_str(), "Write" | "Edit" | "Bash"))
        .collect();
    let plan_tools: Vec<_> = tool_uses
        .iter()
        .filter(|t| matches!(t.as_str(), "EnterPlanMode" | "ExitPlanMode"))
        .collect();

    println!("\n  Read-only tools: {read_only:?}");
    println!("  Plan mode tools: {plan_tools:?}");
    println!("  Write tools: {write_tools:?}");

    if !write_tools.is_empty() {
        println!("\n  Note: Write/Edit in plan mode typically means writing to ~/.claude/plans/");
        println!("        (plan files, not project code changes)");
    }
    println!("\n  Plan mode session completed");

    client.close().await?;
    Ok(())
}
