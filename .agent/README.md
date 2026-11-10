# Beru — AI Agent Onboarding Guide

Welcome to **Beru**, a declarative C++ package manager and CMake build orchestrator written in Rust (Edition 2024, MSRV 1.85).

Beru replaces ad-hoc CMake dependency plumbing with declarative `Beru.toml` manifests, resolves version graphs with the PubGrub SAT solver into `Beru.lock`, caches compiled C++ dependencies globally under ABI-isolated keys (`~/.beru/cache/builds/<abi-hash>/<pkg>-<ver>/`), and synthesizes `beru-toolchain.cmake` for seamless native builds and IDE integration.

Read these documents before making changes:

| Document | Purpose |
| :--- | :--- |
| [RULES.md](./RULES.md) | Non-negotiable constraints, what NOT to do, safety, compatibility, and lint rules |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Crate dependency graph, 4-stage pipeline, data flow, and cross-platform details |
| [PERFORMANCE.md](./PERFORMANCE.md) | Core latency principles, hot-path optimization rules, and identified bottlenecks |
| [WORKFLOW.md](./WORKFLOW.md) | Step-by-step change methodology, testing strategy, and CI verification |

---

## Workspace Map

Beru is structured as a 6-crate Cargo workspace (`Cargo.toml` at repo root):

| Crate | Path | Responsibility |
| :--- | :--- | :--- |
| `beru-core` | `crates/beru-core/` | ABI profiling (`AbiProfile`), compiler detection (`detect_compiler`), and global cache paths (`BeruCache`) |
| `beru-manifest` | `crates/beru-manifest/` | `Beru.toml` / `Beru.lock` / inline `/// beru` script manifest parsing, validation, and errors |
| `beru-recipe` | `crates/beru-recipe/` | `recipe.toml` schema, 5-tier recipe lookup + embedded fallbacks, tarball/git fetching with SHA-256 verification |
| `beru-resolve` | `crates/beru-resolve/` | PubGrub SAT solver integration (`BeruProvider`), version range parsing, and lockfile generation (`resolve_graph`) |
| `beru-build` | `crates/beru-build/` | Dependency build orchestration, CMake configure/build/install, custom scripts, ad-hoc builds, and `beru-toolchain.cmake` synthesis |
| `beru` | `crates/beru-cli/` | `clap` CLI (`src/main.rs`, 23 subcommands in `src/commands/`), bundled recipes (`recipes/`), integration tests (`tests/testsuite/`) |

All crates enforce `#![warn(missing_docs)]` at `src/lib.rs` (or `src/main.rs`).

---

## Environment Variables & CLI Flags

- `BERU_HOME` — Overrides `~/.beru` root (used heavily by integration tests for sandbox isolation).
- `BERU_JOBS` — Parallel job count (also set via global `-j` / `--jobs`).
- `BERU_OFFLINE` — Disables network calls when set (also set via global `--offline`).
- `BERU_LOG` — `tracing_subscriber::EnvFilter` directive (defaults to `info,pubgrub=warn`).

---

## Mandatory Verification Before Finishing

Every PR and push must pass the CI matrix (`macos-latest`, `windows-latest`, `debian:latest`, `fedora:latest`, `archlinux:latest`):

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

> **Low-Memory Tip (Windows / constrained hosts):** If `rustc` hits LLVM out-of-memory errors during incremental builds, set `$env:CARGO_INCREMENTAL = "0"` and build with `-j 1`. If an interrupted build corrupts incremental state (`only metadata stub found`), delete `target/debug/incremental`.
>
> **Offline Sandbox Note:** Tests requiring live DNS/network (such as `run::test_beru_run_inline_manifest_zero_context` if fetching remote recipes) require network access; verify offline-capable test subsets explicitly and report exact test scope.

