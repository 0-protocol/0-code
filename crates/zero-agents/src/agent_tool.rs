use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use zero_llm::ModelProvider;
use zero_tools::{Tool, ToolError, ToolResult, ToolRegistry};

use crate::{AgentConfig, SubAgent};

/// Tool that spawns a sub-agent to handle a task.
pub struct AgentTool {
    provider: Arc<dyn ModelProvider>,
    tools: Arc<ToolRegistry>,
}

impl AgentTool {
    pub fn new(provider: Arc<dyn ModelProvider>, tools: Arc<ToolRegistry>) -> Self {
        Self { provider, tools }
    }
}

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "AgentTool"
    }

    fn description(&self) -> &str {
        "Spawn a sub-agent to work on a task autonomously"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string", "description": "Task description for the sub-agent" },
                "system_prompt": { "type": "string", "description": "Optional system prompt override" }
            },
            "required": ["prompt"]
        })
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let prompt = input["prompt"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("prompt required".into()))?;
        let system_prompt = input["system_prompt"].as_str().map(String::from);

        let config = AgentConfig {
            name: "sub-agent".into(),
            system_prompt,
            allowed_tools: None,
            max_turns: 20,
            model: "default".into(),
        };

        let mut agent = SubAgent::new(config, prompt.to_string());
        match agent
            .run(prompt, self.provider.clone(), self.tools.clone())
            .await
        {
            Ok(result) => Ok(ToolResult::success(result)),
            Err(e) => Ok(ToolResult::error(e.to_string())),
        }
    }
}
