//! File Checkpointing Rewind Test
//!
//! Tests file checkpointing and rewind functionality:
//! 1. Have Claude create a file with content "ORIGINAL"
//! 2. Capture the checkpoint UUID from the user message
//! 3. Have Claude modify the file to "MODIFIED"
//! 4. Call rewind_files() with the checkpoint UUID
//! 5. Verify file state
//!
//! TODO: Investigate rewind protocol - CLI acknowledges but file revert needs verification
//!
//! Run with: RUST_LOG=debug cargo run --example checkpointing_rewind_test

use anthropic_agent_sdk::{
    ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message, PermissionMode,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "anthropic_agent_sdk=debug".parse().unwrap()),
        )
        .init();

    println!("=== File Checkpointing Rewind Test ===\n");

    // Create temp directory for test
    let temp_dir = std::env::temp_dir().join("claude_rewind_test");
    std::fs::create_dir_all(&temp_dir)?;
    let test_file = temp_dir.join("test_file.txt");

    // Clean up any previous test file
    if test_file.exists() {
        std::fs::remove_file(&test_file)?;
    }

    println!("Test directory: {}", temp_dir.display());
    println!("Test file: {}\n", test_file.display());

    let options = ClaudeAgentOptions::builder()
        .enable_file_checkpointing(true)
        .cwd(temp_dir.clone())
        .model("haiku")
        .max_turns(5u32)
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Step 1: Create file with ORIGINAL content
    println!("--- Step 1: Creating file with ORIGINAL content ---");
    let create_prompt = format!(
        "Use the Write tool to create a file at {} with the exact content: ORIGINAL\n\
         Do not add any other text. Just the word ORIGINAL.",
        test_file.display()
    );

    client.send_message(&create_prompt).await?;

    let mut checkpoint_uuid: Option<String> = None;

    loop {
        match tokio::time::timeout(Duration::from_secs(60), client.next_message()).await {
            Ok(Some(Ok(msg))) => {
                match &msg {
                    Message::User { uuid: Some(u), .. } => {
                        println!("[CHECKPOINT] UUID: {}", u);
                        // Only capture the FIRST UUID - checkpoint before any file ops
                        if checkpoint_uuid.is_none() {
                            checkpoint_uuid = Some(u.clone());
                        }
                    }
                    Message::User { uuid: None, .. } => {}
                    Message::Assistant { message, .. } => {
                        for block in &message.content {
                            match block {
                                ContentBlock::Text { text } => println!("[ASSISTANT] {}", text),
                                ContentBlock::ToolUse { name, .. } => println!("[TOOL] {}", name),
                                _ => {}
                            }
                        }
                    }
                    Message::Result { subtype, .. } => {
                        println!("[RESULT] {}", subtype);
                        break;
                    }
                    _ => {}
                }
            }
            Ok(Some(Err(e))) => {
                eprintln!("[ERROR] {}", e);
                break;
            }
            Ok(None) => break,
            Err(_) => {
                eprintln!("[TIMEOUT]");
                break;
            }
        }
    }

    // Verify file was created
    if !test_file.exists() {
        eprintln!("\n[FAIL] File was not created!");
        client.close().await?;
        return Ok(());
    }

    let content_after_create = std::fs::read_to_string(&test_file)?;
    println!(
        "\n[VERIFY] File content after create: {:?}",
        content_after_create.trim()
    );

    if !content_after_create.contains("ORIGINAL") {
        eprintln!("[FAIL] Expected ORIGINAL, got: {}", content_after_create);
        client.close().await?;
        return Ok(());
    }
    println!("[PASS] File contains ORIGINAL\n");

    // Step 2: Modify file to MODIFIED content
    println!("--- Step 2: Modifying file to MODIFIED content ---");
    let modify_prompt = format!(
        "Use the Write tool to overwrite the file at {} with the exact content: MODIFIED\n\
         Do not add any other text. Just the word MODIFIED.",
        test_file.display()
    );

    client.send_message(&modify_prompt).await?;

    loop {
        match tokio::time::timeout(Duration::from_secs(60), client.next_message()).await {
            Ok(Some(Ok(msg))) => match &msg {
                Message::Assistant { message, .. } => {
                    for block in &message.content {
                        match block {
                            ContentBlock::Text { text } => println!("[ASSISTANT] {}", text),
                            ContentBlock::ToolUse { name, .. } => println!("[TOOL] {}", name),
                            _ => {}
                        }
                    }
                }
                Message::Result { subtype, .. } => {
                    println!("[RESULT] {}", subtype);
                    break;
                }
                _ => {}
            },
            Ok(Some(Err(e))) => {
                eprintln!("[ERROR] {}", e);
                break;
            }
            Ok(None) => break,
            Err(_) => {
                eprintln!("[TIMEOUT]");
                break;
            }
        }
    }

    let content_after_modify = std::fs::read_to_string(&test_file)?;
    println!(
        "\n[VERIFY] File content after modify: {:?}",
        content_after_modify.trim()
    );

    if !content_after_modify.contains("MODIFIED") {
        eprintln!("[FAIL] Expected MODIFIED, got: {}", content_after_modify);
        client.close().await?;
        return Ok(());
    }
    println!("[PASS] File contains MODIFIED\n");

    // Step 3: Rewind to checkpoint using control request
    println!("--- Step 3: Rewinding to checkpoint ---");

    if let Some(uuid) = &checkpoint_uuid {
        println!("Rewinding to UUID: {} (via control request)", uuid);

        match client.rewind_files(uuid).await {
            Ok(()) => {
                println!("[OK] Rewind control request sent");
                // Read any response that comes back
                println!("[INFO] Reading CLI response...");
                loop {
                    match tokio::time::timeout(Duration::from_secs(3), client.next_message()).await
                    {
                        Ok(Some(Ok(msg))) => {
                            println!("[RAW RESPONSE] {:?}", msg);
                        }
                        Ok(Some(Err(e))) => {
                            println!("[RAW ERROR] {:?}", e);
                            break;
                        }
                        Ok(None) => {
                            println!("[RAW] Stream ended");
                            break;
                        }
                        Err(_) => {
                            println!("[RAW] Timeout - no more messages");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[ERROR] Rewind failed: {}", e);
                client.close().await?;
                return Ok(());
            }
        }
    } else {
        eprintln!("[FAIL] No checkpoint UUID captured!");
        client.close().await?;
        return Ok(());
    }

    // Step 4: Verify file was restored
    println!("\n--- Step 4: Verifying file was restored ---");

    let content_after_rewind = std::fs::read_to_string(&test_file)?;
    println!(
        "[VERIFY] File content after rewind: {:?}",
        content_after_rewind.trim()
    );

    if content_after_rewind.contains("ORIGINAL") {
        println!("\n=== TEST PASSED ===");
        println!("File was successfully restored from MODIFIED to ORIGINAL!");
    } else if content_after_rewind.contains("MODIFIED") {
        println!("\n=== TEST FAILED ===");
        println!("File still contains MODIFIED - rewind did not work.");
        println!("Possible reasons:");
        println!("  - CLI may not support rewind_files control method yet");
        println!("  - File checkpointing may require different protocol format");
    } else {
        println!("\n=== TEST INCONCLUSIVE ===");
        println!("File content: {}", content_after_rewind);
    }

    client.close().await?;

    // Cleanup
    if test_file.exists() {
        std::fs::remove_file(&test_file)?;
    }

    Ok(())
}
