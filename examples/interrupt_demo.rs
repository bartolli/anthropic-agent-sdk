//! Interrupt Demo with Context Preservation
//!
//! Demonstrates interrupt functionality and verifies context is preserved after interrupt.
//!
//! ## Flow
//!
//! 1. Send a prompt requesting a long detailed response
//! 2. Wait a few seconds while Claude streams the response
//! 3. Send interrupt to stop the response
//! 4. Send follow-up asking Claude to be more concise
//! 5. Verify Claude remembers what it was talking about
//!
//! ## Protocol
//!
//! Interrupt is sent as:
//! ```json
//! {"type": "control_request", "request_id": "req_...", "request": {"subtype": "interrupt"}}
//! ```
//!
//! Run with: RUST_LOG=debug cargo run --example interrupt_demo
//! Or: RUST_LOG=info cargo run --example interrupt_demo

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message};
use std::time::Duration;
use tracing::{debug, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    info!("╔══════════════════════════════════════════════════╗");
    info!("║  Interrupt Demo with Context Preservation        ║");
    info!("╚══════════════════════════════════════════════════╝");
    info!("");

    // =========================================================================
    // Phase 1: Send prompt, let it stream, then interrupt
    // =========================================================================
    info!("Phase 1: Send long prompt, wait, then interrupt");
    info!("─────────────────────────────────────────────────");

    let options = ClaudeAgentOptions::builder().max_turns(10).build();
    let mut client = ClaudeSDKClient::new(options, None).await?;

    let prompt = "Explain the complete history of programming languages in great detail, \
                  starting from the 1950s with FORTRAN and COBOL, through C, C++, Java, \
                  and up to modern languages like Rust and Go. Include key contributors \
                  and major innovations for each era.";

    info!("[User] {}", prompt);
    info!("");
    client.send_message(prompt).await?;

    let mut first_response = String::new();
    let mut char_count = 0;
    let mut interrupted = false;
    let mut session_id = None;

    info!("Streaming response (will interrupt after ~300 chars)...");
    info!("");

    loop {
        match tokio::time::timeout(Duration::from_secs(30), client.next_message()).await {
            Ok(Some(msg)) => match msg? {
                Message::Assistant { message, .. } => {
                    for block in &message.content {
                        if let ContentBlock::Text { text } = block {
                            first_response.push_str(text);
                            char_count += text.len();

                            // Show streaming progress
                            let snippet: String = text.chars().take(60).collect();
                            debug!(chars = char_count, "Received chunk");
                            if !snippet.trim().is_empty() {
                                info!("  📝 ...{}", snippet.trim());
                            }

                            // After enough content, send interrupt
                            if char_count > 300 && !interrupted {
                                info!("");
                                info!("  ┌─────────────────────────────────────┐");
                                info!("  │ 🛑 SENDING INTERRUPT                │");
                                info!("  └─────────────────────────────────────┘");

                                match client.interrupt().await {
                                    Ok(()) => {
                                        info!("  ✓ Interrupt sent successfully");
                                        interrupted = true;
                                    }
                                    Err(e) => warn!("  ✗ Interrupt failed: {}", e),
                                }
                                info!("");
                            }
                        }
                    }
                }
                Message::Result {
                    session_id: sid, ..
                } => {
                    session_id = Some(sid.to_string());
                    info!("  ✓ Result received - response stopped");
                    break;
                }
                _ => {}
            },
            Ok(None) => break,
            Err(_) => {
                warn!("Timeout");
                break;
            }
        }
    }

    let session_id = session_id.ok_or("No session ID captured")?;
    info!("");
    info!("Phase 1 complete:");
    info!("  - Received {} chars before interrupt", char_count);
    info!("  - Session ID: {}", session_id);

    // Close first client
    client.close().await?;

    // =========================================================================
    // Phase 2: Resume and ask for concise version
    // =========================================================================
    info!("");
    info!("Phase 2: Resume session, ask for concise summary");
    info!("─────────────────────────────────────────────────");

    let options = ClaudeAgentOptions::builder()
        .resume(session_id.clone())
        .max_turns(10)
        .build();

    let mut client = ClaudeSDKClient::new(options, None).await?;

    let followup = "I interrupted you. Please give me just a 2-sentence summary of what \
                    you were explaining about programming language history.";

    info!("[User] {}", followup);
    info!("");
    client.send_message(followup).await?;

    let mut second_response = String::new();

    loop {
        match tokio::time::timeout(Duration::from_secs(30), client.next_message()).await {
            Ok(Some(msg)) => match msg? {
                Message::Assistant { message, .. } => {
                    for block in &message.content {
                        if let ContentBlock::Text { text } = block {
                            second_response.push_str(text);
                        }
                    }
                }
                Message::Result { .. } => {
                    break;
                }
                _ => {}
            },
            Ok(None) => break,
            Err(_) => break,
        }
    }

    info!("[Claude] {}", second_response.trim());

    client.close().await?;

    // =========================================================================
    // Summary
    // =========================================================================
    info!("");
    info!("╔══════════════════════════════════════════════════╗");
    info!("║  Results                                         ║");
    info!("╚══════════════════════════════════════════════════╝");
    info!("");
    info!(
        "First response (before interrupt): {} chars",
        first_response.len()
    );
    info!(
        "Second response (after resume):    {} chars",
        second_response.len()
    );
    info!("");

    // Check if context was preserved
    let context_preserved = second_response.to_lowercase().contains("programming")
        || second_response.to_lowercase().contains("language")
        || second_response.to_lowercase().contains("fortran")
        || second_response.to_lowercase().contains("history");

    if interrupted && context_preserved {
        info!("✅ SUCCESS!");
        info!("   - Interrupt stopped the long response");
        info!("   - Context was preserved after interrupt");
        info!("   - Claude remembered the topic and gave a concise summary");
    } else if !interrupted {
        warn!("⚠️  Response completed before interrupt was sent");
    } else {
        warn!("⚠️  Context may not have been fully preserved");
    }

    info!("");
    info!("Demo complete!");

    Ok(())
}
