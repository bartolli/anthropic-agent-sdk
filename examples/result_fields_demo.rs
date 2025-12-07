//! Example demonstrating the new Result message fields
//!
//! This example shows how to access:
//! - model_usage: Per-model token usage and cost breakdown
//! - permission_denials: Tools that were denied by the permission system
//! - structured_output: JSON output when using --json-schema
//! - errors: Error messages from failed operations
//!
//! Run with: cargo run --example result_fields_demo

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Result Message Fields Demo");
    println!("==========================\n");

    // Create a client with default options
    let options = ClaudeAgentOptions::builder().max_turns(3).build();

    let mut client = ClaudeSDKClient::new(options, None).await?;

    // Send a simple query that will complete and show usage
    println!("Sending query to demonstrate model_usage...\n");
    client
        .send_message("What is the capital of France? Answer in one word.")
        .await?;

    // Collect and display the response
    let mut messages = Box::pin(client.receive_response());

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant { message, .. } => {
                for block in &message.content {
                    if let ContentBlock::Text { text } = block {
                        println!("Claude: {}", text);
                    }
                }
            }
            Message::Result {
                subtype,
                session_id,
                duration_ms,
                num_turns,
                total_cost_usd,
                model_usage,
                permission_denials,
                structured_output,
                errors,
                ..
            } => {
                println!("\n--- Result Message Fields ---");
                println!("Subtype: {}", subtype);
                println!("Session ID: {}", session_id);
                println!("Duration: {}ms", duration_ms);
                println!("Turns: {}", num_turns);

                if let Some(cost) = total_cost_usd {
                    println!("Total Cost: ${:.6}", cost);
                }

                // Display model_usage (per-model token breakdown)
                if !model_usage.is_empty() {
                    println!("\n📊 Model Usage (per model):");
                    for (model_id, usage) in &model_usage {
                        println!("  Model: {}", model_id);
                        println!("    Input tokens: {}", usage.input_tokens);
                        println!("    Output tokens: {}", usage.output_tokens);
                        println!("    Cache read tokens: {}", usage.cache_read_input_tokens);
                        println!(
                            "    Cache creation tokens: {}",
                            usage.cache_creation_input_tokens
                        );
                        println!("    Total tokens: {}", usage.total_tokens());
                        println!("    Effective input: {}", usage.effective_input_tokens());
                        println!("    Cost: ${:.6}", usage.cost_usd);
                        if usage.web_search_requests > 0 {
                            println!("    Web searches: {}", usage.web_search_requests);
                        }
                    }
                } else {
                    println!("\n📊 Model Usage: (none reported)");
                }

                // Display permission_denials
                if !permission_denials.is_empty() {
                    println!("\n🚫 Permission Denials:");
                    for denial in &permission_denials {
                        println!("  Tool: {}", denial.tool_name);
                        println!("    Use ID: {}", denial.tool_use_id);
                        println!("    Input: {}", denial.tool_input);
                    }
                } else {
                    println!("\n🚫 Permission Denials: (none)");
                }

                // Display structured_output (if outputFormat was used)
                if let Some(output) = &structured_output {
                    println!("\n📋 Structured Output:");
                    println!("  {}", serde_json::to_string_pretty(output)?);
                } else {
                    println!("\n📋 Structured Output: (none - use --json-schema to enable)");
                }

                // Display errors
                if !errors.is_empty() {
                    println!("\n❌ Errors:");
                    for error in &errors {
                        println!("  - {}", error);
                    }
                } else {
                    println!("\n❌ Errors: (none)");
                }

                break;
            }
            _ => {}
        }
    }

    // Clean up
    drop(messages);
    client.close().await?;

    println!("\n✓ Demo complete!");
    Ok(())
}
