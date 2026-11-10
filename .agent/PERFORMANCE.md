# Beru — Performance Engineering Guide

**Goal:** Beru must feel instantaneous on cached/incremental workflows (`check`, `build`, `run`, `resolve`, `tree`) and saturate available parallelism on cold builds. Every design choice and code change should protect or improve wall-clock latency.

---

## 1. Where Time Actually Goes in Beru

In priority order from highest impact to lowest:

1. **Subprocess spawns (`cmake`, `git`, `c++` compiler probes)** — ~30 ms to 500+ ms *per process* on Windows/CI, plus seconds/minutes when CMake builds run. Eliminating one redundant subprocess beats a thousand micro-allocations.
2. **Cold CMake configure & dependency builds** — Multi-second to multi-minute cost. Cache hits, parallel compilation (`--parallel`), and skipping dependency tests/docs (`-DBUILD_TESTING=OFF`) dominate.
3. **Network operations (`git ls-remote`, `git fetch`, tarball HTTP downloads)** — Latency + TLS setup cost. Must be avoided on cached paths and minimized via shallow clones, connection reuse, and batched/deduplicated metadata lookups.
4. **Filesystem traversal & repeated parsing** — Stat storms across large install prefixes (`Eigen`, `Boost`) or re-parsing `recipe.toml` / `CMakeCache.txt` multiple times per command.
5. **Memory allocations & string formatting** — Avoid quadratic `Vec`/`String` copies and redundant `.clone()` calls inside resolver/introspection loops.

---

## 2. Strict Performance Principles for All Changes

### Principle A: Cache-First Short-Circuiting
- Always check the cheapest cache signal first (`cache.has_build(...)`, `adhoc_build_dir` binary existence, `Beru.lock` validity) **before** touching recipes, vendor paths, Git, HTTP, or `which`.
- Keep lockfile staleness checks cheap and local (compare manifest dependencies/sources against `Beru.lock` without spawning `git` or network).

### Principle B: Probe Once, Reuse Everywhere
- Never call `which::which("cmake")` or `detect_compiler()` multiple times for the same operation when the resolved `PathBuf` / `CompilerInfo` can be passed down.
- Be especially careful with `beru_core::toolchain::build_abi_profile()` when `cxx_std` is `20`/`23`/`26`: `supports_concepts` and `supports_modules` currently invoke the C++ compiler twice to compile dummy `.cpp` files in temp storage. Cache probe results by `(compiler_path, compiler_version, cxx_std)` (in-memory per process or on disk keyed by compiler mtime/version) instead of repeating feature test compiles across workspace members or commands!
- In `beru-cli/src/commands/build.rs`, `check.rs`, etc., workspace builds should reuse host toolchain detection and dependency introspection rather than re-probing per member.

### Principle C: Avoid Redundant Disk/Recipe Introspection
Look closely at existing duplication patterns and **do not worsen them (and eliminate them when touching nearby code)**:
- In `crates/beru-build/src/deps.rs` (`prepare_locked_dependencies`):
  - `resolve_and_build_locked_dep` calls `find_dep_recipe(...)`, and then `prepare_locked_dependencies` calls `find_dep_recipe(...)` a second time for the same package before calling `inspect_installed_dependency`. Return or reuse the already-loaded `Option<Recipe>` instead of reading & parsing `recipe.toml` twice!
- In `crates/beru-build/src/toolchain.rs` (`generate_toolchain_cmake_with_config`):
  - `prepare_locked_dependencies` already calls `inspect_installed_dependency` (which calls `discover_cmake_info` + `collect_include_dirs_with`), producing `CMakeDependency { include_dirs, ... }`.
  - Yet `generate_toolchain_cmake_with_config` also loops over `prefix_paths` and calls `collect_prefix_include_dirs(path)` -> `discover_cmake_info(prefix, "")`, walking every prefix directory tree a *second* time! Prefer reusing `cmake_deps` introspection directly or guarding so each prefix is scanned at most once.

### Principle D: Smart CMake Invocation
- Keep `is_cmake_cache_valid()` fast and accurate so incremental builds do not delete valid `CMakeCache.txt` / `CMakeFiles` or re-trigger `-DCMAKE_TOOLCHAIN_FILE` reconfigure warnings.
- When generating `beru-toolchain.cmake` or dynamic `CMakeLists.txt`, **skip rewriting the file if its byte content is unchanged** (or compare content before write) where timestamp preservation prevents CMake from re-running configure on up-to-date projects! *(Double-check whether user/test workflows expect mtime updates or unchanged builds — avoiding needless mtime bumps on identical `CMakeLists.txt` / `beru-toolchain.cmake` prevents CMake `Rerun CMake` triggers on `cmake --build`!)*.
- Always pass `--parallel` (or `--parallel <BERU_JOBS>`) to `cmake --build`.
- Keep dependency CMake configure lean (`-DBUILD_TESTING=OFF`, `-DFMT_TEST=OFF`, `-DFMT_DOC=OFF`, etc.).

### Principle E: Network, Git & Archive Speed
- Reuse a shared `reqwest::blocking::Client` when downloading multiple tarballs in one command so TLS sessions and HTTP keep-alive connections are reused (do not rebuild `Client::builder()` inside a loop).
- Always fetch Git sources with `--depth 1`.
- Never run `git ls-remote` redundantly for unchanged dependencies or in `--offline` mode (`BERU_OFFLINE`).
- Stream or buffer cleanly without double-reading compressed tarballs; verify SHA-256 directly on the downloaded buffer before disk extraction.

### Principle F: Parallelize Safe Independent Work
- Independent non-conflicting dependencies (e.g., sibling leaf packages in `Beru.lock` whose build dirs and source dirs are distinct) and multi-tarball downloads are strong candidates for bounded concurrency (`std::thread::scope` / worker pool capped by available parallelism or `BERU_JOBS`), provided:
  1. Distinct `source_dir` / `dest_dir` / `install_prefix` paths are guaranteed not to race (e.g., if two versions share a git repo dir, synchronize by repo key).
  2. `stdout`/`tracing` output remains readable and deterministic.
  3. Errors propagate cleanly without corrupting shared cache directories.

### Principle G: Bounded Walks & Zero-Copy Hot Loops
- Always sort/bound filesystem walks (`collect_cmake_files_bounded` `max_depth = 5`), skip irrelevant subdirectories early, and check extension via `path.extension()` without allocating `String`s.
- Use `HashMap` / `HashSet` or sorted `Vec` binary search instead of $O(N^2)$ linear scans when graphs grow large (e.g., PubGrub provider caches in `BeruProvider`, `seen_includes` / `seen_targets`).
- Avoid rebuilding normalized strings repeatedly (`normalize_cmake_path`, `to_ascii_lowercase`) inside nested loops.

---

## 3. High-ROI Optimization Targets in Current Codebase

Agents tasked with improving speed should prioritize inspecting these areas first:

1. **Duplicate prefix inspection (`beru-build/src/toolchain.rs` & `deps.rs`):** Eliminate scanning each dependency install prefix twice (`inspect_installed_dependency` + `collect_prefix_include_dirs`) and parsing `recipe.toml` twice per dependency.
2. **Compiler feature probe caching (`beru-core/src/toolchain.rs`):** Avoid compiling 2 temporary C++ files (`supports_concepts`, `supports_modules`) on *every* C++20/23/26 CLI invocation when compiler path + version + `cxx_std` haven't changed.
3. **Unchanged generated CMake files (`beru-build/src/toolchain.rs`, `beru-cli/src/commands/build.rs`, `check.rs`):** Write-if-changed for `beru-toolchain.cmake` / `beru-override.cmake` / generated `CMakeLists.txt` so incremental `cmake --build` avoids pointless CMake reconfigure steps.
4. **Topological parallel dependency builds (`beru-build/src/deps.rs`):** Build independent dependency graph layers concurrently while sharing a `reqwest::blocking::Client` and per-prefix `cmake` path lookup.
5. **Parallel workspace member builds (`beru-cli/src/commands/build.rs`):** Share resolved `AbiProfile` and `BeruCache` across workspace members instead of re-probing the compiler per member.
6. **Subprocess-free `git` or early short-circuits (`beru-resolve/src/resolve.rs`):** Note `resolve_git_tag_to_sha` currently runs `git ls-remote` synchronously per git dep even if a 40-char commit rev is already pinned or cached.

---

## 4. Performance Review Checklist

Before completing any performance or feature PR, verify:
- [ ] Zero new subprocess calls added to cached `beru build` / `beru check` / `beru run` hot paths?
- [ ] Cache lookup happens *before* network, Git, or source inspection?
- [ ] No duplicate `recipe.toml`, `Beru.toml`, or install-prefix filesystem scans?
- [ ] Existing ABI hash isolation and SHA-256 verification remain intact?
- [ ] Cross-platform correctness preserved (Windows MSVC/MinGW, Linux, macOS)?

