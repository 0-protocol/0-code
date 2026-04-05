use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::AgentConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMessage {
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct TeamMember {
    pub name: String,
    pub config: AgentConfig,
    pub inbox: mpsc::Receiver<TeamMessage>,
    pub outbox: mpsc::Sender<TeamMessage>,
}

pub struct Team {
    members: Arc<Mutex<HashMap<String, mpsc::Sender<TeamMessage>>>>,
    router: mpsc::Sender<TeamMessage>,
    _dispatch: JoinHandle<()>,
}

impl Team {
    pub fn new() -> Self {
        let members: Arc<Mutex<HashMap<String, mpsc::Sender<TeamMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let members_dispatch = Arc::clone(&members);
        let (router, mut router_rx) = mpsc::channel::<TeamMessage>(256);

        let _dispatch = tokio::spawn(async move {
            while let Some(msg) = router_rx.recv().await {
                let target = {
                    let guard = members_dispatch.lock().unwrap_or_else(|e| e.into_inner());
                    guard.get(&msg.to).cloned()
                };
                if let Some(tx) = target {
                    let _ = tx.send(msg).await;
                }
            }
        });

        Self {
            members,
            router,
            _dispatch,
        }
    }

    /// Register a member and return its [`TeamMember`] endpoints.
    pub fn add_member(&mut self, name: String, config: AgentConfig) -> TeamMember {
        let (tx, rx) = mpsc::channel(256);
        self.members
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(name.clone(), tx);
        TeamMember {
            name,
            config,
            inbox: rx,
            outbox: self.router.clone(),
        }
    }

    /// Deliver a message to the target member's inbox.
    pub async fn route(&self, msg: TeamMessage) -> Result<(), TeamError> {
        let to = msg.to.clone();
        let tx = {
            let guard = self.members.lock().unwrap_or_else(|e| e.into_inner());
            guard.get(&msg.to).cloned()
        };
        if let Some(tx) = tx {
            tx.send(msg)
                .await
                .map_err(|_| TeamError::MemberUnavailable)?;
            Ok(())
        } else {
            Err(TeamError::MemberNotFound(to))
        }
    }

    pub fn member_names(&self) -> Vec<String> {
        self.members
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }
}

impl Default for Team {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TeamError {
    #[error("Member not found: {0}")]
    MemberNotFound(String),
    #[error("Member unavailable")]
    MemberUnavailable,
}
