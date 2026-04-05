use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifecycleEvent {
    SessionStart,
    PreToolUse,
    PostToolUse,
    Stop,
    SubagentStart,
    SubagentStop,
    PermissionDenied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub event: LifecycleEvent,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub tool_result: Option<String>,
    pub session_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookResult {
    Continue,
    Block,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub name: String,
    pub event: LifecycleEvent,
    pub hook_type: HookType,
    pub command: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookType {
    Command,
    Prompt,
    Function,
}
