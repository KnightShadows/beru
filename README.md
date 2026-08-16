<div align="center">
  

  <h1>📦 Beru</h1>
  <p><strong>A modern, declarative, and lightning-fast C++ package manager and build orchestrator.</strong></p>

  <a href="https://github.com/KnightShadows/Beru/actions"><img src="https://github.com/KnightShadows/Beru/workflows/CI/badge.svg" alt="Build Status"></a>
  <a href="https://crates.io/crates/beru"><img src="https://img.shields.io/crates/v/beru.svg" alt="Crates.io"></a>
  <a href="https://github.com/KnightShadows/beru_index"><img src="https://img.shields.io/badge/Package%20Index-beru__index-orange.svg" alt="Beru Index"></a>
  <a href="https://github.com/KnightShadows/Beru/blob/main/LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg" alt="License"></a>
  
  <br/>
  <br/>
</div>

## The C++ Package Manager You've Always Wanted

C++ developers have suffered through sprawling `CMakeLists.txt` files, global environment variables, ABI mismatches, and cryptic linking errors for decades. **Beru changes everything.**

Written entirely in Rust, Beru brings the developer experience of Cargo and npm to the C++ ecosystem. It acts as an intelligent orchestrator, abstracting away the pain of CMake generation and dependency resolution so you can focus entirely on writing highly optimized native code.

## ✨ Core Features

* 📦 **Zero-Configuration Manifests**: Ditch CMake for your project definitions. Use a strongly-typed, human-readable `Beru.toml` file to declare your dependencies, C++ standard, and build profile.
* 🏃 **Ad-Hoc Execution**: Need to run a quick script or competitive programming file? Run `beru run script.cpp` and Beru will auto-configure CMake and magically link all your dependencies for you!
* 🧠 **Mathematical Dependency Resolution**: Beru integrates the battle-tested **[PubGrub algorithm](https://github.com/pubgrub-rs/pubgrub)**. When your dependencies conflict, Beru doesn't fail with a cryptic linker error; it outputs a step-by-step logical proof explaining exactly *why* the graph is unsolvable.
* ⚡ **Decentralized, Instant Graph Resolution**: Say goodbye to slow API calls. Beru clones the global registry from **[beru_index](https://github.com/KnightShadows/beru_index)** via Git. Resolution happens entirely offline in $O(1)$ time.
* 🛠️ **Enterprise Tooling Built-in**: Fast syntax-only compilations via `beru check`, dependency graph visualization via `beru tree`, and automatic parallel test execution via `beru test`.
* 🛡️ **Cryptographic Binary Caching**: Third-party libraries are compiled exactly once. Their resulting `.a`/`.lib` artifacts and headers are globally cached under strict cryptographic hashes of your compiler version and requested C++ standard, completely eliminating ABI poisoning.
* 🔌 **Seamless CMake Integration**: While Beru handles the orchestration, it generates standard `cmake` toolchain files under the hood. It integrates perfectly with your favorite IDEs (CLion, VSCode, Visual Studio).

---

## 📚 Official Package Index

Explore available C++ libraries, browse version recipes, or contribute new packages at the official Beru Registry:

👉 **[github.com/KnightShadows/beru_index](https://github.com/KnightShadows/beru_index)**

Sync and update your local recipe index at any time:
```bash
beru index update
```

---

## 🚀 Quick Start

### 1. Installation

Beru is distributed as a single, statically linked binary.

**Linux & macOS:**
```bash
curl -sSL https://raw.githubusercontent.com/KnightShadows/Beru/main/install.sh | bash
```

**Windows (PowerShell):**
```powershell
Invoke-WebRequest -Uri https://raw.githubusercontent.com/KnightShadows/Beru/main/install.ps1 -OutFile install.ps1; .\install.ps1
```

*(Alternatively, install directly via Cargo: `cargo install beru`)*

### 2. Scaffold a New Project

```bash
# Create a new C++20 executable project
beru new my_engine --type executable --cxx-std c++20
cd my_engine
```

### 3. Add Dependencies

You can dynamically inject dependencies into your project without touching the manifest:

```bash
beru add fmt --version 11.0.2
beru add spdlog --version 1.14.1
```

### 4. Build and Run

```bash
beru run
```
*Beru will instantly resolve the PubGrub graph, download the release tarballs, verify their SHA-256 signatures, compile them using your system's compiler, cache the binaries, synthesize a CMake toolchain, compile your `my_engine` source code, and run the resulting executable.*

---

## 📖 The Beru Book (Documentation)

We believe world-class software requires world-class documentation. 

Everything you need to master Beru—from migrating legacy Conan/vcpkg projects, to authoring new recipes for the global index, to understanding the internal Rust architecture—is covered extensively in the **[Beru Documentation](docs/Home.md)**.

*   [**Getting Started Guide**](docs/Getting-Started.md)
*   [**Manifest (`Beru.toml`) Reference**](docs/Reference-Manifest-BeruToml.md)
*   [**Command Line Interface Reference**](docs/Reference-CLI.md)
*   [**Authoring Recipes for Beru Index**](docs/Guides-Authoring-Recipes.md)
*   [**Official Package Index Repository**](https://github.com/KnightShadows/beru_index)
*   [**Architecture & PubGrub Concepts**](docs/Architecture.md)

---

## 🤝 Contributing

Beru is an open-source project driven by the C++ and Rust communities. Whether you want to package a new C++ library for the global index (**[beru_index](https://github.com/KnightShadows/beru_index)**) or hack on the core Rust orchestrator, we welcome your PRs!

Please read our comprehensive **[Contributing Guide](docs/Contributing.md)** before submitting code.

## 📄 License

Beru is dual-licensed under either the **[MIT License](LICENSE-MIT)** or the **[Apache License, Version 2.0](LICENSE-APACHE)**, at your option.
