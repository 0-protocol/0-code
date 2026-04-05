use async_trait::async_trait;
use serde_json::{json, Value};

use crate::tool::{Tool, ToolError};
use crate::types::ToolResult;

pub struct FileEditTool;

impl FileEditTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Perform an exact string replacement in a file. The old_string must appear exactly once."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact string to find and replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement string"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let path_str = input["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("'path' must be a string".into()))?;
        let old_string = input["old_string"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("'old_string' must be a string".into()))?;
        let new_string = input["new_string"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("'new_string' must be a string".into()))?;

        let content = tokio::fs::read_to_string(path_str)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Cannot read file: {e}")))?;

        let count = content.matches(old_string).count();
        if count == 0 {
            return Err(ToolError::InvalidInput(format!(
                "old_string not found in {path_str}"
            )));
        }
        if count > 1 {
            return Err(ToolError::InvalidInput(format!(
                "old_string found {count} times in {path_str} (must be unique)"
            )));
        }

        let new_content = content.replacen(old_string, new_string, 1);

        tokio::fs::write(path_str, &new_content)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Cannot write file: {e}")))?;

        // Build a simple diff preview
        let old_lines: Vec<&str> = old_string.lines().collect();
        let new_lines: Vec<&str> = new_string.lines().collect();
        let mut diff = String::new();
        for line in &old_lines {
            diff.push_str(&format!("- {line}\n"));
        }
        for line in &new_lines {
            diff.push_str(&format!("+ {line}\n"));
        }

        Ok(ToolResult::success(format!(
            "Applied edit to {path_str}\n{diff}"
        )))
    }
}
