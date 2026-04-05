use std::sync::Arc;

use zero_core::{AgentEvent, EngineConfig, QueryEngine};
use zero_llm::ModelProvider;
use zero_tools::ToolRegistry;

use crate::{AgentConfig, TaskInfo, TaskStatus};

/// A sub-agent with its own isolated context.
pub struct SubAgent {
    config: AgentConfig,
    task: TaskInfo,
}

impl SubAgent {
    pub fn new(config: AgentConfig, task_description: String) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            config,
            task: TaskInfo::new(id, task_description),
        }
    }

    /// Run the sub-agent with isolated context. Returns the final text result.
    pub async fn run(
        &mut self,
        prompt: &str,
        provider: Arc<dyn ModelProvider>,
        tools: Arc<ToolRegistry>,
    ) -> Result<String, SubAgentError> {
        self.task.status = TaskStatus::Running;

        let engine_config = EngineConfig {
            model: self.config.model.clone(),
            system_prompt: self.config.system_prompt.clone(),
            max_turns: self.config.max_turns,
            ..Default::default()
        };

        let mut engine = QueryEngine::new(provider, tools, engine_config);
        let (tx, mut rx) = tokio::sync::mpsc::channel(256);

        let prompt_owned = prompt.to_string();
        let run_handle = tokio::spawn(async move { engine.run(&prompt_owned, tx).await });

        let mut output = String::new();
        while let Some(event) = rx.recv().await {
            if let AgentEvent::TextDelta(t) = event {
                output.push_str(&t);
            }
        }

        match run_handle.await {
            Ok(Ok(())) => {
                self.task.complete(output.clone());
                Ok(output)
            }
            Ok(Err(e)) => {
                let msg = e.to_string();
                self.task.fail(msg.clone());
                Err(SubAgentError::EngineFailed(msg))
            }
            Err(e) => {
                let msg = e.to_string();
                self.task.fail(msg.clone());
                Err(SubAgentError::TaskPanicked(msg))
            }
        }
    }

    pub fn task(&self) -> &TaskInfo {
        &self.task
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SubAgentError {
    #[error("Engine failed: {0}")]
    EngineFailed(String),
    #[error("Task panicked: {0}")]
    TaskPanicked(String),
}
