# Reference: Command Line Interface (CLI)

The `beru` binary is the entry point for all interactions with the Beru orchestrator. It is a monolithic, statically linked executable that exposes a suite of subcommands designed to handle every stage of the C++ software development lifecycle, from scaffolding a pristine project to executing a fully resolved and orchestrated build.

This section provides exhaustive reference documentation for every subcommand, flag, and environment configuration supported by the CLI.

---

## 1. Global CLI Behavior

Before diving into specific subcommands, it is important to understand the global behaviors that apply to the CLI as a whole.

### 1.1. Working Directory Sensitivity
With the exception of `beru new` (which creates a directory) and `beru index update` (which operates on the global cache), all Beru commands are heavily dependent on the current working directory. 

Commands like `build`, `run`, and `resolve` must be executed within a directory containing a valid `Beru.toml` manifest file, or in a subdirectory thereof. If executed outside a valid project context, the CLI will abort with a fatal error.

### 1.2. Verbosity and Logging
Beru utilizes a centralized tracing architecture. By default, it prints clean, human-readable status updates to standard error (`stderr`), leaving standard output (`stdout`) pristine for your actual application's output (when using `beru run`).

You can override the default logging level by setting the `BERU_LOG` environment variable before invocation.

```bash
# Print debug information, including raw CMake commands
BERU_LOG=debug beru build

# Print exhaustive traces of the PubGrub resolution logic
BERU_LOG=trace beru resolve
```

### 1.3. Help and Version Discovery
Beru includes built-in documentation for every command, accessible via the `--help` flag.

```bash
beru --version       # Prints the current semantic version of the CLI
beru --help          # Prints the top-level command list
beru build --help    # Prints detailed flag documentation for the 'build' command
```

---

## 2. Command Index

The following table provides a high-level overview of the available subcommands. Click on any command to jump to its exhaustive reference page.

| Command | Lifecycle Phase | Primary Purpose |
| :--- | :--- | :--- |
| **[`beru new`](Reference-CLI-new.md)** | Scaffolding | Creates a new directory and populates it with a standardized project layout. |
| **[`beru init`](Reference-CLI-init.md)** | Scaffolding | Initializes a `Beru.toml` manifest within an existing directory without destroying existing code. |
| **[`beru add`](Reference-CLI-add.md)** | Dependency | Adds a new dependency directly to the `Beru.toml` manifest, dynamically mutating the AST. |
| **[`beru tree`](Reference-CLI-tree.md)** | Analysis | Visualizes the resolved dependency graph using an optimal DAG traversal. |
| **[`beru resolve`](Reference-CLI-resolve.md)** | Resolution | Executes the PubGrub algorithm to compute a deterministic dependency graph and writes it to `Beru.lock`. |
| **[`beru check`](Reference-CLI-check.md)** | Orchestration | Performs a fast, syntax-only compilation check bypassing the linker to ensure rapid feedback. |
| **[`beru build`](Reference-CLI-build.md)** | Orchestration | Resolves, fetches, compiles, and links the entire project and its dependencies. |
| **[`beru test`](Reference-CLI-test.md)** | Execution | Implicitly builds the project and executes the CTest suite in parallel. |
| **[`beru run`](Reference-CLI-run.md)** | Execution | A convenience wrapper that invokes `build` and then immediately spawns the resulting executable binary. |
| **[`beru index update`](Reference-CLI-index-update.md)** | Maintenance | Synchronizes the local clone of the package registry with the upstream Git repository. |
| **[`beru cache`](Reference-CLI-cache.md)** | Maintenance | Manage the global binary cache: view disk usage (`beru cache size`) or clean cached data (`beru cache clean`). |
| **[`beru clean`](Reference-CLI-clean.md)** | Maintenance | Removes the `build/` directory and generated CMake files (`beru-toolchain.cmake`, `beru-override.cmake`), restoring the project to a pristine state. |
