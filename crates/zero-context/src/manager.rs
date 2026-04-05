use std::path::Path;

use crate::file_cache::FileCache;
use crate::memory::MemoryManager;
use crate::types::{CompactionTier, TokenBudget};

pub struct ContextManager {
    budget: TokenBudget,
    file_cache: FileCache,
    memory: MemoryManager,
}

impl ContextManager {
    pub fn new(max_context_tokens: u64, project_path: &Path) -> Self {
        Self {
            budget: TokenBudget::new(max_context_tokens),
            file_cache: FileCache::default_limits(),
            memory: MemoryManager::new(project_path),
        }
    }

    pub fn with_cache_limits(
        max_context_tokens: u64,
        project_path: &Path,
        max_entries: usize,
        max_bytes: usize,
    ) -> Self {
        Self {
            budget: TokenBudget::new(max_context_tokens),
            file_cache: FileCache::new(max_entries, max_bytes),
            memory: MemoryManager::new(project_path),
        }
    }

    pub fn update_token_count(&mut self, tokens: u64) {
        self.budget.current_tokens = tokens;
    }

    pub fn check_compaction_needed(&self) -> Option<CompactionTier> {
        if self.budget.needs_reactive_compact() {
            return Some(CompactionTier::Reactive);
        }
        if self.budget.needs_auto_compact() {
            return Some(CompactionTier::Auto);
        }
        if self.budget.needs_micro_compact() {
            return Some(CompactionTier::Micro);
        }
        None
    }

    pub fn file_cache(&mut self) -> &mut FileCache {
        &mut self.file_cache
    }

    pub fn memory(&self) -> &MemoryManager {
        &self.memory
    }

    pub fn budget(&self) -> &TokenBudget {
        &self.budget
    }

    pub fn budget_mut(&mut self) -> &mut TokenBudget {
        &mut self.budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compaction_priority_reactive_over_auto() {
        let tmp = std::env::temp_dir();
        let mut m = ContextManager::new(1000, &tmp);
        m.budget_mut().current_tokens = 960; // 96%
        assert_eq!(m.check_compaction_needed(), Some(CompactionTier::Reactive));
    }
}
