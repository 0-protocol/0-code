use std::sync::Arc;

use crate::registry::ToolRegistry;

use super::bash::BashTool;
use super::file_edit::FileEditTool;
use super::file_read::FileReadTool;
use super::file_write::FileWriteTool;
use super::glob_tool::GlobTool;
use super::grep::GrepTool;
use super::web_fetch::WebFetchTool;
use super::web_search::WebSearchTool;

/// Register all core tools with the given registry.
pub fn register_core_tools(registry: &mut ToolRegistry) {
    registry.register(Arc::new(BashTool::new()));
    registry.register(Arc::new(FileReadTool::new()));
    registry.register(Arc::new(FileWriteTool::new()));
    registry.register(Arc::new(FileEditTool::new()));
    registry.register(Arc::new(GlobTool::new()));
    registry.register(Arc::new(GrepTool::new()));
    registry.register(Arc::new(WebFetchTool::new()));
    registry.register(Arc::new(WebSearchTool::new()));
}
