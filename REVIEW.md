# Review -- All Phases: 0-code Agent-Native Coding Tool

## Verdict: PASS

Two audit rounds performed. Round 1 found 7 Critical, 19 Major, and 22 Minor issues across 12 crates (70 source files, ~6300 lines of Rust). All Critical and top Major issues were fixed in Round 1. Round 2 found one remaining path traversal edge case in `normalize_path`, which was fixed. Round 2 re-audit confirmed all 10 patched files are correct with no regressions.

## Scores

| Dimension | Score | Notes |
|-----------|-------|-------|
| Functionality | 4/5 | Core agent loop, 8 tools, permission pipeline, context compaction, multi-agent, hooks, skills, MCP, TUI, and protocol integration all present and tested. WebFetch/WebSearch are stubs (documented). Auto-mode LLM classifier is placeholder. |
| Design quality | 4/5 | Clean 12-crate workspace with clear separation of concerns. Trait-based abstractions (ModelProvider, Tool, PermissionLayer). Consistent error handling via thiserror. |
| Code quality | 4/5 | 62 tests passing, 0 clippy warnings, 0 compiler warnings. UTF-8 safety verified in SSE parser and TUI. Async correctness (no mutex-across-await). |
| Security & robustness | 4/5 | Multi-layer permission pipeline with path traversal protection. kill_on_drop for bash. Dangerous file/command detection. |
| Accessibility | 3/5 | N/A for TUI (no HTML/web). Keyboard controls present. Multi-byte input fully supported. |

**Weighted score: 3.95/5**

## Issues Fixed (Round 1 + 2)

### Critical Fixes (7)

| # | File | Issue | Fix |
|---|------|-------|-----|
| C1 | `zero-llm/messages_api.rs` | UTF-8 corruption at SSE chunk boundaries | Byte buffer with valid-UTF8-boundary draining |
| C2 | `zero-tools/bash.rs` | Orphaned process on timeout | `kill_on_drop(true)` |
| C3 | `zero-perms/rules.rs` | Tool name case mismatch | Match both casings |
| C4 | `zero-perms/pipeline.rs` | Bare `..` bypasses path traversal check | `normalize_path_checked` rejects upward escapes |
| C5 | `zero-perms/pipeline.rs` | AcceptEdits bypass via `.any()` on all strings | `.all()` on path-only keys |
| C6 | `zero-agents/team.rs` | `std::sync::Mutex` held across `.await` | Drop guard before await, poison recovery |
| C7 | `zero-tui/ui.rs` | Scroll arguments swapped | `(vertical, horizontal)` corrected |

### Major Fixes (12)

| # | File | Issue | Fix |
|---|------|-------|-----|
| M1 | `zero-core/engine.rs` | No cancellation | Detect closed channel, return `Aborted` |
| M2 | `zero-core/engine.rs` | Tool defs silently dropped | Log warning per failure |
| M3 | `zero-core/engine.rs` | No retry backoff | Exponential backoff with cap |
| M4 | `zero-core/engine.rs` | Max-turns exit produces no event | Send error event before break |
| M5 | `zero-llm/messages_api.rs` | No HTTP request timeout | connect_timeout(10s), timeout(300s) |
| M6 | `zero-perms/pipeline.rs` | Static rules scan content strings | Only scan danger-relevant keys |
| M7 | `zero-perms/pipeline.rs` | normalize_path didn't resolve components | Full component resolution |
| M8 | `zero-perms/pipeline.rs` | normalize_path dropped upward traversals | Returns None on escape |
| M9 | `zero-protocol/dex.rs` | Stub returns fake success | Returns Err(NotImplemented) |
| M10 | `zero-protocol/ads.rs` | Stub returns fake success | Returns Err(NotImplemented) |
| M11 | `zero-tui/run.rs` | TUI panics on multi-byte input | Char-boundary-aware cursor |
| M12 | `zero-context/compaction.rs` | Clippy ptr_arg warning | `&mut [Value]` |

## Remaining Known Issues (not blocking)

| # | Severity | File | Issue | Mitigation |
|---|----------|------|-------|------------|
| 1 | Major | `zero-tools/bash.rs` | Full env inherited | Future: `env_clear()` with whitelist |
| 2 | Major | `zero-tools/registry.rs` | Batch reorders calls | Future: preserve order |
| 3 | Major | `zero-tools/registry.rs` | Panicked tasks silently dropped | Future: push error results |
| 4 | Major | `zero-tools/file_write.rs` | No write size limit | Future: MAX_WRITE_SIZE |
| 5 | Major | `zero-tools/glob_tool.rs` | No result limit | Future: max_results param |
| 6 | Major | `zero-mcp/stdio.rs` | No request-response correlation | Future: id-based demux |
| 7 | Major | `zero-hooks/engine.rs` | Hook commands unsandboxed | Future: timeout + opt-in flag |
| 8 | Minor | Web stubs | Registered but always error | Future: feature-gate |
| 9 | Minor | `zero-perms/pipeline.rs` | Auto mode is a no-op | Future: implement classifier |
| 10 | Minor | Cross-platform | HOME env var not on Windows | Future: dirs::home_dir() |

## Passed Checks

- [x] 62 unit/integration tests pass (0 failures)
- [x] 0 clippy warnings across all 12 crates
- [x] 0 compiler warnings
- [x] Release binary builds successfully
- [x] Startup benchmark: <50ms
- [x] UTF-8 safety verified (SSE parser + TUI input)
- [x] Async safety (no mutex held across await)
- [x] Path safety (traversal attacks rejected)
- [x] Process safety (kill_on_drop for child processes)
- [x] API safety (HTTP timeouts configured)
- [x] Protocol safety (stubs return errors, not fake success)
