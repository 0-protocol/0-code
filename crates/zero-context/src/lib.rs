//! Context management, compaction, and persistent memory for the 0-code agent.

mod compaction;
mod file_cache;
mod manager;
mod memory;
mod types;

pub use compaction::Compactor;
pub use file_cache::{CacheEntry, FileCache};
pub use manager::ContextManager;
pub use memory::{MemoryEntry, MemoryManager, MemorySource, ProjectMemory};
pub use types::{CompactionTier, TokenBudget};

#[cfg(test)]
mod tests {
    use super::TokenBudget;

    #[test]
    fn token_budget_utilization() {
        let mut b = TokenBudget::new(1000);
        b.current_tokens = 500;
        assert!((b.utilization() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn token_budget_thresholds() {
        let mut b = TokenBudget::new(1000);
        b.current_tokens = 500;
        assert!(!b.needs_micro_compact()); // 0.5 < 0.6
        b.current_tokens = 600;
        assert!(b.needs_micro_compact());
        assert!(!b.needs_auto_compact());
        b.current_tokens = 800;
        assert!(b.needs_auto_compact());
        assert!(!b.needs_reactive_compact());
        b.current_tokens = 950;
        assert!(b.needs_reactive_compact());
    }
}
