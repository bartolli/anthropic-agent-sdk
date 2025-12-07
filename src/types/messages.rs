//! Message types for conversations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::identifiers::SessionId;
use super::introspection::{ModelUsage, SDKPermissionDenial};

// ============================================================================
// Message Types
// ============================================================================

/// Content value for tool results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentValue {
    /// String content
    String(String),
    /// Structured content blocks
    Blocks(Vec<serde_json::Value>),
}

/// Content block types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content block
    Text {
        /// Text content
        text: String,
    },
    /// Thinking content block (extended thinking)
    Thinking {
        /// Thinking content
        thinking: String,
        /// Signature for verification
        signature: String,
    },
    /// Tool use request
    ToolUse {
        /// Tool use ID
        id: String,
        /// Tool name
        name: String,
        /// Tool input parameters
        input: serde_json::Value,
    },
    /// Tool execution result
    ToolResult {
        /// ID of the tool use this is a result for
        tool_use_id: String,
        /// Result content
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ContentValue>,
        /// Whether this is an error result
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

/// User message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageContent {
    /// Message role (always "user")
    pub role: String,
    /// Message content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<UserContent>,
}

/// User content can be string or blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserContent {
    /// Plain string content
    String(String),
    /// Structured content blocks
    Blocks(Vec<ContentBlock>),
}

/// Assistant message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageContent {
    /// Model that generated the message
    pub model: String,
    /// Message content blocks
    pub content: Vec<ContentBlock>,
}

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// User message
    User {
        /// Parent tool use ID for nested conversations
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_tool_use_id: Option<String>,
        /// Message content
        message: UserMessageContent,
        /// Session ID
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<SessionId>,
    },
    /// Assistant message
    Assistant {
        /// Parent tool use ID for nested conversations
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_tool_use_id: Option<String>,
        /// Message content
        message: AssistantMessageContent,
        /// Session ID
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<SessionId>,
    },
    /// System message
    System {
        /// System message subtype
        subtype: String,
        /// Additional system message data
        #[serde(flatten)]
        data: serde_json::Value,
    },
    /// Result message with metrics
    Result {
        /// Result subtype (success, `error_max_turns`, `error_during_execution`, etc.)
        subtype: String,
        /// Total duration in milliseconds
        duration_ms: u64,
        /// API call duration in milliseconds
        duration_api_ms: u64,
        /// Whether this is an error result
        is_error: bool,
        /// Number of conversation turns
        num_turns: u32,
        /// Session ID
        session_id: SessionId,
        /// Total cost in USD
        #[serde(skip_serializing_if = "Option::is_none")]
        total_cost_usd: Option<f64>,
        /// Token usage statistics (aggregate)
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<serde_json::Value>,
        /// Result message (for success subtype)
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<String>,
        /// Per-model usage statistics
        #[serde(
            rename = "modelUsage",
            default,
            skip_serializing_if = "HashMap::is_empty"
        )]
        model_usage: HashMap<String, ModelUsage>,
        /// List of denied tool uses
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        permission_denials: Vec<SDKPermissionDenial>,
        /// Structured output (when outputFormat is specified)
        #[serde(skip_serializing_if = "Option::is_none")]
        structured_output: Option<serde_json::Value>,
        /// Error messages (for error subtypes)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        errors: Vec<String>,
    },
    /// Stream event for partial messages
    StreamEvent {
        /// Event UUID
        uuid: String,
        /// Session ID
        session_id: SessionId,
        /// Raw stream event data
        event: serde_json::Value,
        /// Parent tool use ID
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_tool_use_id: Option<String>,
    },
}
