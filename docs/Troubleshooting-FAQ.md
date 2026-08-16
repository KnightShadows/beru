# Troubleshooting and FAQ

Even with robust orchestration, compiling native C++ code across different platforms, compilers, and operating systems is an inherently fragile process.

This chapter details the most common errors encountered when using Beru, their root causes, and explicit instructions on how to resolve them.

---

## 1. Resolution and Locking Errors

### 1.1. "Failed to resolve dependency graph"
**Symptoms:** When executing `beru build` or `beru resolve`, the CLI halts immediately and prints a logical trace of constraints provided by the PubGrub algorithm.

**Root Cause:** You have requested a dependency graph that violates the One Definition Rule. For example, your `Beru.toml` strictly demands `fmt = 9.0.0`, but you also depend on a graphics library that strictly demands `fmt = 11.0.0`. It is mathematically impossible to link both into the same executable.

**Resolution:** 
Read the error output carefully. PubGrub prints a step-by-step logical proof of the conflict. You must trace the conflict up to your direct dependencies. To fix it, edit your `Beru.toml` and either upgrade or downgrade your direct dependency to a version that is compatible with the transitive constraints of the rest of your graph.

### 1.2. "Package not found in index"
**Symptoms:** You add a newly released library to your `Beru.toml`, but Beru claims the package or version does not exist.

**Root Cause:** Beru resolves dependencies entirely offline using the local cache in `~/.beru/index/`. If a recipe was published to the global GitHub repository after your last sync, your local index is stale.

**Resolution:** 
Run `beru index update` to perform a fast-forward Git pull on your local index, then retry your build.

---

## 2. Compilation and Build Errors

### 2.1. "CMake not found" or "Execution failed"
**Symptoms:** Beru halts immediately before the cache compilation stage, stating that it failed to invoke CMake.

**Root Cause:** Beru is an orchestrator, not a compiler. It relies on the system's `cmake` binary being available on your global `PATH`.

**Resolution:** 
Install CMake >= 3.20.
*   **Ubuntu/Debian:** `sudo apt install cmake`
*   **macOS:** `brew install cmake`
*   **Windows:** Use the official installer from cmake.org, or `winget install cmake`. Ensure you check the box to add CMake to the system PATH during installation.

### 2.2. Linker Errors (Undefined Reference to...)
**Symptoms:** The build proceeds normally, dependencies compile, but the final linking stage fails with dozens of "undefined reference" or "unresolved external symbol" errors pointing to a third-party library.

**Root Cause:** This usually indicates a broken `recipe.toml` in the global index. Specifically, the `cmake_targets` array in the `[export]` section of the recipe is missing targets, or the library author failed to properly export their CMake interface. Beru successfully compiled the `.a`/`.lib` file, but your local CMake project wasn't instructed on how to link it.

**Resolution:** 
First, ensure you are actually calling `target_link_libraries(my_project PRIVATE target::name)` in your local `CMakeLists.txt` (if you are not relying on Beru's auto-generated CMake). 
If you are, the upstream recipe is likely flawed. You will need to inspect the library's `CMakeLists.txt` to find the correct target name, patch the `recipe.toml` locally in `~/.beru/index/`, and submit a Pull Request to the `beru_index` repository.

---

## 3. Cache Corruption and Strange Behavior

### 3.1. Weird ABI Errors after upgrading your OS/Compiler
**Symptoms:** A project that compiled perfectly yesterday now fails with bizarre errors inside standard library headers (`<string>`, `<vector>`), or crashes with a segfault immediately upon startup.

**Root Cause:** You likely updated your system C++ compiler (e.g., upgrading from Ubuntu 22.04 to 24.04, which bumps GCC from v11 to v13). While Beru hashes the compiler version to prevent this exact issue, highly esoteric environment variables (`CXXFLAGS`) injected by your shell or OS can sometimes poison the binary cache, tricking Beru into linking an old GCC 11 binary against your new GCC 13 application.

**Resolution:** 
The Beru cache is completely safe to destroy. It is just generated binaries. 
1. Delete the local project build artifacts: `beru clean`
2. Wipe the global binary cache: `beru cache clean` (or manually remove `~/.beru/cache/`).
3. Run `beru build`. Beru will re-download the tarballs and recompile everything from scratch using your new, pristine compiler environment.
