//! Interactive REPL client example
//!
//! This example demonstrates a true interactive conversation with Claude.
//! Type your messages and press Enter to send. Type 'quit' or 'exit' to end.
//!
//! Run with: cargo run --example `interactive_client`

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message};
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════╗");
    println!("║     Claude Interactive REPL                ║");
    println!("╠════════════════════════════════════════════╣");
    println!("║  Type your message and press Enter         ║");
    println!("║  Commands: 'quit' or 'exit' to end         ║");
    println!("╚════════════════════════════════════════════╝");
    println!();

    // Create client with reasonable limits
    let options = ClaudeAgentOptions::builder().max_turns(50).build();

    let mut client = ClaudeSDKClient::new(options, None).await?;
    println!("✓ Connected to Claude\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // Prompt for input
        print!("You: ");
        stdout.flush()?;

        // Read user input
        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();

        // Check for exit commands
        if input.is_empty() {
            continue;
        }
        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            println!("\nGoodbye!");
            break;
        }

        // Send message to Claude
        client.send_message(input).await?;

        // Read and display response
        print!("\nClaude: ");
        stdout.flush()?;

        while let Some(message) = client.next_message().await {
            match message {
                Ok(msg) => match msg {
                    Message::Assistant { message, .. } => {
                        for block in &message.content {
                            if let ContentBlock::Text { text } = block {
                                print!("{text}");
                                stdout.flush()?;
                            }
                        }
                    }
                    Message::Result { total_cost_usd, .. } => {
                        println!();
                        if let Some(cost) = total_cost_usd {
                            println!("  [cost: ${cost:.4}]");
                        }
                        println!();
                        break;
                    }
                    _ => {}
                },
                Err(e) => {
                    eprintln!("\nError: {e}");
                    break;
                }
            }
        }
    }

    // Cleanup
    client.close().await?;
    println!("Session closed.");

    Ok(())
}
