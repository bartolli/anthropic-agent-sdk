//! Debug init message to see what the CLI sends
//!
//! This example shows the raw init data from the CLI, including:
//! - Available tools, agents, skills, slash commands
//! - MCP servers (if configured and approved)
//! - Session metadata

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, Message, SettingSource};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Debug Init Message");
    println!("===================\n");

    // Load all setting sources to capture MCP server config:
    // - User: ~/.claude/settings.json
    // - Project: .claude/settings.json (shared with team)
    // - Local: .claude/settings.local.json (gitignored, has enableAllProjectMcpServers)
    let options = ClaudeAgentOptions::builder()
        .setting_sources(vec![
            SettingSource::User,
            SettingSource::Project,
            SettingSource::Local,
        ])
        .max_turns(1)
        .build();

    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Send a quick message and process response to capture init
    client.send_message("Say hi briefly").await?;
    {
        let mut messages = Box::pin(client.receive_response());
        while let Some(msg) = messages.next().await {
            match &msg {
                Ok(Message::System { subtype, data }) => {
                    println!("System message [{}]:", subtype);
                    if subtype == "init" {
                        println!("  Raw init data keys:");
                        if let Some(obj) = data.as_object() {
                            for (key, value) in obj {
                                // Show full list for tools to see MCP naming
                                if key == "tools" {
                                    if let Some(arr) = value.as_array() {
                                        let mcp_tools: Vec<_> = arr
                                            .iter()
                                            .filter_map(|v| v.as_str())
                                            .filter(|t| t.starts_with("mcp__"))
                                            .collect();
                                        println!(
                                            "    {}: {} total, {} MCP tools:",
                                            key,
                                            arr.len(),
                                            mcp_tools.len()
                                        );
                                        for t in &mcp_tools {
                                            println!("      - {}", t);
                                        }
                                    }
                                } else {
                                    let preview = format!("{}", value);
                                    let preview = if preview.len() > 200 {
                                        format!("{}...", &preview[..200])
                                    } else {
                                        preview
                                    };
                                    println!("    {}: {}", key, preview);
                                }
                            }
                        }
                    }
                }
                Ok(Message::Result { .. }) => break,
                _ => {}
            }
        }
    }

    // Now check session_info
    println!("\nSession Info (captured):");
    if let Some(info) = client.session_info() {
        println!("  model: {:?}", info.model);
        println!("  cwd: {:?}", info.cwd);
        println!("  tools count: {}", info.tools.len());
        println!("  mcp_servers count: {}", info.mcp_servers.len());
        println!();
        println!("Extra fields ({}):", info.extra.len());
        for (key, value) in &info.extra {
            let preview = format!("{}", value);
            let preview = if preview.len() > 200 {
                format!("{}...", &preview[..200])
            } else {
                preview
            };
            println!("  {}: {}", key, preview);
        }
    } else {
        println!("  None captured!");
    }

    client.close().await?;
    Ok(())
}
