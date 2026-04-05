use crate::rules::{is_always_allowed_tool, is_dangerous_command, is_dangerous_file};
use crate::types::{PermissionContext, PermissionDecision, PermissionMode, PermissionRequest};

pub struct PermissionPipeline {
    layers: Vec<Box<dyn PermissionLayer>>,
}

pub trait PermissionLayer: Send + Sync {
    fn evaluate(
        &self,
        request: &PermissionRequest,
        ctx: &PermissionContext,
    ) -> Option<PermissionDecision>;
}

/// Layer 1: Static rules (always allow safe read-only tools; escalate dangerous targets).
pub struct StaticRulesLayer;

/// Layer 2: Mode-based (current permission mode).
pub struct ModeBasedLayer;

/// Layer 3: LLM classifier (placeholder — returns [`None`] for now).
pub struct LlmClassifierLayer;

impl PermissionPipeline {
    pub fn new() -> Self {
        Self {
            layers: vec![
                Box::new(StaticRulesLayer),
                Box::new(ModeBasedLayer),
                Box::new(LlmClassifierLayer),
            ],
        }
    }

    pub fn with_layers(layers: Vec<Box<dyn PermissionLayer>>) -> Self {
        Self { layers }
    }

    pub fn evaluate(&self, request: &PermissionRequest, ctx: &PermissionContext) -> PermissionDecision {
        for layer in &self.layers {
            if let Some(decision) = layer.evaluate(request, ctx) {
                return decision;
            }
        }
        PermissionDecision::Ask
    }
}

impl Default for PermissionPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionLayer for StaticRulesLayer {
    fn evaluate(
        &self,
        request: &PermissionRequest,
        ctx: &PermissionContext,
    ) -> Option<PermissionDecision> {
        for s in collect_danger_strings(request) {
            if is_dangerous_file(&s) || is_dangerous_command(&s) {
                return Some(PermissionDecision::Ask);
            }
        }

        // `Plan` must review every action; defer to [`ModeBasedLayer`].
        if ctx.mode == PermissionMode::Plan {
            return None;
        }

        if is_always_allowed_tool(&request.tool_name) && request.is_read_only {
            return Some(PermissionDecision::Allow);
        }

        None
    }
}

impl PermissionLayer for ModeBasedLayer {
    fn evaluate(
        &self,
        request: &PermissionRequest,
        ctx: &PermissionContext,
    ) -> Option<PermissionDecision> {
        match ctx.mode {
            PermissionMode::Bypass => Some(PermissionDecision::Allow),
            PermissionMode::Plan => Some(PermissionDecision::Ask),
            PermissionMode::Default => {
                if request.is_read_only {
                    Some(PermissionDecision::Allow)
                } else {
                    Some(PermissionDecision::Ask)
                }
            }
            PermissionMode::AcceptEdits => {
                if request.is_read_only || edit_paths_allowed_under_cwd(request, &ctx.cwd) {
                    Some(PermissionDecision::Allow)
                } else {
                    Some(PermissionDecision::Ask)
                }
            }
            PermissionMode::Auto => None,
        }
    }
}

impl PermissionLayer for LlmClassifierLayer {
    fn evaluate(
        &self,
        _request: &PermissionRequest,
        _ctx: &PermissionContext,
    ) -> Option<PermissionDecision> {
        None
    }
}

const DANGER_SCAN_KEYS: &[&str] = &[
    "command",
    "cmd",
    "path",
    "file_path",
    "directory",
    "working_directory",
];

fn collect_danger_strings(request: &PermissionRequest) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(obj) = request.arguments.as_object() {
        for (key, val) in obj {
            if DANGER_SCAN_KEYS.contains(&key.as_str()) {
                if let Some(s) = val.as_str() {
                    out.push(s.to_string());
                }
            }
        }
    }
    out
}

/// Normalize a path string. Returns `None` if the path attempts to escape
/// above the starting point (e.g. `../../etc/passwd` from a relative root).
fn normalize_path_checked(s: &str) -> Option<String> {
    let replaced = s.replace('\\', "/");
    let is_absolute = replaced.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for component in replaced.split('/') {
        match component {
            "." | "" => {}
            ".." => {
                if parts.pop().is_none() && !is_absolute {
                    return None;
                }
            }
            c => parts.push(c),
        }
    }
    let joined = parts.join("/");
    if is_absolute {
        Some(format!("/{joined}"))
    } else {
        Some(joined)
    }
}

fn normalize_path(s: &str) -> String {
    normalize_path_checked(s).unwrap_or_default()
}

const PATH_KEYS: &[&str] = &["path", "file_path", "directory", "working_directory"];

fn collect_path_strings(value: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            if PATH_KEYS.contains(&key.as_str()) {
                if let Some(s) = val.as_str() {
                    out.push(s.to_string());
                }
            }
        }
    }
    out
}

fn edit_paths_allowed_under_cwd(request: &PermissionRequest, cwd: &str) -> bool {
    let cwd_norm = normalize_path(cwd);
    let paths = collect_path_strings(&request.arguments);

    if paths.is_empty() {
        return false;
    }

    paths.iter().all(|p| is_path_under_cwd(p, &cwd_norm))
}

fn is_path_under_cwd(path: &str, cwd_norm: &str) -> bool {
    let p = match normalize_path_checked(path) {
        Some(p) if !p.is_empty() => p,
        _ => return false,
    };
    let prefix = format!("{}/", cwd_norm);
    if p == *cwd_norm || p.starts_with(&prefix) {
        return true;
    }
    // Relative paths (no absolute prefix) are considered under cwd
    !p.starts_with('/') && !p.contains(":\\")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(mode: PermissionMode) -> PermissionContext {
        PermissionContext {
            mode,
            cwd: "/project".to_string(),
            denied_count: 0,
            consecutive_denials: 0,
        }
    }

    #[test]
    fn pipeline_order_static_before_mode() {
        let p = PermissionPipeline::with_layers(vec![
            Box::new(StaticRulesLayer),
            Box::new(ModeBasedLayer),
        ]);

        let req = PermissionRequest {
            tool_name: "Shell".to_string(),
            description: "run".to_string(),
            is_read_only: false,
            working_directory: None,
            arguments: json!({ "command": "rm -rf /" }),
        };

        let d = p.evaluate(&req, &ctx(PermissionMode::Bypass));
        assert_eq!(d, PermissionDecision::Ask);
    }

    #[test]
    fn mode_bypass_after_static_allows_only_if_static_does_not_apply() {
        let p = PermissionPipeline::new();
        let req = PermissionRequest {
            tool_name: "Write".to_string(),
            description: "edit".to_string(),
            is_read_only: false,
            working_directory: None,
            arguments: json!({ "path": "/project/src/lib.rs" }),
        };
        assert_eq!(
            p.evaluate(&req, &ctx(PermissionMode::Bypass)),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn mode_plan_always_asks() {
        let p = PermissionPipeline::new();
        let req = PermissionRequest {
            tool_name: "Grep".to_string(),
            description: "search".to_string(),
            is_read_only: true,
            working_directory: None,
            arguments: json!({}),
        };
        assert_eq!(
            p.evaluate(&req, &ctx(PermissionMode::Plan)),
            PermissionDecision::Ask
        );
    }

    #[test]
    fn mode_default_allows_read_denies_write_via_ask() {
        let p = PermissionPipeline::new();
        let read = PermissionRequest {
            tool_name: "CustomRead".to_string(),
            description: "".to_string(),
            is_read_only: true,
            working_directory: None,
            arguments: json!({}),
        };
        assert_eq!(
            p.evaluate(&read, &ctx(PermissionMode::Default)),
            PermissionDecision::Allow
        );

        let write = PermissionRequest {
            tool_name: "Write".to_string(),
            description: "".to_string(),
            is_read_only: false,
            working_directory: None,
            arguments: json!({ "path": "/project/a" }),
        };
        assert_eq!(
            p.evaluate(&write, &ctx(PermissionMode::Default)),
            PermissionDecision::Ask
        );
    }

    #[test]
    fn mode_accept_edits_allows_under_cwd() {
        let p = PermissionPipeline::new();
        let req = PermissionRequest {
            tool_name: "ApplyPatch".to_string(),
            description: "".to_string(),
            is_read_only: false,
            working_directory: Some("/project".to_string()),
            arguments: json!({ "path": "/project/foo.rs" }),
        };
        assert_eq!(
            p.evaluate(&req, &ctx(PermissionMode::AcceptEdits)),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn custom_layer_ordering_first_wins() {
        struct AlwaysAllow;
        impl PermissionLayer for AlwaysAllow {
            fn evaluate(
                &self,
                _request: &PermissionRequest,
                _ctx: &PermissionContext,
            ) -> Option<PermissionDecision> {
                Some(PermissionDecision::Allow)
            }
        }

        let p = PermissionPipeline::with_layers(vec![
            Box::new(AlwaysAllow),
            Box::new(StaticRulesLayer),
        ]);

        let req = PermissionRequest {
            tool_name: "Shell".to_string(),
            description: "".to_string(),
            is_read_only: false,
            working_directory: None,
            arguments: json!({ "cmd": "rm -rf /" }),
        };

        assert_eq!(
            p.evaluate(&req, &ctx(PermissionMode::Default)),
            PermissionDecision::Allow
        );
    }
}
