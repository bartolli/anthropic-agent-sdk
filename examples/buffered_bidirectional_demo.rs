//! Buffered Bidirectional Communication Demo
//!
//! Demonstrates a workaround for CLI's streaming limitation using a message buffer.
//!
//! ## The Problem
//!
//! CLI only reads stdin between turns, not during streaming.
//! Messages sent during streaming are ignored.
//!
//! ## The Solution: Message Buffer
//!
//! 1. User can "send" messages anytime (they go to a buffer)
//! 2. When Result arrives, buffered messages are sent to CLI
//! 3. Conversation continues with context preserved
//!
//! This creates the illusion of bidirectional communication while
//! respecting CLI's turn-based architecture.
//!
//! Run with: `RUST_LOG=info` cargo run --example `buffered_bidirectional_demo`

use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message};
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Message buffer that queues messages and sends them when CLI is ready
struct MessageBuffer {
    /// Queued messages waiting to be sent
    pending: VecDeque<String>,
    /// Channel to receive new messages from "user"
    rx: mpsc::UnboundedReceiver<String>,
}

impl MessageBuffer {
    fn new(rx: mpsc::UnboundedReceiver<String>) -> Self {
        Self {
            pending: VecDeque::new(),
            rx,
        }
    }

    /// Check for new messages and add to queue (non-blocking)
    fn poll(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            info!(message = %msg, "Buffered message (will send after current turn)");
            self.pending.push_back(msg);
        }
    }

    /// Get next message to send, if any
    fn next(&mut self) -> Option<String> {
        self.pending.pop_front()
    }

    /// Check if buffer has pending messages
    fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Number of pending messages
    fn len(&self) -> usize {
        self.pending.len()
    }
}

/// Handle that allows "sending" messages to the buffer from anywhere
#[derive(Clone)]
struct BufferHandle {
    tx: mpsc::UnboundedSender<String>,
}

impl BufferHandle {
    /// Queue a message (will be sent when CLI is ready)
    fn send(&self, message: impl Into<String>) -> bool {
        self.tx.send(message.into()).is_ok()
    }
}

/// Create a message buffer and its handle
fn create_buffer() -> (MessageBuffer, BufferHandle) {
    let (tx, rx) = mpsc::unbounded_channel();
    (MessageBuffer::new(rx), BufferHandle { tx })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("=== Buffered Bidirectional Demo ===");
    info!("Messages sent during streaming are buffered and sent after each turn\n");

    // Create the message buffer
    let (mut buffer, handle) = create_buffer();

    // Create client with enough turns for multi-message conversation
    let options = ClaudeAgentOptions::builder().max_turns(10).build();

    let mut client = ClaudeSDKClient::new(options, None).await?;
    info!("Client connected\n");

    // Send initial message
    info!("--- Sending initial question ---");
    client
        .send_message("What is Python? Answer in one sentence.")
        .await?;

    // Simulate user sending messages "during" streaming
    // In a real app, this could come from user input, webhooks, etc.
    let handle_clone = handle.clone();
    tokio::spawn(async move {
        // Wait a bit to simulate user typing while Claude responds
        tokio::time::sleep(Duration::from_millis(100)).await;
        handle_clone.send("What is TypeScript? Answer in one sentence.");

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle_clone.send("Now compare Rust to both. One sentence.");
    });

    let mut turn = 0;

    loop {
        turn += 1;
        info!("\n--- Turn {turn} ---");

        // Read messages until Result
        let mut got_result = false;

        loop {
            // Poll buffer for new messages (non-blocking)
            buffer.poll();

            match tokio::time::timeout(Duration::from_secs(30), client.next_message()).await {
                Ok(Some(message)) => match message? {
                    Message::Assistant { message, .. } => {
                        info!("[Claude]");
                        for block in &message.content {
                            if let ContentBlock::Text { text } = block {
                                println!("{text}");
                            }
                        }
                    }
                    Message::Result {
                        session_id,
                        num_turns,
                        ..
                    } => {
                        info!(
                            session_id = %session_id,
                            num_turns,
                            buffered = buffer.len(),
                            "[Result] Turn complete"
                        );
                        got_result = true;
                        break;
                    }
                    _ => {}
                },
                Ok(None) => {
                    warn!("Stream ended unexpectedly");
                    break;
                }
                Err(_) => {
                    warn!("Timeout waiting for response");
                    break;
                }
            }
        }

        if !got_result {
            break;
        }

        // Check buffer for pending messages
        buffer.poll();

        if buffer.has_pending() {
            info!("Buffer has {} pending message(s)", buffer.len());
        }

        if let Some(next_msg) = buffer.next() {
            info!(
                remaining = buffer.len(),
                "\n--- Sending buffered message ---"
            );
            info!(message = %next_msg, "[User]");
            client.send_message(&next_msg).await?;
        } else {
            info!("No more buffered messages, conversation complete");
            break;
        }
    }

    client.close().await?;

    info!("\n=== Demo Complete ===");
    info!("Total turns: {turn}");
    info!("Key insight: Messages were buffered during streaming, sent between turns");
    info!("Result: Claude's final answer references Python and TypeScript from context!");

    Ok(())
}
