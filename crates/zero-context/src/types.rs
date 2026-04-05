use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    pub max_tokens: u64,
    pub current_tokens: u64,
    pub micro_compact_threshold: f32, // 0.6 = trigger at 60% capacity
    pub auto_compact_threshold: f32,  // 0.8
    pub reactive_threshold: f32,      // 0.95
}

impl TokenBudget {
    pub fn new(max_tokens: u64) -> Self {
        Self {
            max_tokens,
            current_tokens: 0,
            micro_compact_threshold: 0.6,
            auto_compact_threshold: 0.8,
            reactive_threshold: 0.95,
        }
    }

    pub fn utilization(&self) -> f32 {
        if self.max_tokens == 0 {
            return 1.0;
        }
        self.current_tokens as f32 / self.max_tokens as f32
    }

    pub fn needs_micro_compact(&self) -> bool {
        self.utilization() >= self.micro_compact_threshold
    }

    pub fn needs_auto_compact(&self) -> bool {
        self.utilization() >= self.auto_compact_threshold
    }

    pub fn needs_reactive_compact(&self) -> bool {
        self.utilization() >= self.reactive_threshold
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionTier {
    Micro,    // Clear old tool results
    Auto,     // Summarize via LLM
    Session,  // Extract to persistent store
    Reactive, // Emergency truncation on API error
}
