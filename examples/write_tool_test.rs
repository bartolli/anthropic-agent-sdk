//! Minimal test to verify Write tool actually writes to disk
//!
//! Run with: `RUST_LOG=info` cargo run --example `write_tool_test`

use anthropic_agent_sdk::{
    ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message, PermissionMode, ToolName,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".parse().unwrap()),
        )
        .init();

    println!("=== Write Tool Test ===\n");

    // Use a temp file path
    let test_file = std::env::temp_dir().join("sdk_write_test.txt");
    let test_file_str = test_file.to_string_lossy();

    // Clean up any existing file
    let _ = std::fs::remove_file(&test_file);

    // Test 1: Without allowed_tools constraint
    println!("--- Test 1: No tool restrictions ---");
    let options = ClaudeAgentOptions::builder()
        .model("haiku")
        .permission_mode(PermissionMode::BypassPermissions)
        .max_turns(1_u32)
        .stderr(std::sync::Arc::new(|line| {
            eprintln!("[stderr] {line}");
        }))
        .build();

    run_write_test(&test_file, test_file_str.as_ref(), options).await?;

    // Clean up for next test
    let _ = std::fs::remove_file(&test_file);

    // Test 2: With allowed_tools constraint (like role-play demo)
    println!("\n--- Test 2: With allowed_tools=[Write] ---");
    let test_file2 = std::env::temp_dir().join("sdk_write_test2.txt");
    let test_file2_str = test_file2.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&test_file2);

    let options2 = ClaudeAgentOptions::builder()
        .model("haiku")
        .permission_mode(PermissionMode::BypassPermissions)
        .allowed_tools(vec![ToolName::new("Write")])
        .max_turns(1_u32)
        .stderr(std::sync::Arc::new(|line| {
            eprintln!("[stderr] {line}");
        }))
        .build();

    run_write_test(&test_file2, &test_file2_str, options2).await?;

    // Cleanup
    let _ = std::fs::remove_file(&test_file2);

    Ok(())
}

async fn run_write_test(
    test_file: &std::path::Path,
    test_file_str: &str,
    options: ClaudeAgentOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Test file path: {test_file_str}");
    println!("File exists before: {}\n", test_file.exists());

    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Ask Claude to write a file
    let prompt = format!(
        "Use the Write tool to create a file at exactly this path: {test_file_str}\n\
         Write this exact content: HELLO FROM SDK TEST\n\
         Do not use any other tools. Just Write."
    );

    println!("Sending prompt...\n");
    client.send_message(&prompt).await?;

    while let Some(message) = client.next_message().await {
        match message? {
            Message::Assistant { message, .. } => {
                for block in &message.content {
                    match block {
                        ContentBlock::Text { text } => {
                            println!("[Text] {text}");
                        }
                        ContentBlock::ToolUse { name, input, .. } => {
                            println!("[ToolUse] {name} with {input:?}");
                        }
                        ContentBlock::ToolResult {
                            content, is_error, ..
                        } => {
                            println!("[ToolResult] error={is_error:?} content={content:?}");
                        }
                        _ => {}
                    }
                }
            }
            Message::User { .. } => {
                println!("[User message - tool result returned]");
            }
            Message::Result { .. } => {
                println!("\n[Result] Session complete");
                break;
            }
            _ => {}
        }
    }

    client.close().await?;

    // Check if file was actually written
    println!("\n=== Verification ===");
    println!("File exists after: {}", test_file.exists());

    if test_file.exists() {
        let content = std::fs::read_to_string(test_file)?;
        println!("File content: {content}");
        println!("\nWrite tool WORKS!");
    } else {
        println!("\nWrite tool did NOT write the file!");
    }

    // Cleanup
    let _ = std::fs::remove_file(test_file);

    Ok(())
}
