use std::collections::HashMap;
use std::sync::Arc;

use zero_llm::ModelProvider;
use zero_tools::ToolRegistry;

use crate::{AgentConfig, SubAgent, TaskInfo, TaskStatus};

/// Coordinates multiple sub-agents working on related tasks.
pub struct Coordinator {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    workers: Vec<SubAgent>,
    tasks: Vec<TaskInfo>,
    /// Task id → (agent config, run prompt).
    specs: HashMap<String, (AgentConfig, String)>,
    provider: Arc<dyn ModelProvider>,
    tools: Arc<ToolRegistry>,
}

impl Coordinator {
    pub fn new(
        name: String,
        provider: Arc<dyn ModelProvider>,
        tools: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            name,
            workers: Vec::new(),
            tasks: Vec::new(),
            specs: HashMap::new(),
            provider,
            tools,
        }
    }

    /// Add a task to be worked on by a sub-agent. Returns the task id.
    pub fn add_task(
        &mut self,
        config: AgentConfig,
        description: String,
        prompt: String,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.tasks.push(TaskInfo::new(id.clone(), description));
        self.specs.insert(id.clone(), (config, prompt));
        id
    }

    /// Run all pending tasks concurrently, returning updated task snapshots.
    pub async fn run_all(&mut self) -> Vec<TaskInfo> {
        let pending_ids: Vec<String> = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .map(|t| t.id.clone())
            .collect();

        let mut handles = Vec::new();
        for id in pending_ids {
            let Some((config, prompt)) = self.specs.get(&id).cloned() else {
                continue;
            };
            let description = self
                .tasks
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.description.clone())
                .unwrap_or_default();

            let provider = self.provider.clone();
            let tools = self.tools.clone();

            handles.push(tokio::spawn(async move {
                let mut agent = SubAgent::new(config, description);
                let _ = agent.run(&prompt, provider, tools).await;
                agent.task().clone()
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(task) => {
                    if let Some(slot) = self.tasks.iter_mut().find(|t| t.id == task.id) {
                        *slot = task.clone();
                    }
                    results.push(task);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "coordinator sub-task join failed");
                }
            }
        }

        results
    }

    pub fn tasks(&self) -> &[TaskInfo] {
        &self.tasks
    }
}
