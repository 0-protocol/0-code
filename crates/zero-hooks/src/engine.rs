use crate::{HookConfig, HookContext, HookResult, HookType, LifecycleEvent};
use std::collections::HashMap;
use tracing::{debug, warn};

pub struct HookEngine {
    hooks: HashMap<LifecycleEvent, Vec<HookConfig>>,
}

impl HookEngine {
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    pub fn register(&mut self, config: HookConfig) {
        self.hooks.entry(config.event).or_default().push(config);
    }

    pub async fn fire(&self, ctx: &HookContext) -> HookResult {
        let hooks = match self.hooks.get(&ctx.event) {
            Some(h) => h,
            None => return HookResult::Continue,
        };

        for hook in hooks {
            debug!(hook = %hook.name, event = ?ctx.event, "firing hook");

            let result = match hook.hook_type {
                HookType::Command => self.run_command_hook(hook, ctx).await,
                HookType::Prompt => HookResult::Continue,
                HookType::Function => HookResult::Continue,
            };

            if result != HookResult::Continue {
                debug!(hook = %hook.name, result = ?result, "hook short-circuited");
                return result;
            }
        }

        HookResult::Continue
    }

    async fn run_command_hook(&self, hook: &HookConfig, ctx: &HookContext) -> HookResult {
        let command = match &hook.command {
            Some(cmd) => cmd,
            None => {
                warn!(hook = %hook.name, "command hook has no command configured");
                return HookResult::Error;
            }
        };

        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(command);

        cmd.env("ZERO_EVENT", format!("{:?}", ctx.event));
        cmd.env("ZERO_SESSION_ID", &ctx.session_id);
        if let Some(tool) = &ctx.tool_name {
            cmd.env("ZERO_TOOL_NAME", tool);
        }

        match cmd.status().await {
            Ok(status) => match status.code() {
                Some(0) => HookResult::Continue,
                Some(2) => HookResult::Block,
                other => {
                    warn!(hook = %hook.name, exit_code = ?other, "command hook failed");
                    HookResult::Error
                }
            },
            Err(e) => {
                warn!(hook = %hook.name, error = %e, "failed to spawn command hook");
                HookResult::Error
            }
        }
    }

    pub fn hooks_for_event(&self, event: LifecycleEvent) -> Vec<&HookConfig> {
        self.hooks
            .get(&event)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }
}

impl Default for HookEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(event: LifecycleEvent) -> HookContext {
        HookContext {
            event,
            tool_name: Some("test_tool".into()),
            tool_input: None,
            tool_result: None,
            session_id: "test-session".into(),
        }
    }

    #[test]
    fn test_register_and_lookup() {
        let mut engine = HookEngine::new();

        engine.register(HookConfig {
            name: "pre-check".into(),
            event: LifecycleEvent::PreToolUse,
            hook_type: HookType::Command,
            command: Some("echo ok".into()),
            prompt: None,
        });

        engine.register(HookConfig {
            name: "post-log".into(),
            event: LifecycleEvent::PostToolUse,
            hook_type: HookType::Command,
            command: Some("echo done".into()),
            prompt: None,
        });

        assert_eq!(engine.hooks_for_event(LifecycleEvent::PreToolUse).len(), 1);
        assert_eq!(engine.hooks_for_event(LifecycleEvent::PostToolUse).len(), 1);
        assert_eq!(engine.hooks_for_event(LifecycleEvent::SessionStart).len(), 0);
    }

    #[test]
    fn test_multiple_hooks_same_event() {
        let mut engine = HookEngine::new();

        for i in 0..3 {
            engine.register(HookConfig {
                name: format!("hook-{i}"),
                event: LifecycleEvent::PreToolUse,
                hook_type: HookType::Function,
                command: None,
                prompt: None,
            });
        }

        assert_eq!(engine.hooks_for_event(LifecycleEvent::PreToolUse).len(), 3);
    }

    #[tokio::test]
    async fn test_fire_no_hooks() {
        let engine = HookEngine::new();
        let ctx = make_ctx(LifecycleEvent::SessionStart);
        assert_eq!(engine.fire(&ctx).await, HookResult::Continue);
    }

    #[tokio::test]
    async fn test_fire_command_success() {
        let mut engine = HookEngine::new();
        engine.register(HookConfig {
            name: "ok-hook".into(),
            event: LifecycleEvent::PreToolUse,
            hook_type: HookType::Command,
            command: Some("exit 0".into()),
            prompt: None,
        });

        let ctx = make_ctx(LifecycleEvent::PreToolUse);
        assert_eq!(engine.fire(&ctx).await, HookResult::Continue);
    }

    #[tokio::test]
    async fn test_fire_command_block() {
        let mut engine = HookEngine::new();
        engine.register(HookConfig {
            name: "block-hook".into(),
            event: LifecycleEvent::PreToolUse,
            hook_type: HookType::Command,
            command: Some("exit 2".into()),
            prompt: None,
        });

        let ctx = make_ctx(LifecycleEvent::PreToolUse);
        assert_eq!(engine.fire(&ctx).await, HookResult::Block);
    }

    #[tokio::test]
    async fn test_fire_command_error() {
        let mut engine = HookEngine::new();
        engine.register(HookConfig {
            name: "err-hook".into(),
            event: LifecycleEvent::PreToolUse,
            hook_type: HookType::Command,
            command: Some("exit 1".into()),
            prompt: None,
        });

        let ctx = make_ctx(LifecycleEvent::PreToolUse);
        assert_eq!(engine.fire(&ctx).await, HookResult::Error);
    }

    #[tokio::test]
    async fn test_fire_command_missing_command() {
        let mut engine = HookEngine::new();
        engine.register(HookConfig {
            name: "no-cmd".into(),
            event: LifecycleEvent::PreToolUse,
            hook_type: HookType::Command,
            command: None,
            prompt: None,
        });

        let ctx = make_ctx(LifecycleEvent::PreToolUse);
        assert_eq!(engine.fire(&ctx).await, HookResult::Error);
    }

    #[tokio::test]
    async fn test_fire_short_circuits_on_block() {
        let mut engine = HookEngine::new();

        engine.register(HookConfig {
            name: "blocker".into(),
            event: LifecycleEvent::PreToolUse,
            hook_type: HookType::Command,
            command: Some("exit 2".into()),
            prompt: None,
        });

        // This hook should never run
        engine.register(HookConfig {
            name: "after-blocker".into(),
            event: LifecycleEvent::PreToolUse,
            hook_type: HookType::Command,
            command: Some("exit 0".into()),
            prompt: None,
        });

        let ctx = make_ctx(LifecycleEvent::PreToolUse);
        assert_eq!(engine.fire(&ctx).await, HookResult::Block);
    }

    #[tokio::test]
    async fn test_fire_function_and_prompt_pass_through() {
        let mut engine = HookEngine::new();

        engine.register(HookConfig {
            name: "fn-hook".into(),
            event: LifecycleEvent::Stop,
            hook_type: HookType::Function,
            command: None,
            prompt: None,
        });

        engine.register(HookConfig {
            name: "prompt-hook".into(),
            event: LifecycleEvent::Stop,
            hook_type: HookType::Prompt,
            command: None,
            prompt: Some("check safety".into()),
        });

        let ctx = make_ctx(LifecycleEvent::Stop);
        assert_eq!(engine.fire(&ctx).await, HookResult::Continue);
    }
}
