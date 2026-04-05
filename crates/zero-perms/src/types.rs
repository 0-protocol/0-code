use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionMode {
    /// Ask for all write operations.
    Default,
    /// Auto-allow file edits in cwd.
    AcceptEdits,
    /// Pause for review before any action.
    Plan,
    /// Allow everything (dangerous).
    Bypass,
    /// LLM classifier decides.
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny,
    /// Need to prompt user.
    Ask,
}

#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub tool_name: String,
    pub description: String,
    pub is_read_only: bool,
    pub working_directory: Option<String>,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct PermissionContext {
    pub mode: PermissionMode,
    pub cwd: String,
    pub denied_count: u32,
    pub consecutive_denials: u32,
}
