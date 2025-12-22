//! Agent Skills demonstration example
//!
//! This example shows how to use Agent Skills with the SDK:
//! - Configure `setting_sources` to load skills from filesystem
//! - Enable the `Skill` tool in `allowed_tools`
//! - Claude automatically discovers and invokes relevant skills
//!
//! Skills are defined as SKILL.md files in:
//! - Project: `.claude/skills/*/SKILL.md`
//! - User: `~/.claude/skills/*/SKILL.md`
//!
//! Run with: cargo run --example `skills_demo`

use anthropic_agent_sdk::{
    ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message, SettingSource,
};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Agent Skills Demo");
    println!("=================\n");

    // =========================================================================
    // Configure SDK to load skills from filesystem
    // =========================================================================
    println!("📍 Configuration");
    println!("----------------");
    println!("Setting sources: [user, project]");
    println!("Allowed tools: [Skill, Read, Bash]\n");

    let options = ClaudeAgentOptions::builder()
        // Required: Load settings from filesystem to discover skills
        // Include Local to load .claude/settings.local.json (gitignored settings)
        .setting_sources(vec![
            SettingSource::User,
            SettingSource::Project,
            SettingSource::Local,
        ])
        // Required: Enable the Skill tool
        .allowed_tools(vec!["Skill".into(), "Read".into(), "Bash".into()])
        .max_turns(5)
        .build();

    let mut client = ClaudeSDKClient::new(options, None).await?;
    println!("✓ Client connected with skills enabled\n");

    // =========================================================================
    // Ask Claude what skills are available
    // =========================================================================
    println!("📍 Discovering Available Skills");
    println!("-------------------------------");
    println!("Asking Claude: 'What skills are available?'\n");

    client
        .send_message("What skills are available? List them briefly.")
        .await?;

    {
        let mut messages = Box::pin(client.receive_response());
        while let Some(msg) = messages.next().await {
            match msg? {
                Message::Assistant { message, .. } => {
                    for block in &message.content {
                        if let ContentBlock::Text { text } = block {
                            println!("Claude: {text}\n");
                        }
                    }
                }
                Message::Result { .. } => {
                    break;
                }
                _ => {}
            }
        }
    } // Drop messages stream before closing

    client.close().await?;

    // =========================================================================
    // Help message if no skills found
    // =========================================================================
    println!("📍 Creating Skills");
    println!("-----------------");
    println!("To create a skill, add a SKILL.md file:");
    println!();
    println!("  mkdir -p .claude/skills/my-skill");
    println!("  cat > .claude/skills/my-skill/SKILL.md << 'EOF'");
    println!("  ---");
    println!("  description: Help with specific task X");
    println!("  ---");
    println!("  # My Skill");
    println!("  Instructions for Claude when this skill is invoked...");
    println!("  EOF");
    println!();
    println!("Claude will automatically invoke the skill when the request");
    println!("matches the description.\n");

    println!("Demo complete!");

    Ok(())
}
