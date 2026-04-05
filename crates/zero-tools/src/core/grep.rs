use std::path::Path;

use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::tool::{Tool, ToolError};
use crate::types::ToolResult;

const DEFAULT_MAX_RESULTS: usize = 200;

pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search file contents using a regex pattern. Returns matching lines with file paths and line numbers."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "directory": {
                    "type": "string",
                    "description": "Directory to search in (defaults to current dir)"
                },
                "include": {
                    "type": "string",
                    "description": "Glob pattern to include files (e.g. '*.rs')"
                },
                "exclude": {
                    "type": "string",
                    "description": "Glob pattern to exclude files (e.g. '*.log')"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of matching lines to return (default: 200)"
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
        let include = input["include"].as_str();
        let exclude = input["exclude"].as_str();
        let max_results = input["max_results"]
            .as_u64()
            .map(|v| v as usize)
            .unwrap_or(DEFAULT_MAX_RESULTS);

        let re = Regex::new(pattern_str)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid regex: {e}")))?;

        let include_glob = include
            .map(|p| {
                let p = if p.starts_with("**/") {
                    p.to_string()
                } else {
                    format!("**/{p}")
                };
                globset::Glob::new(&p)
                    .map(|g| g.compile_matcher())
                    .map_err(|e| ToolError::InvalidInput(format!("Invalid include glob: {e}")))
            })
            .transpose()?;

        let exclude_glob = exclude
            .map(|p| {
                let p = if p.starts_with("**/") {
                    p.to_string()
                } else {
                    format!("**/{p}")
                };
                globset::Glob::new(&p)
                    .map(|g| g.compile_matcher())
                    .map_err(|e| ToolError::InvalidInput(format!("Invalid exclude glob: {e}")))
            })
            .transpose()?;

        let root = Path::new(directory).to_path_buf();
        if !root.is_dir() {
            return Err(ToolError::ExecutionFailed(format!(
                "Not a directory: {directory}"
            )));
        }

        let results = tokio::task::spawn_blocking(move || {
            let mut matches = Vec::new();
            for entry in WalkDir::new(&root)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }

                let rel_path = entry.path().strip_prefix(&root).unwrap_or(entry.path());

                if let Some(ref inc) = include_glob {
                    if !inc.is_match(rel_path) {
                        continue;
                    }
                }
                if let Some(ref exc) = exclude_glob {
                    if exc.is_match(rel_path) {
                        continue;
                    }
                }

                let content = match std::fs::read_to_string(entry.path()) {
                    Ok(c) => c,
                    Err(_) => continue, // skip binary/unreadable files
                };

                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        matches.push(format!(
                            "{}:{}:{}",
                            entry.path().display(),
                            line_num + 1,
                            line
                        ));
                        if matches.len() >= max_results {
                            return matches;
                        }
                    }
                }
            }
            matches
        })
        .await
        .map_err(|e| ToolError::ExecutionFailed(format!("Task join error: {e}")))?;

        let count = results.len();
        let truncated = count >= max_results;

        let content = if results.is_empty() {
            "No matches found.".to_string()
        } else {
            let mut out = results.join("\n");
            if truncated {
                out.push_str(&format!("\n\n(truncated at {max_results} results)"));
            }
            out
        };

        Ok(ToolResult::success(content).with_metadata(json!({
            "count": count,
            "truncated": truncated,
        })))
    }
}
