use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use tracing::{debug, error, warn};

use crate::tool::Tool;
use crate::types::{ToolCall, ToolResult};

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    max_concurrent: usize,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            max_concurrent: 8,
        }
    }

    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        debug!(tool = %name, "Registering tool");
        self.tools.insert(name, tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn list_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.tools.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Returns tool definitions in the format expected by LLM APIs.
    pub fn list_definitions(&self) -> Vec<serde_json::Value> {
        let mut defs: Vec<serde_json::Value> = self
            .tools
            .values()
            .map(|tool| {
                json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "input_schema": tool.input_schema(),
                })
            })
            .collect();
        defs.sort_by(|a, b| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or(""))
        });
        defs
    }

    /// Execute a batch of tool calls, respecting concurrency rules:
    /// - Read-only tools run concurrently (up to max_concurrent)
    /// - Write tools run serially
    pub async fn execute_batch(&self, calls: Vec<ToolCall>) -> Vec<(String, ToolResult)> {
        let mut read_calls = Vec::new();
        let mut write_calls = Vec::new();

        for call in &calls {
            match self.tools.get(&call.name) {
                Some(tool) if tool.is_concurrency_safe() => read_calls.push(call),
                _ => write_calls.push(call),
            }
        }

        let mut results: Vec<(String, ToolResult)> = Vec::new();

        // Execute read-only calls concurrently via JoinSet
        if !read_calls.is_empty() {
            let mut join_set = tokio::task::JoinSet::new();
            for call in read_calls {
                let tool = self.tools.get(&call.name).cloned();
                let call_id = call.id.clone();
                let call_name = call.name.clone();
                let input = call.input.clone();
                let _permit = self.max_concurrent; // captured for documentation; real semaphore is the JoinSet size

                join_set.spawn(async move {
                    let result = match tool {
                        Some(t) => match t.call(input).await {
                            Ok(r) => r,
                            Err(e) => ToolResult::error(e.to_string()),
                        },
                        None => ToolResult::error(format!("Tool not found: {call_name}")),
                    };
                    (call_id, result)
                });

                // Limit concurrency by awaiting when at capacity
                if join_set.len() >= self.max_concurrent {
                    if let Some(Ok(pair)) = join_set.join_next().await {
                        results.push(pair);
                    }
                }
            }
            while let Some(Ok(pair)) = join_set.join_next().await {
                results.push(pair);
            }
        }

        // Execute write calls serially
        for call in write_calls {
            let result = self.execute(call).await;
            results.push((call.id.clone(), result));
        }

        results
    }

    /// Execute a single tool call.
    pub async fn execute(&self, call: &ToolCall) -> ToolResult {
        let tool = match self.tools.get(&call.name) {
            Some(t) => t,
            None => {
                warn!(tool = %call.name, "Tool not found");
                return ToolResult::error(format!("Tool not found: {}", call.name));
            }
        };

        debug!(tool = %call.name, id = %call.id, "Executing tool");

        match tool.call(call.input.clone()).await {
            Ok(result) => result,
            Err(e) => {
                error!(tool = %call.name, error = %e, "Tool execution failed");
                ToolResult::error(e.to_string())
            }
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
