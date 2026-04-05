pub struct DenialTracker {
    consecutive_denials: u32,
    total_denials: u32,
    max_consecutive: u32,
    max_total: u32,
}

impl DenialTracker {
    pub fn new() -> Self {
        Self {
            consecutive_denials: 0,
            total_denials: 0,
            max_consecutive: 3,
            max_total: 20,
        }
    }

    pub fn record_denial(&mut self) {
        self.consecutive_denials = self.consecutive_denials.saturating_add(1);
        self.total_denials = self.total_denials.saturating_add(1);
    }

    pub fn record_allow(&mut self) {
        self.consecutive_denials = 0;
    }

    /// When limits are exceeded, callers should force `Ask` to avoid tight deny loops.
    pub fn should_fallback_to_ask(&self) -> bool {
        self.consecutive_denials >= self.max_consecutive || self.total_denials >= self.max_total
    }

    pub fn consecutive(&self) -> u32 {
        self.consecutive_denials
    }

    pub fn total(&self) -> u32 {
        self.total_denials
    }
}

impl Default for DenialTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consecutive_trips_circuit_breaker() {
        let mut t = DenialTracker::new();
        assert!(!t.should_fallback_to_ask());
        t.record_denial();
        t.record_denial();
        assert!(!t.should_fallback_to_ask());
        t.record_denial();
        assert!(t.should_fallback_to_ask());
    }

    #[test]
    fn allow_resets_consecutive() {
        let mut t = DenialTracker::new();
        t.record_denial();
        t.record_denial();
        t.record_allow();
        assert_eq!(t.consecutive(), 0);
        t.record_denial();
        assert_eq!(t.consecutive(), 1);
        assert!(!t.should_fallback_to_ask());
    }

    #[test]
    fn total_trips_at_max_total() {
        let mut t = DenialTracker {
            consecutive_denials: 0,
            total_denials: 0,
            max_consecutive: 100,
            max_total: 5,
        };
        for _ in 0..4 {
            t.record_denial();
            t.record_allow();
        }
        assert!(!t.should_fallback_to_ask());
        t.record_denial();
        assert!(t.should_fallback_to_ask());
    }
}
