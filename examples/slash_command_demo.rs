//! Slash Command demonstration example
//!
//! This example shows how to:
//! - Configure `setting_sources` to load commands from filesystem
//! - List available slash commands via `supported_commands()`
//! - Invoke a slash command by sending a message starting with `/`
//!
//! Slash commands are defined in:
//! - Project: `.claude/commands/*.md`
//! - User: `~/.claude/commands/*.md`
//!
//! Run with: cargo run --example slash_command_demo

use anthropic_agent_sdk::{
    ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message, SettingSource,
};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Slash Command Demo");
    println!("==================\n");

    // =========================================================================
    // Configure SDK to load commands from filesystem
    // =========================================================================
    println!("📍 Configuration");
    println!("----------------");
    println!("Setting sources: [user, project]");
    println!("This loads commands from ~/.claude/commands/ and .claude/commands/\n");

    let options = ClaudeAgentOptions::builder()
        // Required: Load settings from filesystem to discover custom commands
        // Include Local to load .claude/settings.local.json (gitignored settings)
        .setting_sources(vec![
            SettingSource::User,
            SettingSource::Project,
            SettingSource::Local,
        ])
        .max_turns(3)
        .build();
    let mut client = ClaudeSDKClient::new(options, None).await?;

    // =========================================================================
    // Send initial message to trigger session initialization
    // =========================================================================
    println!("Initializing session...");
    client.send_message("Say 'ready' in one word").await?;
    {
        let mut messages = Box::pin(client.receive_response());
        while let Some(msg) = messages.next().await {
            if let Ok(Message::Result { .. }) = msg {
                break;
            }
        }
    }
    println!();

    // =========================================================================
    // List available slash commands
    // =========================================================================
    println!("📍 Available Slash Commands");
    println!("---------------------------");

    let commands = client.supported_commands();
    if commands.is_empty() {
        println!("No custom slash commands found.");
        println!();
        println!("Create commands in:");
        println!("  - ~/.claude/commands/*.md (user-level, all projects)");
        println!("  - .claude/commands/*.md (project-level)");
        println!();
        println!("Example: Create .claude/commands/hello.md with content:");
        println!("  Say hello to the user in a friendly way.");
        println!();
        println!("Then /hello will be available as a command.\n");
    } else {
        println!("Found {} slash commands:\n", commands.len());
        for cmd in &commands {
            print!("  /{}", cmd.name);
            if !cmd.description.is_empty() {
                print!(" - {}", cmd.description);
            }
            if !cmd.argument_hint.is_empty() {
                print!(" [{}]", cmd.argument_hint);
            }
            println!();
        }
        println!();

        // =========================================================================
        // Invoke the test-cmd if available
        // =========================================================================
        if let Some(test_cmd) = commands.iter().find(|c| c.name == "test-cmd") {
            println!("📍 Invoking /{}", test_cmd.name);
            println!("---------------------------");

            let command_msg = format!("/{} Hello SDK!", test_cmd.name);
            println!("Sending: {}\n", command_msg);

            client.send_message(&command_msg).await?;

            let mut messages = Box::pin(client.receive_response());
            while let Some(msg) = messages.next().await {
                match msg? {
                    Message::Assistant { message, .. } => {
                        for block in &message.content {
                            if let ContentBlock::Text { text } = block {
                                // Print first 500 chars of response
                                let preview = if text.len() > 500 {
                                    format!("{}...", &text[..500])
                                } else {
                                    text.clone()
                                };
                                println!("Response:\n{}\n", preview);
                            }
                        }
                    }
                    Message::Result { .. } => {
                        println!("✓ Command completed");
                        break;
                    }
                    _ => {}
                }
            }
        } else {
            println!(
                "Note: /test-cmd not found - create .claude/commands/test-cmd.md to test invocation"
            );
        }
    }

    client.close().await?;
    println!("\nDemo complete!");

    Ok(())
}
