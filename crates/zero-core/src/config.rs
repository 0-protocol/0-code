#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub max_retries: u32,
    pub max_turns: u32,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
    pub model: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            max_turns: 100,
            max_tokens: 16384,
            temperature: None,
            system_prompt: None,
            model: "default".to_string(),
        }
    }
}
