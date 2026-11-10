# Beru — Rules & Constraints ("What Not To Do")

Follow every rule below strictly when modifying Beru.

---

## 1. Performance & Latency Rules (Core Product Mandate)

Beru exists to be **as fast as possible**. Most end-to-end latency comes from subprocess spawns (`cmake`, `git`, `c++`), filesystem walks, archive extraction, and network I/O — not CPU-bound Rust loops.

- **DO NOT** perform network, Git, tarball, or CMake work before checking `BeruCache::has_build(abi_hash, pkg, version)` (or equivalent ad-hoc binary cache hit). Cache hits must short-circuit immediately.
- **DO NOT** spawn redundant subprocesses (`cmake`, `git`, `c++ --version`, compiler feature probe compiles) in a loop or per-dependency when the value can be probed once and reused.
- **DO NOT** invalidate `CMakeCache.txt` unnecessarily or re-pass `-DCMAKE_TOOLCHAIN_FILE` when `is_cmake_cache_valid()` confirms `CMAKE_HOME_DIRECTORY` and `CMAKE_TOOLCHAIN_FILE` already match.
- **DO NOT** perform unbounded directory traversals (`WalkDir` / infinite recursion) inside install prefixes, project trees, or `~/.beru/`. Prefix introspection (`collect_cmake_files_bounded`) must stay bounded (currently `max_depth = 5` over `lib`, `lib64`, `share`, `cmake`).
- **DO NOT** enable upstream dependency tests, examples, or docs during dependency builds (keep `-DBUILD_TESTING=OFF` and package-specific disable flags in `build_dependency_cmake`).
- **DO NOT** clone full Git histories when a shallow fetch (`--depth 1`) or content-addressed SHA-256 tarball works.
- **DO NOT** block or ignore `--parallel` / `BERU_JOBS` when invoking CMake (`cmake --build`) or custom build commands (`{jobs}`).
- **DO NOT** make hot-path memory allocations (`to_string()`, `.clone()`, `collect::<Vec<_>>()`) when borrowing (`&str`, `&Path`, slices, `Cow<str>`) or pre-sizing (`Vec::with_capacity`) works cleanly.

---

## 2. Correctness, Cache & Security Rules

- **NEVER weaken ABI cache isolation:** `AbiProfile::hash()` hashes compiler family, compiler version, stdlib, architecture, OS, build type, `cxx_std`, `shared_libs`, and sorted `features`. Never omit ABI dimensions for compiled artifacts — poisoning `~/.beru/cache/builds/` causes silent ODR violations and linker failures. Use `header_only_hash()` only for genuinely compiler-independent header-only cache keys.
- **NEVER bypass SHA-256 verification:** Tarball downloads in `beru-recipe/src/fetch.rs` MUST verify the SHA-256 digest over the downloaded bytes before extracting to disk, and MUST remove partially extracted directories if extraction fails.
- **NEVER bypass `BERU_OFFLINE`:** Any code path performing HTTP requests (`reqwest`) or remote Git operations (`git ls-remote`, `git fetch`, `git pull`, `git clone`) MUST check `std::env::var("BERU_OFFLINE").is_ok()` before initiating network I/O and fail fast with a clear message.
- **NEVER re-introduce Windows `MAX_PATH` or backslash bugs:**
  - Keep `BeruCache::source_dir` truncated to the first 16 hex chars of `sha256`.
  - Strip Windows extended-length canonical prefixes (`\\?\`) and normalize `\` to `/` before writing paths into generated CMake files or comparing paths against `CMakeCache.txt`.
  - Remember Windows path comparisons are case-insensitive (`to_ascii_lowercase()` when `cfg!(windows)`).

---

## 3. Backward Compatibility Rules

- **DO NOT break `Beru.toml`, `Beru.lock`, or `recipe.toml` schemas:**
  - Existing fields, defaults (`cxx-std`, `PackageType`, `BuildSystem`, exact-pin version semantics), and TOML key aliases (e.g., `cmake-args` / `args` / `cmake_args`, `include-dirs` / `include_dirs`, `link-libs` / `link_libs`, `cmake-package` / `cmake_package`, `cmake-targets` / `cmake_targets`) must keep deserializing cleanly.
  - Lockfile format (`version = 1`) changes must remain backward-compatible or cleanly migratable.
- **DO NOT break CLI contracts:** Preserve existing subcommand names, aliases (e.g., `rm` for `remove`, `self-update`), flags, exit behavior, and `BERU_HOME` / `BERU_JOBS` / `BERU_OFFLINE` / `BERU_LOG` semantics.
- **DO NOT break CMake target macros:** Existing CMake projects expect `beru-toolchain.cmake` to set `CMAKE_CXX_STANDARD`, `CMAKE_BUILD_TYPE`, `CMAKE_PREFIX_PATH`, and define `beru_link_dependencies(<TARGET_NAME>)`.

---

## 4. Rust Code Style & Crate Hygiene Rules

- **DO NOT leave public items undocumented:** Every workspace crate enables `#![warn(missing_docs)]` and CI treats warnings as errors (`-D warnings`). Every `pub` module, struct, enum, variant, field, function, constant, and method MUST have a doc comment.
- **DO NOT introduce `unsafe` code:** The only permitted `unsafe` calls currently in the codebase are `std::env::set_var` in single-threaded CLI startup/setup (`beru-cli/src/main.rs`, `beru-cli/src/commands/setup.rs`), required by Rust 2024 and annotated with `// SAFETY: ...`. Avoid adding `unsafe` anywhere else.
- **DO NOT use `.unwrap()` or `.expect()` in production code paths** on user input, TOML content, filesystem reads, CLI outputs, or network responses. Propagate errors with context (`thiserror` in library crates, `anyhow::{Context, bail}` in `beru-cli` and high-level orchestrators).
- **DO NOT add new crates/dependencies to `Cargo.toml`** without genuine necessity. Reuse existing workspace dependencies (`serde`, `toml`, `toml_edit`, `sha2`, `reqwest`, `which`, `dirs`, `tempfile`, `thiserror`, `anyhow`, `pubgrub`, `semver`, `tracing`, `console`, `indicatif`, `tar`, `flate2`).
- **DO NOT duplicate logic** across crates or commands (e.g., extract shared helpers inside the lowest appropriate crate rather than copy-pasting between `build`, `check`, `run`, or `test`).
- **DO NOT ignore Rust Edition 2024 / MSRV 1.85 idioms:** Use `let` chains (`if let ... && ...`) cleanly where helpful, keep functions small with shallow nesting and early returns, and handle variants uniformly via data tables/mappings.

---

## 5. Testing, CI & Repository Hygiene Rules

- **DO NOT modify `crates/beru-cli/tests/testsuite/support/mod.rs`** or existing test assertions just to make a failing test pass; fix the underlying bug instead.
- **DO NOT touch real user `~/.beru` in tests:** Always use `ProjectBuilder` / `Project` (`BERU_HOME` set to a `TempDir`) or `BeruCache::with_root(...)`.
- **DO NOT strip unrelated comments or docstrings** when editing files.
- **DO NOT break relative markdown links** in `README.md` or `docs/**/*.md` — CI runs an automated Python link verifier on them.
- **DO NOT edit `.github/workflows/*` or `.githooks/*`** unless your task explicitly targets CI/hooks.
- **DO NOT report tests or checks as passing** unless you actually ran them after your final edit and observed them pass. State exact scope for any network-gated or filtered run.

