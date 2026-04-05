use std::path::Path;

use async_trait::async_trait;
use globset::{Glob, GlobSetBuilder};
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::tool::{Tool, ToolError};
use crate::types::ToolResult;

pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern by recursively walking directories."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match (e.g. '**/*.rs')"
                },
                "directory": {
                    "type": "string",
                    "description": "Root directory to search in (defaults to current dir)"
                }
            },
            "required": ["pattern"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let pattern_str = input["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("'pattern' must be a string".into()))?;

        let directory = input["directory"].as_str().unwrap_or(".");

        // Auto-prepend **/ if the pattern doesn't start with it
        let full_pattern = if pattern_str.starts_with("**/") {
            pattern_str.to_string()
        } else {
            format!("**/{pattern_str}")
        };

        let glob = Glob::new(&full_pattern)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid glob pattern: {e}")))?;

        let mut builder = GlobSetBuilder::new();
        builder.add(glob);
        let glob_set = builder
            .build()
            .map_err(|e| ToolError::InvalidInput(format!("Failed to build glob set: {e}")))?;

        let root = Path::new(directory);
        if !root.is_dir() {
            return Err(ToolError::ExecutionFailed(format!(
                "Not a directory: {directory}"
            )));
        }

        // Walk directory in a blocking context since walkdir is synchronous
        let root_owned = root.to_path_buf();
        let matches = tokio::task::spawn_blocking(move || {
            let mut results = Vec::new();
            for entry in WalkDir::new(&root_owned)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let rel_path = entry
                    .path()
                    .strip_prefix(&root_owned)
                    .unwrap_or(entry.path());

                if glob_set.is_match(rel_path) {
                    results.push(entry.path().to_string_lossy().to_string());
                }
            }
            results.sort();
            results
        })
        .await
        .map_err(|e| ToolError::ExecutionFailed(format!("Task join error: {e}")))?;

        let count = matches.len();
        let content = if matches.is_empty() {
            "No files matched.".to_string()
        } else {
            matches.join("\n")
        };

        Ok(ToolResult::success(content).with_metadata(json!({ "count": count })))
    }
}
