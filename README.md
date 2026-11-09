<div align="center">

<pre align="center">
██████╗ ███████╗██████╗ ██╗   ██╗
██╔══██╗██╔════╝██╔══██╗██║   ██║
██████╔╝█████╗  ██████╔╝██║   ██║
██╔══██╗██╔══╝  ██╔══██╗██║   ██║
██████╔╝███████╗██║  ██║╚██████╔╝
╚═════╝ ╚══════╝╚═╝  ╚═╝ ╚═════╝ 
</pre>

<h3><strong>Next-Generation Declarative C++ Package Manager & Build Orchestrator</strong></h3>

[![CI](https://github.com/KnightShadows/Beru/workflows/CI/badge.svg)](https://github.com/KnightShadows/Beru/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust 1.85+](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Package Index](https://img.shields.io/badge/Registry-beru__index-orange.svg)](https://github.com/KnightShadows/beru_index)
[![Platforms: Linux | macOS | Windows](https://img.shields.io/badge/Platforms-Linux%20%7C%20macOS%20%7C%20Windows-purple.svg)](#-installation)

<br/>

**Beru** combines the speed and ergonomic precision of modern systems tooling (`cargo`, `uv`, `bun`) with native C++ compilation. It replaces convoluted `CMakeLists.txt` scripts with declarative `Beru.toml` manifests, resolves graphs mathematically with the **PubGrub** SAT algorithm, eliminates ABI poisoning via cryptographic hashing, and synthesizes standalone CMake toolchains and presets out of the box.

<br/>

</div>

---

## ⚡ Why Beru?

For decades, C++ developers have struggled through sprawling `CMakeLists.txt` scripts, fragmented package managers, elusive ABI incompatibilities, and cryptic linker errors. **Beru solves these challenges:**

* **Declarative Manifests**: Declare your project name, dependencies, C++ standard, and compiler flags in a readable `Beru.toml`—no bespoke CMake scripting required.
* **Mathematical Conflict Resolution**: Powered by the **PubGrub** SAT algorithm. When dependencies conflict, Beru prints a step-by-step logical proof explaining exactly why requirements cannot be satisfied.
* **Deterministic Builds**: Every resolution is locked to exact versions and commit SHAs in `Beru.lock` for reproducible builds across machines and CI.
* **Cryptographic ABI Isolation**: Dependencies are built once and cached globally under `~/.beru/cache/builds/<abi-hash>/`. Keys are computed from an exhaustive `AbiProfile` (compiler, version, stdlib, architecture, OS, build type, C++ standard, and features), preventing ABI poisoning.
* **Zero-Friction Toolchain Setup**: `beru setup` inspects your host machine for `git`, `cmake`, C++ compilers (`clang++`, `g++`, MSVC `cl.exe`), and `ninja`, offering automatic remediation and PATH configuration.
* **Seamless IDE Integration**: Generates standard `beru-toolchain.cmake` and `CMakePresets.json`, giving CLion, VS Code, and Visual Studio instant autocompletion and debugging.

---

## 🚀 Installation

Beru is distributed as a single standalone binary (`musl` on Linux, native on macOS and Windows).

### Linux & macOS
```bash
curl -sSL https://raw.githubusercontent.com/KnightShadows/Beru/main/install.sh | bash
```

### Windows (PowerShell)
```powershell
Invoke-WebRequest -Uri https://raw.githubusercontent.com/KnightShadows/Beru/main/install.ps1 -OutFile install.ps1; .\install.ps1
```

*(Or build from source: `cargo install --git https://github.com/KnightShadows/beru.git`)*

### Verify Prerequisites with `beru setup`
Run `beru setup` to inspect and configure your toolchain:
```bash
beru setup
```

Output:
```text
=== Beru Environment & Toolchain Setup ===

  git             [OK]     git version 2.43.0
  cmake           [OK]     cmake version 3.28.3 (>= 3.20 supported)
  C++ compiler    [OK]     clang version 18.1.3
  ninja           [OK]     ninja 1.11.1
  PATH            [OK]     ~/.beru/bin is in PATH

Success: All prerequisites are met! You are ready to build with Beru.
```

If any tool is missing, run `beru setup --install-prereqs` for automated package manager remediation (`winget`, `brew`, `apt-get`, `dnf`, `pacman`), or `beru setup --add-to-path` to update your shell environment.

### In-Place Updates (`beru self-update`)
To upgrade Beru to the latest release at any time:
```bash
beru self-update
```
*(Supports `--check` for dry-run checks and `--version <ver>` to pin releases).*

---

## 🏃 Quick Start (30 Seconds)

### 1. Scaffold a New Project
```bash
beru new my-engine --type executable --cxx-std c++20
cd my-engine
```

This generates a pristine project directory:
```text
my-engine/
├── Beru.toml          # Declarative project manifest
├── CMakeLists.txt     # Portable CMake entrypoint (with standalone presets)
├── CMakePresets.json  # IDE configuration for CLion, VS Code, Visual Studio
└── src/
    └── main.cpp       # Application entrypoint
```

### 2. Add Dependencies
Inject dependencies into `Beru.toml` without manual editing:
```bash
beru add fmt --version 11.0.2
beru add spdlog --version 1.14.1
```

### 3. Build & Run
```bash
beru run
```
Beru resolves the dependency graph, fetches source packages, compiles and caches artifacts under your active ABI profile, links your executable, and runs it!

---

## ✨ Key Features & Workflows

### 1. Ad-Hoc Scripts (Zero-Context Execution)
Need to test a code snippet, run an algorithm, or execute a single-file utility? Run C++ files directly:
```bash
beru run script.cpp
```

Declare dependencies inside an inline `// /// beru` metadata block:
```cpp
// /// beru
// [dependencies]
// fmt = "11.0.2"
// ///

#include <fmt/core.h>

int main() {
    fmt::print("Hello from zero-ceremony Beru!\n");
    return 0;
}
```

### 2. Complete Dependency Lifecycle Management
Manage your project's dependencies with ergonomic, Cargo-like subcommands:
* **`beru add <pkg>`**: Add and resolve a new dependency.
* **`beru remove <pkg>`** (alias `rm`): Cleanly drop a dependency and refresh `Beru.lock`.
* **`beru update [pkg]`**: Upgrade locked dependencies to their latest semver-compatible versions.
* **`beru outdated`**: Compare current lockfile versions against latest registry releases:
  ```text
  Package    Current    Latest     Type      Status
  --------------------------------------------------------------------------
  fmt        11.0.0     11.0.2     registry  Update available (semver compatible)
  spdlog     1.14.0     1.14.1     registry  Update available (minor)
  gtest      1.14.0     1.14.0     embedded  Up to date
  ```

### 3. Monorepo Workspaces (`[workspace]`)
Manage multi-library, multi-executable repositories with a single root `Beru.lock`:
```toml
# Root Beru.toml
[workspace]
members = [
    "libs/math",
    "libs/physics",
    "apps/game"
]
```
* Single unified resolution prevents diamond dependency divergence.
* Run builds, tests, or cleans across all members or isolated via `--package <member>`.

### 4. Hermetic & Air-Gapped Builds (`--offline`)
Build with complete network isolation in air-gapped CI environments:
```bash
beru build --offline
beru test --offline
```
Fails fast with actionable messages if any required artifact is not locally cached.

### 5. Declarative Compiler Profiles & System Dependencies
Configure compiler flags and system libraries cleanly in `Beru.toml`:
```toml
[profile.dev]
warnings = "strict"                   # -Wall -Wextra (GCC/Clang) or /W4 (MSVC)
sanitizers = ["address", "undefined"] # Clang/GCC AddressSanitizer
debug = true

[profile.release]
opt-level = 3
lto = true

[system-dependencies]
threads = { cmake = "Threads" }
zlib = { pkg-config = "zlib", fallback = "1.3.1" }
```

### 6. Global Tool Installation (`beru install`)
Compile and install executable C++ CLI utilities into `~/.beru/bin`:
```bash
beru install --path ./tools/asset-baker
beru uninstall asset-baker
```

---

## 🧰 Command Cheatsheet

| Category | Command | Description |
| :--- | :--- | :--- |
| **Scaffolding** | `beru new <name>` | Create a new executable or library C++ project |
| | `beru init` | Initialize a `Beru.toml` manifest in an existing directory |
| **Dependencies** | `beru add <pkg>` | Add a dependency to `Beru.toml` |
| | `beru remove <pkg>` | Remove a dependency (alias: `beru rm`) |
| | `beru update [pkg]` | Update dependencies in `Beru.lock` to latest semver matches |
| | `beru outdated` | Display outdated packages compared to the registry |
| | `beru resolve` | Compute deterministic dependency graph and emit `Beru.lock` |
| | `beru tree` | Print a visual DAG tree of project dependencies |
| **Build & Run** | `beru build` | Compile the project and all dependencies |
| | `beru check` | Rapid syntax-only compilation check bypassing the linker |
| | `beru run [script]` | Build and execute the target or ad-hoc script |
| | `beru test` | Build and execute tests in parallel with CTest |
| | `beru clean` | Remove build artifacts and generated CMake files |
| **Tooling & Env** | `beru setup` | Inspect and configure toolchain prerequisites (`git`, `cmake`, compiler, `ninja`) |
| | `beru self-update` | Download and update to the latest Beru release binary |
| | `beru install` | Install an executable binary globally to `~/.beru/bin` |
| | `beru uninstall` | Remove an installed executable from `~/.beru/bin` |
| | `beru cache` | Manage global binary cache (`clean`, `size`) |
| | `beru recipe` | Author (`recipe init`) and validate (`recipe lint`) recipes |
| | `beru index update` | Synchronize local clone of the package registry |

---

## 📊 Feature Comparison

| Capability | CMake (FetchContent) | Conan 2.0 | vcpkg | **Beru** |
| :--- | :---: | :---: | :---: | :---: |
| **Declarative TOML Manifest** | ❌ | ❌ (Python / INI) | ❌ (JSON) | **✅ Yes (`Beru.toml`)** |
| **Deterministic Lockfile** | ❌ | ⚠️ Optional | ⚠️ Git Submodule | **✅ Yes (`Beru.lock`)** |
| **SAT Dependency Solver** | ❌ | ⚠️ Custom Heuristic | ⚠️ Heuristic | **✅ PubGrub (Mathematical)** |
| **Conflict Explanations** | ❌ (Silent failure) | ⚠️ Generic Error | ⚠️ Truncated | **✅ Step-by-Step Proof** |
| **Single-File Ad-Hoc Scripts**| ❌ | ❌ | ❌ | **✅ Yes (`beru run`)** |
| **Cryptographic ABI Isolation**| ❌ | ⚠️ Manual Profiles | ⚠️ Triplet Matching | **✅ Automated `AbiProfile`** |
| **Offline-First Execution** | ❌ | ⚠️ Complex Cache | ⚠️ Cache Directory | **✅ Native `--offline`** |
| **Zero-Config Setup Assistant**| ❌ | ❌ | ❌ | **✅ `beru setup`** |
| **In-Place Self Updates** | ❌ | ⚠️ via pip | ⚠️ via git pull | **✅ `beru self-update`** |

---

## 📖 The Beru Book (Documentation)

Comprehensive documentation is available in the **[docs/](docs/Home.md)** directory:

* [**Getting Started Guide**](docs/Getting-Started.md) — Step-by-step introduction to Beru workflows.
* [**IDE Integration Guide**](docs/Guide-IDE-Integration.md) — Configuring CLion, VS Code, and Visual Studio.
* [**Manifest Reference (`Beru.toml`)**](docs/Reference-Manifest-BeruToml.md) — Full specification for project manifests.
* [**CLI Reference**](docs/Reference-CLI.md) — Detailed reference documentation for every subcommand.
* [**Toolchain Setup Reference**](docs/Reference-CLI-setup.md) — Prerequisite detection and automation.
* [**Self-Update Reference**](docs/Reference-CLI-self-update.md) — In-place binary update specification.
* [**Recipe Authoring Guide**](docs/Guide-Writing-A-Recipe.md) — Packaging third-party C++ libraries.
* [**Official Registry Index**](https://github.com/KnightShadows/beru_index) — Browse published packages and recipes.
* [**Architecture & PubGrub Solver**](docs/Architecture.md) — Internal design, SAT solver, and ABI hashing.

---

## 🤝 Contributing

Beru is an open-source project welcoming contributions from the C++ and Rust communities. Whether you are packaging a new library for the registry (**[beru_index](https://github.com/KnightShadows/beru_index)**) or enhancing the core Rust engine, we welcome your pull requests!

Please review our [**Contributing Guide**](docs/Contributing.md) before submitting code.

---

## 📄 License

Beru is dual-licensed under either:

* **[MIT License](LICENSE-MIT)**
* **[Apache License, Version 2.0](LICENSE-APACHE)**

at your option.
