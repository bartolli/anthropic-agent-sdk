//! Structured Output Demo
//!
//! Demonstrates using JSON schema for guaranteed valid structured output.
//! The CLI's --json-schema flag ensures the response matches the schema.
//!
//! Run with: cargo run --example `structured_output_demo`

use anthropic_agent_sdk::{
    ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message, OutputFormat,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".parse().unwrap()),
        )
        .init();

    println!("Structured Output Demo");
    println!("======================\n");

    // Define JSON schema for the expected output
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "answer": { "type": "integer" },
            "explanation": { "type": "string" }
        },
        "required": ["answer", "explanation"]
    });

    println!("Schema: {}", serde_json::to_string_pretty(&schema)?);
    println!();

    // Create output format with schema
    let output_format = OutputFormat::json_schema(schema);
    println!("OutputFormat created:");
    println!("  format_type: {}", output_format.format_type);
    println!("  schema: {}", output_format.schema);
    println!();

    // Build options with structured output
    // Note: structured output requires at least 2 turns (one for text, one for StructuredOutput tool)
    let options = ClaudeAgentOptions::builder()
        .model("haiku")
        .output_format(output_format)
        .max_turns(5_u32)
        .build();

    // Verify output_format is set in options
    println!("Options built:");
    println!(
        "  output_format is Some: {}",
        options.output_format.is_some()
    );
    if let Some(ref of) = options.output_format {
        println!("  format_type: {}", of.format_type);
    }
    println!();

    // Create client and send query
    let mut client = ClaudeSDKClient::new(options, None).await?;

    println!("Sending query: 'What is 2 + 2?'\n");
    client.send_message("What is 2 + 2?").await?;

    // Process response
    while let Some(message) = client.next_message().await {
        match message? {
            Message::Assistant { message, .. } => {
                for block in &message.content {
                    if let ContentBlock::Text { text } = block {
                        println!("[Text] {text}");
                    }
                }
            }
            Message::Result {
                structured_output,
                num_turns,
                ..
            } => {
                println!("\n--- Result ---");
                println!("Turns: {num_turns}");

                if let Some(output) = structured_output {
                    println!("Structured output received:");
                    println!("{}", serde_json::to_string_pretty(&output)?);

                    // Access specific fields
                    if let Some(answer) = output.get("answer") {
                        println!("\nExtracted answer: {answer}");
                    }
                    if let Some(explanation) = output.get("explanation") {
                        println!("Extracted explanation: {explanation}");
                    }
                } else {
                    println!("No structured output received (None)");
                }
                break;
            }
            _ => {}
        }
    }

    client.close().await?;
    println!("\nDemo complete!");

    Ok(())
}
