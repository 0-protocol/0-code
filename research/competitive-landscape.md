# Competitive Landscape — Agentic Coding CLIs & SDKs

Short survey of adjacent projects often compared to **terminal-first coding agents** and **0-code**. Figures (speed, size, memory) come from **vendor or community benchmarks** and should be revalidated for your hardware and workload.

---

## Claw Code

- **Stack:** Python + Rust (hybrid).
- **Tools:** ~**19** integrated tools (filesystem, shell, search, git—exact set varies by release).
- **Editing:** **AST-based** edits where available to reduce malformed patches.
- **Positioning:** Balance between **rapid iteration** (Python) and **performance-critical** paths (Rust).
- **Relevance:** Comparable to “full IDE agent” scope with emphasis on **structurally safe** edits.

---

## Cersei / Abstract

- **Stack:** **Pure Rust** SDK and runtime.
- **Performance (reported):** ~**8.2×** faster startup, ~**29×** smaller binary, ~**68×** lower memory vs typical Electron/Node baselines in their materials (workload-specific).
- **Positioning:** **Minimal footprint** agents for embedding in other products or CI.
- **Relevance:** Strong reference for **Rust-native** agent cores and **resource discipline**.

---

## jcode

- **Stack:** **Rust** TUI.
- **Tools:** **30+** tools.
- **Startup (reported):** ~**13.6 ms** cold-start class (microbenchmark-style; verify locally).
- **Positioning:** Fast terminal UX with **broad tool surface**.
- **Relevance:** Benchmark competitor for **latency-sensitive** interactive sessions.

---

## Koda

- **Stack:** **Rust**, **single binary** distribution.
- **Providers:** ~**14** LLM backends (numbers change with releases).
- **Tools:** **20+** built-in tools.
- **Positioning:** **Polyglot** model support with **low ops overhead** (one artifact to ship).
- **Relevance:** Useful baseline for **multi-provider** parity and **packaging** simplicity.

---

## OpenCode

- **Stack:** **Go**.
- **License:** **MIT**.
- **Providers:** **Multi-provider** (OpenAI-compatible APIs, others—check upstream docs).
- **Positioning:** Open, **community-driven** alternative with **simple deployment** stories.
- **Relevance:** Ecosystem proof that **non-Rust** stacks can win on **distribution** and **contributor onboarding**.

---

## How 0-code fits

**0-code** differentiates on an **agent-native** execution model (workspace crates: `zero-core`, `zero-tools`, `zero-perms`, `zero-context`, and LLM integration) rather than maximizing tool count alone. Use this landscape to **borrow patterns** (AST edits, Rust startup, multi-provider configs) while keeping the **graph-first** architecture coherent.

---

## Disclaimer

Product names and metrics are **descriptive**, not endorsements. Verify licensing, telemetry, and security models before adoption in regulated environments.
