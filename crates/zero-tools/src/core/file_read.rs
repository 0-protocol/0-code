use std::path::Path;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::tool::{Tool, ToolError};
use crate::types::ToolResult;

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB

pub struct FileReadTool;

impl FileReadTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Supports optional line offset and limit."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to read"
                },
                "offset": {
                    "type": "integer",
                    "description": "1-based line number to start reading from"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of lines to read"
                }
            },
            "required": ["path"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value) -> Result<ToolResult, ToolError> {
        let path_str = input["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("'path' must be a string".into()))?;

        let path = Path::new(path_str);

        if !path.exists() {
            return Err(ToolError::ExecutionFailed(format!(
                "File not found: {path_str}"
            )));
        }

        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Cannot read metadata: {e}")))?;

        if metadata.len() > MAX_FILE_SIZE {
            return Err(ToolError::ExecutionFailed(format!(
                "File too large: {} bytes (max {})",
                metadata.len(),
                MAX_FILE_SIZE
            )));
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Cannot read file: {e}")))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let offset = input["offset"]
            .as_u64()
            .map(|v| v.saturating_sub(1) as usize)
            .unwrap_or(0);

        let limit = input["limit"]
            .as_u64()
            .map(|v| v as usize)
            .unwrap_or(total_lines);

        let end = (offset + limit).min(total_lines);

        let mut output = String::new();
        for (i, line) in lines[offset..end].iter().enumerate() {
            let line_num = offset + i + 1;
            output.push_str(&format!("{line_num:6}|{line}\n"));
        }

        if output.is_empty() {
            output = "(empty file)".to_string();
        }

        Ok(ToolResult::success(output).with_metadata(json!({
            "total_lines": total_lines,
            "lines_shown": end - offset,
        })))
    }
}
