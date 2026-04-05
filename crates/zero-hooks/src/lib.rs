//! Hook system for lifecycle event interception.

mod engine;
mod types;

pub use engine::HookEngine;
pub use types::{HookConfig, HookContext, HookResult, HookType, LifecycleEvent};
