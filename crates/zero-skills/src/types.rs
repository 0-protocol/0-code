use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub when_to_use: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub model: Option<String>,
    pub context: Option<String>,
    pub file_patterns: Vec<String>,
    pub execution_mode: ExecutionMode,
    pub content: String,
    pub source: SkillSource,
}

impl Default for Skill {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            when_to_use: None,
            allowed_tools: None,
            model: None,
            context: None,
            file_patterns: Vec::new(),
            execution_mode: ExecutionMode::Inline,
            content: String::new(),
            source: SkillSource::Filesystem,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    Inline,
    Fork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillSource {
    Bundled,
    Filesystem,
    Mcp,
}
