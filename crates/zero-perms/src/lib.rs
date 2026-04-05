//! Multi-layer permission pipeline for the 0-code agent.

mod manager;
mod pipeline;
mod rules;
mod tracker;
mod types;

pub use manager::PermissionManager;
pub use pipeline::{
    LlmClassifierLayer, ModeBasedLayer, PermissionLayer, PermissionPipeline, StaticRulesLayer,
};
pub use rules::{is_always_allowed_tool, is_dangerous_command, is_dangerous_file};
pub use tracker::DenialTracker;
pub use types::{PermissionContext, PermissionDecision, PermissionMode, PermissionRequest};
