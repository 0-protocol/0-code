# 0-code Task Tracker

## Completed

- [x] Phase 0: Scaffold (workspace, license, research, ZERO.md)
- [x] Phase 1: Core engine (QueryEngine), LLM provider (MessagesApiProvider), tool registry, 8 core tools, CLI
- [x] Phase 2: Permission pipeline (4 layers, denial tracker), context management (4-tier compaction, file cache, memory)
- [x] Phase 3: Multi-agent (SubAgent, Coordinator, Team, AgentTool)
- [x] Phase 4: Hooks (7 events), skills (loader + matcher), MCP client (stdio transport)
- [x] Phase 5: TUI (ratatui, streaming, scroll, input)
- [x] Phase 6: Protocol integration (oracle, dex stub, ads stub, verifier)
- [x] Phase 7: Polish (integration tests, startup benchmark, zero warnings)
- [x] Harness review (2 rounds, PASS)
- [x] IP-clean refactor (all vendor references removed)

## Remaining

- [ ] Wire permissions into engine (gate tool execution via PermissionManager)
- [ ] Wire context management into engine (compaction triggers in agent loop)
- [ ] Wire hooks into engine (PreToolUse, PostToolUse lifecycle events)
- [ ] Wire skills into CLI (load on startup, inject into system prompt)
- [ ] Wire MCP into tool registry (deferred-loaded tools)
- [ ] E2E test with real LLM API
- [ ] 0-lang integration (zerolang dependency in zero-protocol, Tensor bridge)
- [ ] CI pipeline (GitHub Actions)
