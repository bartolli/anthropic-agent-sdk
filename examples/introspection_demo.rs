//! Example demonstrating introspection methods in `ClaudeSDKClient`
//!
//! This example shows how to use:
//! - `session_info()` - Get session initialization data (model, tools, cwd)
//! - `supported_models()` - Get list of available models
//! - `supported_commands()` - Get available slash commands
//! - `mcp_server_status()` - Get MCP server connection status
//! - `account_info()` - Get account information
//!
//! Run with: cargo run --example `introspection_demo`

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, Message, SettingSource};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Introspection Methods Demo");
    println!("==========================\n");

    // Create client with all setting sources to load MCP servers
    let options = ClaudeAgentOptions::builder()
        .setting_sources(vec![
            SettingSource::User,
            SettingSource::Project,
            SettingSource::Local, // Loads .claude/settings.local.json (e.g., enableAllProjectMcpServers)
        ])
        .max_turns(2)
        .build();
    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Initialize session by sending a message (required to capture init data)
    println!("Initializing session...\n");
    client.send_message("Say 'ready'").await?;
    {
        let mut messages = Box::pin(client.receive_response());
        while let Some(msg) = messages.next().await {
            if let Ok(Message::Result { .. }) = msg {
                break;
            }
        }
    }

    // =========================================================================
    // Test 1: Session Info
    // =========================================================================
    println!("📍 Session Info");
    println!("---------------");

    if let Some(info) = client.session_info() {
        println!("Model: {:?}", info.model);
        println!("CWD: {:?}", info.cwd);
        println!("Tools available: {}", info.tools.len());

        // Show first 5 tools
        for tool in info.tools.iter().take(5) {
            println!("  - {}", tool.name);
        }
        if info.tools.len() > 5 {
            println!("  ... and {} more", info.tools.len() - 5);
        }

        // MCP servers from session info
        if !info.mcp_servers.is_empty() {
            println!("MCP servers: {}", info.mcp_servers.len());
            for server in &info.mcp_servers {
                let status_icon = if server.is_connected() { "✓" } else { "✗" };
                println!(
                    "  {} {} [{}] ({} tools)",
                    status_icon,
                    server.name,
                    server.status,
                    server.tools.len()
                );
            }
        }
    } else {
        println!("(Session info not available yet)");
    }

    // =========================================================================
    // Test 2: Supported Models (static method)
    // =========================================================================
    println!("\n📍 Supported Models");
    println!("-------------------");

    let models = ClaudeSDKClient::supported_models();
    println!("Known models: {}", models.len());
    for model in &models {
        let thinking = if model.supports_thinking {
            " (thinking)"
        } else {
            ""
        };
        println!("  - {}{}", model.id, thinking);
        if let Some(name) = &model.name {
            println!("    Name: {name}");
        }
    }

    // =========================================================================
    // Test 3: Supported Commands (SlashCommand type)
    // =========================================================================
    println!("\n📍 Supported Commands (SlashCommand)");
    println!("-------------------------------------");

    let commands = client.supported_commands();
    if commands.is_empty() {
        println!("No custom slash commands defined");
    } else {
        println!("Available slash commands: {}", commands.len());
        for cmd in &commands {
            println!("  /{} - {}", cmd.name, cmd.description);
            if !cmd.argument_hint.is_empty() {
                println!("    Args: {}", cmd.argument_hint);
            }
        }
    }

    // =========================================================================
    // Test 4: MCP Server Status
    // =========================================================================
    println!("\n📍 MCP Server Status");
    println!("--------------------");

    let servers = client.mcp_server_status();
    if servers.is_empty() {
        println!("No MCP servers configured");
    } else {
        println!("MCP servers: {}", servers.len());
        for server in &servers {
            let icon = if server.is_connected() { "✓" } else { "✗" };
            println!("  {} {} ({})", icon, server.name, server.status);
            if let Some(err) = &server.error {
                println!("    Error: {err}");
            }
            if !server.tools.is_empty() {
                println!("    Tools: {}", server.tools.join(", "));
            }
        }
    }

    // =========================================================================
    // Test 5: Skills
    // =========================================================================
    println!("\n📍 Skills");
    println!("---------");

    if let Some(info) = client.session_info() {
        let skills = info
            .extra
            .get("skills")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|s| s.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

        if skills.is_empty() {
            println!("No skills available");
            println!("  Create skills in .claude/skills/ or ~/.claude/skills/");
        } else {
            println!("Available skills: {}", skills.len());
            for skill in &skills {
                println!("  - {skill}");
            }
        }
    }

    // =========================================================================
    // Test 6: Plugins
    // =========================================================================
    println!("\n📍 Plugins");
    println!("----------");

    if let Some(info) = client.session_info() {
        let plugins = info
            .extra
            .get("plugins")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|p| p.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

        if plugins.is_empty() {
            println!("No plugins installed");
        } else {
            println!("Installed plugins: {}", plugins.len());
            for plugin in &plugins {
                println!("  - {plugin}");
            }
        }
    }

    // =========================================================================
    // Test 7: Agents (subagent types)
    // =========================================================================
    println!("\n📍 Agents");
    println!("---------");

    if let Some(info) = client.session_info() {
        let agents = info
            .extra
            .get("agents")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|a| a.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

        if agents.is_empty() {
            println!("No agents available");
        } else {
            println!("Available agents: {}", agents.len());
            for agent in &agents {
                println!("  - {agent}");
            }
        }
    }

    // =========================================================================
    // Test 8: Account Info
    // =========================================================================
    println!("\n📍 Account Info");
    println!("---------------");

    match client.account_info() {
        Ok(info) => {
            println!("OAuth account: {}", info.is_oauth);
            if let Some(email) = &info.email {
                println!("Email: {email}");
            }
            if let Some(account_id) = &info.account_id {
                println!("Account ID: {account_id}");
            }
            if let Some(org_id) = &info.organization_id {
                println!("Organization: {org_id}");
            }
        }
        Err(e) => println!("Error getting account info: {e}"),
    }

    // Cleanup
    client.close().await?;
    println!("\n✓ Demo complete!");

    Ok(())
}
