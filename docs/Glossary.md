# Glossary of Terms

The C++ packaging ecosystem is fraught with overloaded terminology (e.g., "target", "toolchain", "cache" can mean wildly different things depending on whether you are talking to CMake, Git, or a compiler).

To ensure clarity, this manual uses the following terms with strict, specific definitions within the context of the Beru orchestrator.

---

### A

*   **ABI (Application Binary Interface):** The low-level interface between compiled software components. If two libraries are compiled with different C++ standards or compilers, their ABIs may mismatch, resulting in catastrophic memory corruption or linking failures. Beru's cache hashing strategy is explicitly designed to prevent ABI mismatches.
*   **Artifact:** The final file produced by a build process. This can be an executable binary (e.g., `my_app.exe`), a static archive (e.g., `libfmt.a`), or a shared object (e.g., `libspdlog.so`).

### B

*   **Beru.toml:** The declarative project manifest. The absolute source of truth for a local project's metadata, compiler requirements, and dependency constraints.
*   **Beru.lock:** The auto-generated lockfile. It contains the mathematically resolved, deterministic, flattened graph of exact package versions that the project will compile against.

### C

*   **Cache (Global):** The directory (`~/.beru/cache/`) where Beru stores the final compiled artifacts of third-party dependencies. It is partitioned by strict cryptographic hashes of compiler and environment metadata to prevent poisoning.
*   **Constraint:** A rule defined in `Beru.toml` specifying acceptable versions for a dependency (e.g., `fmt = "10.0.0"`).

### D

*   **Dependency Graph:** The complex tree of libraries required to build the root project. Beru uses the PubGrub algorithm to mathematically flatten this tree into a single list of non-conflicting versions.

### I

*   **Index:** The decentralized, global registry of Beru packages. Technically, it is a Git repository containing 701 `recipe.toml` files across 223 packages, cloned locally to `~/.beru/index/`.

### O

*   **Orchestrator:** A tool that manages the invocation of other tools. Beru is an orchestrator because it does not compile C++ code itself; it calculates the dependency graph, synthesizes the toolchains, and intelligently invokes CMake to perform the actual compilation.

### P

*   **Prefix (Install Prefix):** A specific, isolated directory where a library installs its headers and compiled binaries after the build phase. Beru installs dependencies into isolated prefix directories within the global cache.
*   **PubGrub:** The state-of-the-art version solving algorithm used by Beru (and Cargo/Dart) to perform fast, conflict-driven dependency resolution.

### R

*   **Recipe (`recipe.toml`):** A blueprint residing in the Beru Index. It instructs Beru on where to fetch a specific version of a third-party library, how to verify its cryptographic checksum, and how to compile it.

### S

*   **Synthesis (Toolchain Synthesis):** The process where Beru dynamically generates a `beru-toolchain.cmake` file in the local project directory. This file translates Beru's cached artifacts into native CMake target definitions, bridging the gap between the package manager and the build system.

### T

*   **Target (CMake Target):** An abstract concept in CMake representing an executable or a library (e.g., `fmt::fmt`). Targets contain properties like include directories and linking requirements. Beru synthesizes targets for your dependencies so your code can link against them seamlessly.
*   **Toolchain File:** A CMake script invoked at the very beginning of the configuration phase, traditionally used for cross-compilation. Beru hijacks this mechanism to inject dependency paths into the build environment.
