# Beru — Change Workflow & Verification Playbook

Use this decision process to apply the highest-quality changes safely and quickly.

---

## 1. Best-Practice Logic for Applying Changes

### Step 1: Locate the Owning Crate & Single Source of Truth
Map the requirement to the right architectural boundary before editing:
- Manifest / lockfile schema, dependency specs, inline `/// beru` parsing -> `beru-manifest`
- Cache directories, ABI keys, compiler probing -> `beru-core`
- `recipe.toml` schema, resolution tiers, tarball/git download & hashing -> `beru-recipe`
- Version range parsing, PubGrub provider, `Beru.lock` generation -> `beru-resolve`
- CMake invocation, cache validation, dependency building, toolchain generation, prefix inspection, ad-hoc script building -> `beru-build`
- CLI flags, subcommands, user formatting (`console`/`indicatif`), setup/self-update -> `beru-cli`

Search the crate first (`rg -n`) to reuse existing helpers instead of writing parallel implementations.

### Step 2: Design Before Coding (Invariants & Performance)
Check these five questions before writing a line of code:
1. **Hot-path impact:** Does this add any subprocess spawn, network call, filesystem walk, or extra `CMake` configure to cached builds? If yes, eliminate or memoize it.
2. **Cache/ABI safety:** Could a change alter compiled output (compiler flags, standard, shared/static, features)? If yes, ensure `AbiProfile` / `adhoc` hash / `CMakeCache.txt` validation accounts for it.
3. **Offline & partial-failure behavior:** Does `BERU_OFFLINE=1` / `--offline` fail fast with an actionable error? Do failed downloads/extractions clean up temp/partial dirs cleanly?
4. **Windows & cross-platform behavior:** Are paths normalized (`/` vs `\`, `\\?\` stripped, `MAX_PATH` respected)? Are CMake compiler flags guarded with `$<$<CXX_COMPILER_ID:MSVC>:...>` vs non-MSVC generator expressions?
5. **Backward compatibility:** Do existing `Beru.toml`, `Beru.lock`, `recipe.toml`, and CMake projects continue working without manual edits?

### Step 3: Write Clean, Idiomatic Rust (2024 Edition)
- Keep functions short, cohesive, and shallowly nested; use early returns.
- Extract shared helpers rather than duplicating blocks across `build.rs` / `check.rs` / `toolchain.rs` / `deps.rs`.
- Add `///` doc comments on every `pub` symbol (`#![warn(missing_docs)]` fails CI under `-D warnings`).
- Comment non-obvious *why* invariants (especially path normalization, cache keying, CMake quirks, and performance shortcuts).
- Preserve unrelated existing comments and docstrings.

### Step 4: Test & Verify Locally

Write unit tests (`#[cfg(test)]`) alongside pure logic (`abi`, `toolchain`, `manifest`, `recipe`, `resolve`, `cmake`, `discover_cmake_info`) and CLI integration tests under `crates/beru-cli/tests/testsuite/` using `ProjectBuilder` + `BERU_HOME` sandbox isolation.

#### Fast Iteration Commands
```powershell
# Targeted crate unit tests
cargo test -p beru-build

# Targeted integration test(s)
cargo test -p beru --test testsuite build::
```

#### Full Pre-Commit / Pre-Push Verification
```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

*(Note: `.githooks/pre-push` also runs `--all-features` on clippy and `--test-threads=1` on Unix.)*

---

## 2. Troubleshooting This Machine / Environment

- **Rust compiler OOM (`rustc-LLVM ERROR: out of memory`):**
  Run with incremental compilation disabled and single build job if needed:
  ```powershell
  $env:CARGO_INCREMENTAL = "0"
  cargo test --workspace -j 1
  ```
- **Corrupted incremental cache (`only metadata stub found for rlib dependency`):**
  ```powershell
  Remove-Item -Recurse -Force target/debug/incremental
  ```
- **Sandbox network restrictions:**
  Commands/tests requiring external GitHub/HTTP access (such as `beru index update`, online recipe fetches, or `run::test_beru_run_inline_manifest_zero_context`) fail with DNS resolution errors inside a network-isolated sandbox. Never weaken tests or fake network responses — note the exact scope when reporting test results.
- **Git `index.lock` permissions:**
  In restricted sandboxes on Windows, run mutating Git operations (`git add`, `git commit`, `git push`) natively / outside sandbox isolation if lockfile creation is blocked.

