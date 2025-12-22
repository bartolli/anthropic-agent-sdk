//! MCP Integration Example
//!
//! This example demonstrates how to configure the Claude Agent SDK to use
//! an external MCP server. It shows:
//!
//! 1. Configuring an MCP server via `McpStdioServerConfig`
//! 2. Passing the configuration to `ClaudeAgentOptions`
//! 3. Claude automatically discovering and using the MCP tools
//!
//! ## Prerequisites
//!
//! Build the MCP server example first:
//!   cargo build --example `mcp_server` --features rmcp
//!
//! Then run this integration example:
//!   cargo run --example `mcp_integration`
//!
//! ## How It Works
//!
//! 1. We configure an MCP server pointing to the `mcp_server` example binary
//! 2. Claude Code CLI spawns the server as a subprocess
//! 3. The CLI discovers tools via the MCP protocol (tools/list)
//! 4. When Claude needs a tool, CLI calls it via MCP protocol (tools/call)

use anthropic_agent_sdk::{
    ClaudeAgentOptions, ClaudeSDKClient, McpServerConfig, McpServers, McpStdioServerConfig,
    Message, PermissionMode,
};
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MCP Integration Example ===\n");

    // Find the path to the mcp_server example binary
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    // Check release first, then debug
    let release_path = format!("{manifest_dir}/target/release/examples/mcp_server");
    let debug_path = format!("{manifest_dir}/target/debug/examples/mcp_server");

    let server_path = if std::path::Path::new(&release_path).exists() {
        release_path
    } else if std::path::Path::new(&debug_path).exists() {
        debug_path
    } else {
        eprintln!("Error: MCP server binary not found.");
        eprintln!("Checked: {release_path}");
        eprintln!("Checked: {debug_path}");
        eprintln!("\nPlease build it first:");
        eprintln!("  cargo build --example mcp_server --features rmcp");
        std::process::exit(1);
    };

    println!("Using MCP server: {server_path}\n");

    // Configure the MCP server
    let mut mcp_servers = HashMap::new();
    mcp_servers.insert(
        "demo-tools".to_string(),
        McpServerConfig::Stdio(McpStdioServerConfig {
            server_type: Some("stdio".to_string()),
            command: server_path,
            args: None,
            env: None,
        }),
    );

    // Create options with the MCP server configured
    // Use BypassPermissions to auto-allow all tools (including MCP tools)
    let options = ClaudeAgentOptions::builder()
        .system_prompt("You have access to MCP tools including a calculator, weather lookup, and notes. \
                        Use them when appropriate to help the user. After using tools, always report the results.")
        .max_turns(5)
        .mcp_servers(McpServers::Dict(mcp_servers))
        .permission_mode(PermissionMode::BypassPermissions)
        .build();

    println!("Starting Claude with MCP server configured...\n");

    // Create the client
    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Send a message that should trigger tool use
    let prompt = "Please calculate 42 + 17 using the add tool, and then get the weather for Tokyo.";
    println!("User: {prompt}\n");

    client.send_message(prompt).await?;

    // Process messages
    println!("Claude:");
    while let Some(message) = client.next_message().await {
        match message? {
            Message::Assistant { message, .. } => {
                for block in message.content {
                    match block {
                        anthropic_agent_sdk::ContentBlock::Text { text } => {
                            println!("  {text}");
                        }
                        anthropic_agent_sdk::ContentBlock::ToolUse { name, input, .. } => {
                            println!("  [Using tool: {name} with {input:?}]");
                        }
                        anthropic_agent_sdk::ContentBlock::ToolResult {
                            content,
                            tool_use_id,
                            ..
                        } => {
                            println!("  [Tool result for {tool_use_id}]");
                            if let Some(value) = content {
                                match value {
                                    anthropic_agent_sdk::ContentValue::String(text) => {
                                        println!("    => {text}");
                                    }
                                    anthropic_agent_sdk::ContentValue::Blocks(blocks) => {
                                        println!("    => {blocks:?}");
                                    }
                                }
                            }
                        }
                        other => {
                            // Debug: print any other content block types
                            println!("  [Other block: {other:?}]");
                        }
                    }
                }
            }
            Message::User { message, .. } => {
                // Tool results often come as User messages
                if let Some(anthropic_agent_sdk::UserContent::Blocks(blocks)) = message.content {
                    for block in blocks {
                        if let anthropic_agent_sdk::ContentBlock::ToolResult {
                            content,
                            tool_use_id,
                            ..
                        } = block
                        {
                            println!("  [Tool result for {tool_use_id}]");
                            if let Some(value) = content {
                                match value {
                                    anthropic_agent_sdk::ContentValue::String(text) => {
                                        println!("    => {text}");
                                    }
                                    anthropic_agent_sdk::ContentValue::Blocks(b) => {
                                        println!("    => {b:?}");
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Message::Result { result, .. } => {
                // Print the final result text if present
                if let Some(ref text) = result {
                    println!("\n  Final result: {text}");
                }
                println!("\n[Conversation complete]");
                break;
            }
            _ => {}
        }
    }

    // Check MCP server status
    if let Some(info) = client.session_info() {
        println!("\n--- MCP Server Status ---");
        for server in &info.mcp_servers {
            println!(
                "  {}: {} (tools: {})",
                server.name,
                server.status,
                server.tools.join(", ")
            );
        }
    }

    client.close().await?;

    println!("\n=== Example Complete ===");
    println!("\nThis demonstrated:");
    println!("1. Configuring an MCP server via McpStdioServerConfig");
    println!("2. Claude discovering tools from the MCP server");
    println!("3. Claude using MCP tools to answer questions");

    Ok(())
}
