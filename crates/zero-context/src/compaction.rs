use crate::types::CompactionTier;
use serde_json::{json, Value};

/// Compaction strategy for conversation messages
pub struct Compactor {
    tier: CompactionTier,
}

impl Compactor {
    pub fn new(tier: CompactionTier) -> Self {
        Self { tier }
    }

    pub fn tier(&self) -> CompactionTier {
        self.tier
    }

    /// Micro-compact: remove old tool results, keep only last N turns worth
    pub fn micro_compact(&self, messages: &mut [Value], keep_last: usize) {
        if messages.len() <= keep_last {
            return;
        }
        let n_old = messages.len() - keep_last;
        for msg in messages.iter_mut().take(n_old) {
            strip_tool_results_in_message(msg);
        }
    }

    /// Auto-compact: partition into portion to summarize (caller runs LLM) vs keep.
    pub fn auto_compact_prepare(&self, messages: &[Value]) -> Vec<Value> {
        if messages.len() <= 1 {
            return Vec::new();
        }
        // Older half (or all but last message) as "to summarize" — engine may refine split.
        let split = messages.len() / 2;
        messages[..split].to_vec()
    }

    /// Reactive compact: truncate so total length is `target_count` (including the system note).
    pub fn reactive_compact(&self, messages: &mut Vec<Value>, target_count: usize) {
        if target_count == 0 {
            messages.clear();
            return;
        }
        let keep_content = target_count.saturating_sub(1);
        if messages.len() <= keep_content {
            return;
        }
        let drop = messages.len() - keep_content;
        messages.drain(0..drop);
        let note = json!({
            "role": "system",
            "content": "[earlier context truncated]"
        });
        messages.insert(0, note);
    }
}

fn strip_tool_results_in_message(msg: &mut Value) {
    if let Some(obj) = msg.as_object_mut() {
        // OpenAI-style tool role
        if obj.get("role").and_then(|r| r.as_str()) == Some("tool") {
            obj.insert(
                "content".to_string(),
                Value::String("[tool result removed]".to_string()),
            );
            return;
        }
        if let Some(content) = obj.get_mut("content") {
            strip_tool_results_in_content(content);
        }
    }
}

fn strip_tool_results_in_content(content: &mut Value) {
    match content {
        Value::Array(parts) => {
            for part in parts.iter_mut() {
                if let Some(obj) = part.as_object() {
                    if obj
                        .get("type")
                        .and_then(|t| t.as_str())
                        .map(|t| t == "tool_result" || t == "tool-result")
                        .unwrap_or(false)
                    {
                        *part = json!({
                            "type": "text",
                            "text": "[tool result removed]"
                        });
                    } else {
                        strip_tool_results_in_content(part);
                    }
                }
            }
        }
        Value::Object(map) => {
            if map
                .get("type")
                .and_then(|t| t.as_str())
                .map(|t| t == "tool_result" || t == "tool-result")
                .unwrap_or(false)
            {
                *content = json!({
                    "type": "text",
                    "text": "[tool result removed]"
                });
            } else {
                for v in map.values_mut() {
                    strip_tool_results_in_content(v);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn micro_compact_strips_old_tool_results() {
        let c = Compactor::new(CompactionTier::Micro);
        let mut messages = vec![
            json!({"role": "user", "content": "hi"}),
            json!({
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "ok"},
                    {"type": "tool_result", "tool_use_id": "1", "content": "SECRET"}
                ]
            }),
            json!({"role": "user", "content": "next"}),
        ];
        c.micro_compact(&mut messages, 1);
        let assistant = &messages[1];
        let content = assistant.get("content").unwrap().as_array().unwrap();
        assert_eq!(content[1]["type"], "text");
        assert_eq!(content[1]["text"], "[tool result removed]");
        // last message unchanged count: keep_last=1 means we compact first two messages
        assert_eq!(messages[2]["content"], "next");
    }

    #[test]
    fn reactive_compact_truncates_and_prepends_note() {
        let c = Compactor::new(CompactionTier::Reactive);
        let mut messages: Vec<Value> = (0..5)
            .map(|i| json!({"role": "user", "content": i}))
            .collect();
        c.reactive_compact(&mut messages, 2);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["content"], "[earlier context truncated]");
        assert_eq!(messages[1]["content"], 4);
    }
}
