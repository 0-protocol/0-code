use crate::stdio::StdioTransport;
use crate::transport::McpTransport;
use crate::{McpResource, McpServerConfig, McpTool, TransportType};
use std::collections::HashMap;
use tracing::{debug, warn};

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Server not found: {0}")]
    ServerNotFound(String),
    #[error("Tool not found: {server}/{tool}")]
    ToolNotFound { server: String, tool: String },
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Auth error: {0}")]
    Auth(String),
    #[error("Timeout")]
    Timeout,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

struct McpServerConnection {
    config: McpServerConfig,
    transport: Box<dyn McpTransport>,
    tools: Vec<McpTool>,
    resources: Vec<McpResource>,
    _initialized: bool,
}

pub struct McpClient {
    servers: HashMap<String, McpServerConnection>,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    pub async fn connect(&mut self, config: McpServerConfig) -> Result<(), McpError> {
        let name = config.name.clone();
        debug!(server = %name, transport = ?config.transport, "connecting to MCP server");

        let transport: Box<dyn McpTransport> = match config.transport {
            TransportType::Stdio => {
                let command = config
                    .command
                    .as_deref()
                    .ok_or_else(|| McpError::ConnectionFailed("no command for stdio".into()))?;
                let args = config.args.as_deref().unwrap_or_default();
                let env = config.env.clone().unwrap_or_default();
                Box::new(StdioTransport::spawn(command, args, &env).await?)
            }
            TransportType::Sse => {
                return Err(McpError::ConnectionFailed(
                    "SSE transport not yet implemented".into(),
                ));
            }
            TransportType::WebSocket => {
                return Err(McpError::ConnectionFailed(
                    "WebSocket transport not yet implemented".into(),
                ));
            }
        };

        let init_req = serde_json::json!({
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "zero-code",
                    "version": "0.1.0"
                }
            }
        });

        let init_resp = transport.send(init_req).await?;
        debug!(server = %name, response = %init_resp, "MCP initialize response");

        let tools_req = serde_json::json!({
            "method": "tools/list",
            "params": {}
        });

        let tools_resp = transport.send(tools_req).await?;
        let tools = Self::parse_tools_response(&name, &tools_resp);

        let resources_req = serde_json::json!({
            "method": "resources/list",
            "params": {}
        });

        let resources = match transport.send(resources_req).await {
            Ok(resp) => Self::parse_resources_response(&resp),
            Err(e) => {
                warn!(server = %name, error = %e, "failed to list resources");
                Vec::new()
            }
        };

        self.servers.insert(
            name,
            McpServerConnection {
                config,
                transport,
                tools,
                resources,
                _initialized: true,
            },
        );

        Ok(())
    }

    pub async fn disconnect(&mut self, server_name: &str) -> Result<(), McpError> {
        let conn = self
            .servers
            .remove(server_name)
            .ok_or_else(|| McpError::ServerNotFound(server_name.into()))?;
        conn.transport.close().await
    }

    pub fn list_tools(&self) -> Vec<McpTool> {
        self.servers
            .values()
            .flat_map(|conn| {
                conn.tools.iter().map(|t| {
                    let mut tool = t.clone();
                    tool.name = format!("mcp__{}_{}", conn.config.name, t.name);
                    tool
                })
            })
            .collect()
    }

    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let conn = self
            .servers
            .get(server_name)
            .ok_or_else(|| McpError::ServerNotFound(server_name.into()))?;

        if !conn.tools.iter().any(|t| t.name == tool_name) {
            return Err(McpError::ToolNotFound {
                server: server_name.into(),
                tool: tool_name.into(),
            });
        }

        let request = serde_json::json!({
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": input
            }
        });

        conn.transport.send(request).await
    }

    pub fn list_resources(&self) -> Vec<McpResource> {
        self.servers
            .values()
            .flat_map(|conn| conn.resources.clone())
            .collect()
    }

    fn parse_tools_response(server_name: &str, response: &serde_json::Value) -> Vec<McpTool> {
        let tools_array = response
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array());

        let Some(tools) = tools_array else {
            return Vec::new();
        };

        tools
            .iter()
            .filter_map(|t| {
                Some(McpTool {
                    name: t.get("name")?.as_str()?.to_string(),
                    description: t
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string(),
                    input_schema: t
                        .get("inputSchema")
                        .cloned()
                        .unwrap_or(serde_json::json!({})),
                    server_name: server_name.to_string(),
                })
            })
            .collect()
    }

    fn parse_resources_response(response: &serde_json::Value) -> Vec<McpResource> {
        let resources_array = response
            .get("result")
            .and_then(|r| r.get("resources"))
            .and_then(|t| t.as_array());

        let Some(resources) = resources_array else {
            return Vec::new();
        };

        resources
            .iter()
            .filter_map(|r| {
                Some(McpResource {
                    uri: r.get("uri")?.as_str()?.to_string(),
                    name: r.get("name")?.as_str()?.to_string(),
                    description: r.get("description").and_then(|d| d.as_str()).map(String::from),
                    mime_type: r.get("mimeType").and_then(|m| m.as_str()).map(String::from),
                })
            })
            .collect()
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tools_response() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "search",
                        "description": "Search files",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "read",
                        "description": "Read a file",
                        "inputSchema": {}
                    }
                ]
            }
        });

        let tools = McpClient::parse_tools_response("test-server", &response);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "search");
        assert_eq!(tools[0].description, "Search files");
        assert_eq!(tools[0].server_name, "test-server");
        assert_eq!(tools[1].name, "read");
    }

    #[test]
    fn test_parse_tools_response_empty() {
        let response = serde_json::json!({ "result": {} });
        let tools = McpClient::parse_tools_response("s", &response);
        assert!(tools.is_empty());
    }

    #[test]
    fn test_parse_resources_response() {
        let response = serde_json::json!({
            "result": {
                "resources": [
                    {
                        "uri": "file:///readme.md",
                        "name": "README",
                        "description": "Project readme",
                        "mimeType": "text/markdown"
                    }
                ]
            }
        });

        let resources = McpClient::parse_resources_response(&response);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].uri, "file:///readme.md");
        assert_eq!(resources[0].name, "README");
        assert_eq!(resources[0].description, Some("Project readme".into()));
        assert_eq!(resources[0].mime_type, Some("text/markdown".into()));
    }

    #[test]
    fn test_list_tools_namespaced() {
        let mut client = McpClient::new();
        client.servers.insert(
            "fs".into(),
            McpServerConnection {
                config: McpServerConfig {
                    name: "fs".into(),
                    transport: TransportType::Stdio,
                    command: None,
                    args: None,
                    url: None,
                    env: None,
                },
                transport: Box::new(NoopTransport),
                tools: vec![McpTool {
                    name: "read".into(),
                    description: "Read file".into(),
                    input_schema: serde_json::json!({}),
                    server_name: "fs".into(),
                }],
                resources: vec![],
                _initialized: true,
            },
        );

        let tools = client.list_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "mcp__fs_read");
    }

    struct NoopTransport;

    #[async_trait::async_trait]
    impl McpTransport for NoopTransport {
        async fn send(
            &self,
            _message: serde_json::Value,
        ) -> Result<serde_json::Value, McpError> {
            Ok(serde_json::json!({}))
        }
        async fn close(&self) -> Result<(), McpError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_call_tool_server_not_found() {
        let client = McpClient::new();
        let result = client
            .call_tool("nonexistent", "read", serde_json::json!({}))
            .await;
        assert!(matches!(result, Err(McpError::ServerNotFound(_))));
    }

    #[tokio::test]
    async fn test_call_tool_not_found() {
        let mut client = McpClient::new();
        client.servers.insert(
            "fs".into(),
            McpServerConnection {
                config: McpServerConfig {
                    name: "fs".into(),
                    transport: TransportType::Stdio,
                    command: None,
                    args: None,
                    url: None,
                    env: None,
                },
                transport: Box::new(NoopTransport),
                tools: vec![],
                resources: vec![],
                _initialized: true,
            },
        );

        let result = client
            .call_tool("fs", "nonexistent", serde_json::json!({}))
            .await;
        assert!(matches!(
            result,
            Err(McpError::ToolNotFound { server: _, tool: _ })
        ));
    }

    #[tokio::test]
    async fn test_disconnect_not_found() {
        let mut client = McpClient::new();
        let result = client.disconnect("nope").await;
        assert!(matches!(result, Err(McpError::ServerNotFound(_))));
    }
}
