//! Hook types for event handling

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

// ============================================================================
// Hook Types
// ============================================================================

/// Hook event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    /// Before a tool is used
    PreToolUse,
    /// After a tool is used
    PostToolUse,
    /// After a tool use fails
    PostToolUseFailure,
    /// When a notification is received
    Notification,
    /// When user submits a prompt
    UserPromptSubmit,
    /// When a session starts
    SessionStart,
    /// When a session ends
    SessionEnd,
    /// When conversation stops
    Stop,
    /// When a subagent starts
    SubagentStart,
    /// When a subagent stops
    SubagentStop,
    /// Before compacting the conversation
    PreCompact,
    /// When a permission is requested
    PermissionRequest,
}

// ============================================================================
// Hook Input Types
// ============================================================================

/// Base fields common to all hook inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseHookInput {
    /// Session identifier
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (if set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
}

/// Input for `SubagentStart` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStartHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Agent identifier
    pub agent_id: String,
    /// Agent type (e.g., "knowledgeBuilder", "codeReviewer")
    pub agent_type: String,
}

/// Input for `SubagentStop` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStopHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Whether stop hook is active
    pub stop_hook_active: bool,
    /// Agent identifier
    pub agent_id: String,
    /// Path to the agent's transcript
    pub agent_transcript_path: String,
}

/// Input for `PreToolUse` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUseHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Name of the tool being used
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
}

/// Input for `PostToolUse` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Name of the tool that was used
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
    /// Tool response/output
    pub tool_response: serde_json::Value,
    /// Tool use identifier
    pub tool_use_id: String,
}

/// Input for `PostToolUseFailure` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseFailureHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Name of the tool that failed
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
    /// Tool use identifier
    pub tool_use_id: String,
    /// Error message
    pub error: String,
    /// Whether this was an interrupt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_interrupt: Option<bool>,
}

/// Input for `SessionStart` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Source of session start
    pub source: SessionStartSource,
}

/// Source of session start
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStartSource {
    /// Fresh startup
    Startup,
    /// Resumed session
    Resume,
    /// After clear
    Clear,
    /// After compact
    Compact,
}

/// Input for `SessionEnd` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Reason for session end
    pub reason: SessionEndReason,
}

/// Reason for session end
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    /// Session cleared
    Clear,
    /// User logged out
    Logout,
    /// User exited prompt input
    PromptInputExit,
    /// Other reason
    Other,
}

/// Input for Stop hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Whether stop hook is active
    pub stop_hook_active: bool,
}

/// Input for `UserPromptSubmit` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptSubmitHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// The submitted prompt
    pub prompt: String,
}

/// Input for Notification hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Notification message
    pub message: String,
    /// Optional title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Input for `PreCompact` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCompactHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Trigger type
    pub trigger: CompactTrigger,
    /// Custom instructions for compaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,
}

/// Trigger type for compaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactTrigger {
    /// Manually triggered
    Manual,
    /// Automatically triggered
    Auto,
}

/// Input for `PermissionRequest` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequestHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Name of the tool requesting permission
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
    /// Permission suggestions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_suggestions: Option<Vec<serde_json::Value>>,
}

/// Union type for all hook inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name")]
pub enum HookInput {
    /// Pre tool use input
    PreToolUse(PreToolUseHookInput),
    /// Post tool use input
    PostToolUse(PostToolUseHookInput),
    /// Post tool use failure input
    PostToolUseFailure(PostToolUseFailureHookInput),
    /// Notification input
    Notification(NotificationHookInput),
    /// User prompt submit input
    UserPromptSubmit(UserPromptSubmitHookInput),
    /// Session start input
    SessionStart(SessionStartHookInput),
    /// Session end input
    SessionEnd(SessionEndHookInput),
    /// Stop input
    Stop(StopHookInput),
    /// Subagent start input
    SubagentStart(SubagentStartHookInput),
    /// Subagent stop input
    SubagentStop(SubagentStopHookInput),
    /// Pre compact input
    PreCompact(PreCompactHookInput),
    /// Permission request input
    PermissionRequest(PermissionRequestHookInput),
}

// ============================================================================
// Hook Output and Decision Types
// ============================================================================

/// Hook decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookDecision {
    /// Block the action
    Block,
}

/// Hook output
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookOutput {
    /// Decision to block or allow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<HookDecision>,
    /// System message to add
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemMessage")]
    pub system_message: Option<String>,
    /// Hook-specific output data
    #[serde(skip_serializing_if = "Option::is_none", rename = "hookSpecificOutput")]
    pub hook_specific_output: Option<serde_json::Value>,
}

/// Context for hook callbacks
///
/// Provides session information and cancellation support to hook callbacks.
/// Equivalent to TypeScript SDK's `{ signal: AbortSignal }` context.
#[derive(Clone, Default)]
pub struct HookContext {
    /// Session ID from the system init message
    pub session_id: Option<String>,
    /// Current working directory from the system init message
    pub cwd: Option<String>,
    /// Cancellation token for aborting operations (like `AbortSignal` in JS)
    pub cancellation_token: Option<CancellationToken>,
}

impl std::fmt::Debug for HookContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookContext")
            .field("session_id", &self.session_id)
            .field("cwd", &self.cwd)
            .field(
                "cancellation_token",
                &self.cancellation_token.as_ref().map(|_| "<token>"),
            )
            .finish()
    }
}

impl HookContext {
    /// Create a new `HookContext` with session information
    #[must_use]
    pub fn new(
        session_id: Option<String>,
        cwd: Option<String>,
        cancellation_token: Option<CancellationToken>,
    ) -> Self {
        Self {
            session_id,
            cwd,
            cancellation_token,
        }
    }

    /// Check if cancellation has been requested
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token
            .as_ref()
            .is_some_and(tokio_util::sync::CancellationToken::is_cancelled)
    }
}

/// Hook matcher configuration
#[derive(Clone)]
pub struct HookMatcher {
    /// Matcher pattern (e.g., tool name like "Bash" or pattern like "Write|Edit")
    pub matcher: Option<String>,
    /// List of hook callbacks (using the trait-based approach)
    pub hooks: Vec<Arc<dyn crate::callbacks::HookCallback>>,
    /// Timeout for all hooks in this matcher (default: 60 seconds)
    ///
    /// If a hook exceeds this timeout, it will be cancelled and a default
    /// `HookOutput` will be returned. This prevents runaway callbacks from
    /// blocking the agent indefinitely.
    pub timeout: Option<std::time::Duration>,
}

impl std::fmt::Debug for HookMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookMatcher")
            .field("matcher", &self.matcher)
            .field("hooks", &format!("[{} callbacks]", self.hooks.len()))
            .field("timeout", &self.timeout)
            .finish()
    }
}
