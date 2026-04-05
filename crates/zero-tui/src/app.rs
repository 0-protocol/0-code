use chrono::Local;

pub struct App {
    pub messages: Vec<DisplayMessage>,
    pub input: String,
    pub input_cursor: usize,
    pub scroll_offset: u16,
    pub model_name: String,
    pub usage_input: u64,
    pub usage_output: u64,
    pub is_processing: bool,
    pub should_quit: bool,
    pub status_text: String,
    pub active_tools: Vec<ActiveTool>,
}

#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool(String),
}

#[derive(Debug, Clone)]
pub struct ActiveTool {
    pub id: String,
    pub name: String,
}

impl App {
    pub fn new(model_name: String) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            input_cursor: 0,
            scroll_offset: 0,
            model_name,
            usage_input: 0,
            usage_output: 0,
            is_processing: false,
            should_quit: false,
            status_text: String::new(),
            active_tools: Vec::new(),
        }
    }

    fn now_ts() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }

    pub fn add_text_delta(&mut self, text: &str) {
        let needs_new = match self.messages.last() {
            None => true,
            Some(m) => !matches!(m.role, MessageRole::Assistant),
        };
        if needs_new {
            self.messages.push(DisplayMessage {
                role: MessageRole::Assistant,
                content: String::new(),
                timestamp: Self::now_ts(),
            });
        }
        if let Some(last) = self.messages.last_mut() {
            if matches!(last.role, MessageRole::Assistant) {
                last.content.push_str(text);
            }
        }
    }

    pub fn tool_start(&mut self, id: String, name: String) {
        self.active_tools.push(ActiveTool { id, name });
    }

    pub fn tool_end(&mut self, id: &str, result: &str, is_error: bool) {
        let name = self
            .active_tools
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "tool".to_string());
        self.active_tools.retain(|t| t.id != id);
        let label = if is_error { "error" } else { "result" };
        self.messages.push(DisplayMessage {
            role: MessageRole::Tool(name),
            content: format!("[{label}] {result}"),
            timestamp: Self::now_ts(),
        });
    }

    pub fn add_user_message(&mut self, text: &str) {
        self.messages.push(DisplayMessage {
            role: MessageRole::User,
            content: text.to_string(),
            timestamp: Self::now_ts(),
        });
    }

    pub fn update_usage(&mut self, input: u64, output: u64) {
        self.usage_input = input;
        self.usage_output = output;
    }
}
