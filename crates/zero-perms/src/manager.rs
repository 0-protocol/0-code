use crate::pipeline::PermissionPipeline;
use crate::tracker::DenialTracker;
use crate::types::{PermissionContext, PermissionDecision, PermissionMode, PermissionRequest};

pub struct PermissionManager {
    pipeline: PermissionPipeline,
    tracker: DenialTracker,
    mode: PermissionMode,
    cwd: String,
}

impl PermissionManager {
    pub fn new(mode: PermissionMode, cwd: String) -> Self {
        Self {
            pipeline: PermissionPipeline::new(),
            tracker: DenialTracker::new(),
            mode,
            cwd,
        }
    }

    pub fn check(&mut self, request: &PermissionRequest) -> PermissionDecision {
        if self.tracker.should_fallback_to_ask() {
            return PermissionDecision::Ask;
        }
        let ctx = PermissionContext {
            mode: self.mode,
            cwd: self.cwd.clone(),
            denied_count: self.tracker.total(),
            consecutive_denials: self.tracker.consecutive(),
        };
        self.pipeline.evaluate(request, &ctx)
    }

    pub fn record_decision(&mut self, allowed: bool) {
        if allowed {
            self.tracker.record_allow();
        } else {
            self.tracker.record_denial();
        }
    }

    pub fn set_mode(&mut self, mode: PermissionMode) {
        self.mode = mode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn manager_fallback_when_tracker_tripped() {
        let mut m = PermissionManager::new(PermissionMode::Bypass, "/p".to_string());
        for _ in 0..3 {
            m.record_decision(false);
        }
        let req = PermissionRequest {
            tool_name: "Write".to_string(),
            description: "".to_string(),
            is_read_only: false,
            working_directory: None,
            arguments: json!({}),
        };
        assert_eq!(m.check(&req), PermissionDecision::Ask);
    }
}
