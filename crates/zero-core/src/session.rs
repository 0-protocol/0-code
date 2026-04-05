use chrono::{DateTime, Utc};
use uuid::Uuid;
use zero_llm::{ContentBlock, Message, Role, Usage};

pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
    pub total_usage: Usage,
    pub turn_count: u32,
    pub created_at: DateTime<Utc>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            messages: Vec::new(),
            total_usage: Usage::default(),
            turn_count: 0,
            created_at: Utc::now(),
        }
    }

    pub fn add_user_message(&mut self, text: &str) {
        self.messages.push(Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
        });
    }

    pub fn add_assistant_message(&mut self, blocks: Vec<ContentBlock>) {
        self.messages.push(Message {
            role: Role::Assistant,
            content: blocks,
        });
    }

    /// Append tool results as a user message with `ToolResult` content blocks.
    pub fn add_tool_results(&mut self, results: Vec<(String, String, bool)>) {
        let blocks = results
            .into_iter()
            .map(|(tool_use_id, content, is_error)| ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            })
            .collect();
        self.messages.push(Message {
            role: Role::User,
            content: blocks,
        });
    }

    pub fn accumulate_usage(&mut self, usage: &Usage) {
        self.total_usage.input_tokens += usage.input_tokens;
        self.total_usage.output_tokens += usage.output_tokens;
        self.total_usage.cache_creation_input_tokens += usage.cache_creation_input_tokens;
        self.total_usage.cache_read_input_tokens += usage.cache_read_input_tokens;
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
