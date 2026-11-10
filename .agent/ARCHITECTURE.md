# Beru — System Architecture

## 1. Crate Dependency Graph

Dependencies flow strictly downward (no cycles):

```text
                crates/beru-cli (bin: beru)
                 /            |            \
                v             v             v
          beru-build     beru-resolve    (core/manifest/recipe)
         /    |     \       /    |
        v     v      v     v     v
   beru-resolve   beru-recipe   beru-manifest
                     \     /       /
                      v   v       v
                        beru-core
```

Note: `beru-build` depends on `beru-manifest`, `beru-core`, `beru-recipe`, and `beru-resolve` (for `build_adhoc`). `beru-resolve` depends on `beru-manifest`, `beru-recipe`, and `beru-core`. `beru-core` and `beru-manifest` are leaf crates.

---

## 2. The Four-Stage Build Pipeline

### Stage 1: Manifest Parsing & Dependency Resolution (`beru-manifest`, `beru-resolve`)

1. `BeruManifest::from_dir()` (`crates/beru-manifest/src/manifest.rs`) parses `Beru.toml` into `Package`, `[dependencies]`, `[dev-dependencies]`, `[build]`, `[profile.*]`, `[system-dependencies]`, and `[workspace]`.
   - For single-file scripts without `Beru.toml`, `extract_inline_manifest()` / `default_adhoc_manifest()` (`crates/beru-manifest/src/inline.rs`) handle `/// beru` blocks.
2. When `Beru.lock` is missing or stale relative to `manifest.dependencies`, `beru_resolve::resolve_graph()` (`crates/beru-resolve/src/resolve.rs`) runs PubGrub via `BeruProvider<'a>` (`crates/beru-resolve/src/lib.rs`).
3. **Version Constraint Semantics (`version_req_to_range`)**:
   - Bare versions (`"11.0.2"`) default to **exact pin** (`=11.0.2`), NOT Cargo-style caret.
   - Supported forms: `*`, `=X.Y.Z`, `^X.Y.Z` (including `0.x` and `0.0.x`), `~X.Y.Z`, `>=X.Y.Z`, `<X.Y.Z`, and comma-separated intersections (`>=1.0.0, <2.0.0`).
4. Outputs deterministic, name-sorted `BeruLock` (`version = 1`) serialized with `LockedPackage` entries (`name`, `version`, `source`, `checksum`, `dependencies`).

### Stage 2: Recipe Lookup & Source Acquisition (`beru-recipe`)

1. `resolve_recipe()` (`crates/beru-recipe/src/resolve.rs`) searches 5 tiers in strict priority order:
   1. Project-local: `<project_dir>/recipes/<name>/recipe.toml`
   2. Binary-adjacent bundled: `<exe_dir>/recipes/<name>/recipe.toml`
   3. User global: `~/.beru/recipes/<name>/recipe.toml`
   4. Registry index: `~/.beru/index/<name>/<version>/recipe.toml`
   5. Compiled-in embedded fallbacks (`EMBEDDED_RECIPES`: `fmt`, `gtest`, `nlohmann_json`, `spdlog` from `crates/beru-cli/recipes/`)
2. Source acquisition (`crates/beru-recipe/src/fetch.rs`):
   - **Tarballs (`fetch_tarball`)**: Keyed at `~/.beru/cache/sources/<first-16-sha256>/`. Checks cache first, respects `BERU_OFFLINE`, downloads via `reqwest::blocking::Client`, verifies SHA-256 over the full response buffer before touching disk, then extracts (cleaning partial directory on error).
   - **Git (`fetch_git`)**: Clones/fetches into `~/.beru/cache/git/<sanitized-url>/` using shallow `git fetch --depth 1 origin <ref>` + `git checkout --force FETCH_HEAD`.

### Stage 3: ABI Profiling & Global Dependency Cache (`beru-core`, `beru-build`)

1. `toolchain::build_abi_profile()` (`crates/beru-core/src/toolchain.rs`) probes `CXX` or `["c++", "g++", "clang++", "cl", "cl.exe"]` (`--version`), detects stdlib (`libstdc++`, `libc++` on macOS clang, `msvc-stl`), OS/arch, and (for `c++20`/`c++23`/`c++26`) tests `supports_concepts` and `supports_modules`.
2. `AbiProfile::hash()` (`crates/beru-core/src/abi.rs`) emits a 32-char lowercase hex prefix of SHA-256 over compiler, version, stdlib, arch, OS, build type, `cxx_std`, `shared_libs`, and sorted features.
3. `prepare_locked_dependencies()` / `resolve_and_build_locked_dep()` (`crates/beru-build/src/deps.rs`):
   - Install prefix: `~/.beru/cache/builds/<abi-hash>/<pkg>-<version>/`.
   - Checks `cache.has_build()` (`dir.exists() && dir.join("include").exists()`) before resolving/fetching sources or checking vendored sources (`vendor/<name>-<version>`).
   - Builds dependencies via `build_dependency_cmake()` (`crates/beru-build/src/cmake.rs`, using `<src>/_beru_build` and installing to cache prefix) or `build_dependency_custom()` (`crates/beru-build/src/custom.rs`, executing shell commands with `{install_dir}` and `{jobs}`).

### Stage 4: Toolchain Synthesis & Project Build (`beru-build`, `beru-cli`)

1. `inspect_installed_dependency()` + `discover_cmake_info()` (`crates/beru-build/src/toolchain.rs`) inspects each installed prefix (combining optional `recipe.export` overrides with bounded prefix discovery of `*Config.cmake` / `*-config.cmake`, `add_library(... IMPORTED)`, `INTERFACE_INCLUDE_DIRECTORIES`, pkg-config `.pc` `Cflags: -I...`, nested umbrella include dirs like `include/eigen3`, and compiled libs in `lib`/`lib64`).
2. `generate_toolchain_cmake_with_config()` emits `<project_dir>/beru-toolchain.cmake` with:
   - `CMAKE_CXX_STANDARD`, `CMAKE_BUILD_TYPE`, profile warnings/optimization/LTO/sanitizers/flags via MSVC vs non-MSVC generator expressions
   - `CMAKE_PREFIX_PATH` pointing to cached dependency install prefixes
   - `macro(beru_link_dependencies TARGET_NAME)` wiring include dirs, link dirs, `find_package()`, `PkgConfig`, imported CMake targets, and raw `link_libs` fallback
3. `build_project()` configures into `build/` (validating existing `CMakeCache.txt` via `is_cmake_cache_valid()`), compiles with `cmake --build --parallel`, and copies `compile_commands.json` to project root if generated.
4. `beru check` adds syntax-only flags (`-fsyntax-only` + `beru-override.cmake` no-op linker rules) on top of `build_project()`.
5. `beru run` handles both executable project targets and standalone ad-hoc `.cpp` scripts (`build_adhoc()` in `crates/beru-build/src/adhoc.rs` cached under `~/.beru/cache/adhoc/<hash>/`).

---

## 3. CLI Subcommands (`crates/beru-cli/src/commands/mod.rs`)

Registered subcommands:
`new`, `init`, `build`, `run`, `resolve`, `index`, `clean`, `cache`, `add`, `remove` (`rm`), `update`, `outdated`, `tree`, `check`, `test`, `install`, `uninstall`, `recipe`, `fmt`, `search`, `self-update`, `setup`, `vendor`.

---

## 4. Cross-Platform Invariants

- **Windows MSVC vs MinGW/UCRT64 vs Unix:**
  - Always generate CMake flags with `$<$<CXX_COMPILER_ID:MSVC>:...>` generator expressions rather than assuming GCC/Clang CLI flags.
  - Normalize `\` to `/` and strip `\\?\` from canonicalized paths (`normalize_cmake_path`, `cmake_paths_match`) before emitting `.cmake` scripts or comparing with `CMakeCache.txt`.
  - Recognize `.lib`, `.dll.a`, `.a`, `.so`, `.dylib` library extensions (`extract_link_lib_name`).
  - Keep `source_dir` paths compact (`sha256[..16]`) to avoid Windows `MAX_PATH` failures during nested CMake extraction/builds.

