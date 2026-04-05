//! Multi-agent orchestration for the 0-code agent: [`SubAgent`], [`Coordinator`], and [`Team`].

pub mod agent_tool;
pub mod coordinator;
pub mod sub_agent;
pub mod team;
pub mod types;

pub use agent_tool::AgentTool;
pub use coordinator::Coordinator;
pub use sub_agent::{SubAgent, SubAgentError};
pub use team::{Team, TeamError, TeamMember, TeamMessage};
pub use types::{AgentConfig, TaskInfo, TaskStatus};
