use crate::transport::McpTransport;
use crate::McpError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

pub struct StdioTransport {
    stdin: Mutex<tokio::process::ChildStdin>,
    stdout: Mutex<BufReader<tokio::process::ChildStdout>>,
    _child: Child,
    next_id: AtomicU64,
}

impl StdioTransport {
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self, McpError> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .envs(env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        let mut child = cmd
            .spawn()
            .map_err(|e| McpError::ConnectionFailed(format!("failed to spawn {command}: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::ConnectionFailed("no stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::ConnectionFailed("no stdout".into()))?;

        Ok(Self {
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
            _child: child,
            next_id: AtomicU64::new(1),
        })
    }

    fn next_request_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send(&self, mut message: serde_json::Value) -> Result<serde_json::Value, McpError> {
        let id = self.next_request_id();
        if let Some(obj) = message.as_object_mut() {
            obj.insert("jsonrpc".into(), "2.0".into());
            obj.insert("id".into(), id.into());
        }

        let serialized =
            serde_json::to_string(&message).map_err(|e| McpError::Protocol(e.to_string()))?;

        {
            let mut stdin = self.stdin.lock().await;
            stdin
                .write_all(serialized.as_bytes())
                .await
                .map_err(McpError::Io)?;
            stdin.write_all(b"\n").await.map_err(McpError::Io)?;
            stdin.flush().await.map_err(McpError::Io)?;
        }

        {
            let mut stdout = self.stdout.lock().await;
            let mut line = String::new();
            stdout.read_line(&mut line).await.map_err(McpError::Io)?;
            serde_json::from_str(&line).map_err(|e| McpError::Protocol(e.to_string()))
        }
    }

    async fn close(&self) -> Result<(), McpError> {
        let mut stdin = self.stdin.lock().await;
        stdin.shutdown().await.map_err(McpError::Io)
    }
}
