# Changelog

All notable changes to **Beru** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.6.2] - 2026-11-10

### ⚡ Performance & Internal Architecture
* **Toolchain & Compiler Probe Caching**: Caches compiler detection (`(CXX, PATH)`) in memory and persists compiler metadata + C++20/23/26 feature probe results under `~/.beru/cache/toolchain-probe.toml` keyed by compiler binary fingerprint to skip redundant subprocess spawns and test compiles.
* **Cached Tool Lookups**: Shared in-memory PATH resolution cache (`which_cached`) eliminates repeated PATH walks for `cmake`, `sh`, and `ctest`.
* **Parallel Dependency Prep & Single-Pass Inspection**: Overlaps uncached non-vendored source fetches, parallelizes cached dependency prefix introspection, reuses already-loaded `recipe.toml` files, skips redundant prefix scans in `generate_toolchain_cmake_with_config`, and skips reading `*ConfigVersion.cmake` files.
* **HTTP Pooling & Immutable Git SHA Short-Circuit**: Reuses a pooled `reqwest::blocking::Client` with per-destination fetch locking and skips remote `git fetch` / `git ls-remote` when an immutable 40-char commit SHA is already present or pinned.
* **Incremental Write-If-Changed**: Avoids rewriting unchanged `beru-toolchain.cmake`, `beru-override.cmake`, `CMakeLists.txt` (ad-hoc), and `Beru.lock` files so CMake preserves timestamps and skips unnecessary reconfigures.
* **Batched `beru fmt`**: Groups `clang-format --check` and `-i` calls into bounded command-line batches instead of spawning one subprocess per source file, with single-file fallback only on failing batches.
* **Faster `beru search`**: Reads only the `[package]` section via `Recipe::read_package_metadata` and inspects only the latest semver directory per package in the registry index.
* **Job Propagation & Shared Helpers**: Propagated `BERU_JOBS` into `beru test` (`ctest -j`), deduplicated build/check toolchain preparation, and added `.agent/` architecture/performance onboarding docs.

## [0.6.1] - 2026-11-10

### 🐛 Bug Fixes & Improvements
* **Automatic CMake/Include Introspection**: Inspects installed dependency prefixes for CMake package configs (`*Config.cmake` / `*-config.cmake`), imported targets (`*Targets.cmake`), `INTERFACE_INCLUDE_DIRECTORIES`, `pkg-config` `.pc` include flags, and compiled libraries on disk—fixing header-only and nested-include packages (such as `eigen` under `include/eigen3` with `Eigen3::Eigen`) even when recipes omit `[export]` metadata.
* **No Bogus Linker Flags for Header-Only Packages**: Avoids emitting fallback raw `-l<pkg>` library links in `beru-toolchain.cmake` when a dependency is header-only or installs no libraries under `lib/`/`lib64/`.
* **Recipe Schema Compatibility**: Added `args` alias for `cmake-args` in `[build]` (plus snake_case `[export]` aliases) to support community index recipes.
* **CMake Cache & Toolchain Reuse**: Validates `build/CMakeCache.txt` before re-configuring to eliminate spurious `CMAKE_TOOLCHAIN_FILE` unused-CLI warnings and reset stale build caches automatically.

## [0.6.0] - 2026-11-09

### 🚀 Major New Features & Enhancements

#### Toolchain & Environment Management
* **`beru setup`**: Automated C++ prerequisite healthcheck and configuration tool (similar to `rustup`). Probes and reports status for `git`, `cmake` ($\ge$ 3.20), C++ compilers (`clang++`, `g++`, MSVC `cl.exe`), `ninja`, and user `PATH`. Supports `--check` for CI pipelines and `--install-prereqs` for automated system package manager remediation (`winget`, `brew`, `apt-get`, `dnf`, `pacman`).
* **`beru self-update`**: In-place atomic binary updater querying official GitHub Releases. Implements Windows executable lock resolution via `.old` file swapping and cleanup, Unix atomic permission replacement, and mirror overrides (`BERU_SELF_UPDATE_URL`).
* **Windows & Unix Installers**: Introduced `install.ps1` for PowerShell on Windows (installs to `~/.beru/bin`, sets User `PATH`, runs `beru setup`) and updated `install.sh` for Linux/macOS.

#### Dependency Lifecycle & Discovery
* **`beru vendor [PATH]`**: Packages all locked project dependencies locally into `./vendor/<pkg>-<version>/` with self-contained `recipe.toml` definitions, enabling hermetic, air-gapped enterprise builds via `beru build --offline`.
* **`beru fmt`**: Automatically formats all C++ project and workspace source files (`.cpp`, `.hpp`, `.cc`, `.cxx`, `.h`, `.c`) in place or checks formatting (`--check`) using `clang-format`.
* **`beru search <query>`**: Search for packages by name and description across the local registry index (`~/.beru/index/`), bundled recipes, and embedded fallbacks.
* **`beru remove <pkg>` / `beru rm`**: Cleanly drop dependencies from `Beru.toml` using AST-preserving edits, automatically re-syncing `Beru.lock`.
* **`beru update [pkg]`**: Re-resolve and advance locked dependencies to their highest semver-compatible releases.
* **`beru outdated`**: Aligned comparison table highlighting available package updates across registry, embedded, and local recipes.
* **`beru install` & `beru uninstall`**: Compile executable C++ packages under Release profile and globally link them into `~/.beru/bin` for system-wide availability.

#### Build Ergonomics, Interoperability & IDE Support
* **Monorepo Workspaces (`[workspace]`)**: First-class support for multi-library, multi-executable repositories with a single unified `Beru.lock` and topological compilation.
* **LSP Auto-Export**: Automatically exports `compile_commands.json` to the project root on `beru build`/`run` and cleans it on `beru clean` for out-of-the-box `clangd`, Neovim, and VS Code support.
* **Standalone CMake & Presets**: Guarded `beru_link_dependencies` with `if(COMMAND ...)` and generated `CMakePresets.json` with `CMAKE_EXPORT_COMPILE_COMMANDS=ON` for standalone CMake and IDE compatibility.
* **Global `--offline` Flag**: Strict air-gapped build mode disabling network fetches and failing fast with actionable messages if dependencies are un-cached.
* **Declarative Compiler Profiles & System Dependencies**: Support for `[profile.dev]` / `[profile.release]` (`warnings = "strict"`, `sanitizers`, `opt-level`, `lto`) and `[system-dependencies]` with automatic CMake `find_package` / `pkg-config` probing and fallback recipes.
* **Recipe Authoring Tools**: Added `beru recipe init <pkg>` and `beru recipe lint` to streamline authoring packages for the global index.

### 🐛 Bug Fixes & Reliability
* **Manifest Validation**: Aligned package naming validation in `beru new` and `beru init` to reject invalid characters early with actionable suggestions.
* **Embedded Fallback Recipes**: Embedded core recipes (`fmt`, `gtest`, `nlohmann_json`, `spdlog`) into the binary to ensure offline zero-context builds succeed on fresh installs.
* **Detailed Conflict Proofs**: Formatted PubGrub derivation trees to surface step-by-step human-readable proofs when resolution fails, silencing noisy internal tracing filters.
* **Tarball Safety**: Replaced panicking `.expect()` calls with proper error propagation when tarball SHA-256 checksums are missing or archives are corrupt.
* **Archive & Linker Fixes**: Added `.tgz` archive support, automatic corrupted cache eviction, `lib64` library linking, and deduplicated recipe resolutions.
* **Static Musl Binaries**: Added `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` release targets to release workflows for true statically linked single-binary distribution.

---

## [0.5.6] - Prior Baseline
* Initial baseline version.
