//! Skill system for loading and matching agent skills.

mod loader;
mod matcher;
mod types;

pub use loader::SkillLoader;
pub use matcher::SkillMatcher;
pub use types::{ExecutionMode, Skill, SkillSource};
