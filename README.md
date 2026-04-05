# 0-code

**Agent-native coding tool** — structured execution, composable permissions, and graph-first context for serious software work.

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

## Positioning

0-code is an **agent-native coding tool**: the runtime treats plans, tools, and workspace state as **typed, schedulable structure** (not only a linear chat transcript). That enables safer automation, clearer audit trails, and native integration with complex workflows—including on-chain and multi-agent scenarios.

## Key differentiators

- **DAG-first execution** — Task graphs with explicit dependencies instead of ad-hoc string streams.
- **Graded permissions** — Probabilistic and budget-aware policies, not only single-shot allow/deny prompts.
- **Graph-native context** — Prune and hydrate structure; avoid summarizing away the relationships that matter.
- **Structured multi-agent** — Shared execution graphs and typed handoffs instead of opaque XML chat relays.
- **Proof-carrying safety** — Attach verifiable intent and policy evidence where the toolchain supports it.
- **Web3 as first-class nodes** — Chain reads, simulation, and submission as native graph operations, not only bolt-on MCP scripts.

## Architecture overview

Rust workspace (`Cargo.toml`) with focused crates:

| Crate | Role |
|--------|------|
| `zero-core` | Orchestration, session loop, integration between subsystems |
| `zero-tools` | Tool registry, dispatch, and execution adapters |
| `zero-perms` | Permission pipeline and policy composition |
| `zero-context` | Context graph, compaction, and workspace memory |
| `zero-llm` | LLM provider abstraction (configurable Messages API, OpenAI-compatible) |
| `zero-agents` | Multi-agent: SubAgent, Coordinator, Team |
| `zero-hooks` | Lifecycle events and hook engine |
| `zero-skills` | Skill loader, matcher, and frontmatter parser |
| `zero-mcp` | MCP client (stdio, SSE, WebSocket transports) |
| `zero-protocol` | 0-protocol integration (oracle, dex, ads) |
| `zero-tui` | ratatui-based terminal UI |
| `zero-cli` | Binary entry point |

## Getting started

Build the workspace:

```bash
cargo build --release
```

### Flock (default provider)

[Flock](https://docs.flock.io/flock-products/api-platform/api-endpoint) is the default LLM provider (OpenAI-compatible):

```bash
export FLOCK_API_KEY=your-key
cargo run -p zero-cli --bin zero-code -- --interactive
```

One-shot mode:

```bash
cargo run -p zero-cli --bin zero-code -- "Explain this codebase"
```

### Other providers

```bash
# OpenAI
export OPENAI_API_KEY=your-key
cargo run -p zero-cli --bin zero-code -- --provider openai --interactive

# Messages API (generic SSE)
export ZERO_CODE_API_KEY=your-key
cargo run -p zero-cli --bin zero-code -- --provider messages-api --interactive
```

### Provider options

| Provider | Env var | Default model | Protocol |
|----------|---------|---------------|----------|
| `flock` | `FLOCK_API_KEY` | `qwen3-30b-a3b-instruct-2507` | OpenAI-compatible |
| `openai` | `OPENAI_API_KEY` | `gpt-4o` | OpenAI-compatible |
| `messages-api` | `ZERO_CODE_API_KEY` | (configurable) | Messages API SSE |

## License

This project is licensed under the **Apache License 2.0** — see [LICENSE](LICENSE).

## Contributing

Contributions are coordinated through the parent **0-protocol** repository: [github.com/0-protocol/0-protocol](https://github.com/0-protocol/0-protocol).
