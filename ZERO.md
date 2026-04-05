# ZERO.md — 0-code Project Memory

## Architecture Decisions

- Rust workspace with 12 crates under `crates/`
- Binary entry point: `zero-cli` → `zero-code`
- 0-lang integration via `zero-protocol` crate (deferred to Phase 6)
- Hybrid mode: Rust-native agent loop with 0-lang graph nodes for tool results

## Conventions

- All public types use `#[derive(Debug, Clone)]` at minimum
- Error types per crate via `thiserror`
- Async runtime: tokio (full features)
- Serialization: serde + serde_json for config/API, Cap'n Proto for wire format (via 0-lang)
- Tracing: use `tracing` crate for structured logging, not `println!` or `log`

## Key Design Principles

- Agent-native: tools are graph nodes, results are tensors, not strings
- Content-addressable: tools referenced by schema hash
- Proof-carrying: operations carry halting/shape proofs verified before execution
- Probabilistic permissions: confidence thresholds, not boolean allow/deny
- Web3 native: on-chain operations are first-class graph nodes
