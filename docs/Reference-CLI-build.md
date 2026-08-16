# Command Reference: `beru build`

The `beru build` command is the powerhouse of the Beru ecosystem. It represents the realization of Beru's core philosophy: a single command that transforms a declarative manifest and a directory of source code into a fully linked, highly optimized C++ artifact, completely obscuring the underlying orchestration complexity from the developer.

---

## 1. Usage Synopsis

```bash
beru build
```

## 2. Detailed Description

When you execute `beru build`, you are triggering a sophisticated, multi-stage pipeline. Understanding this pipeline is critical for debugging complex integration issues.

### 2.1. Stage 1: Resolution and Locking
The build command first verifies the integrity of the dependency graph. It implicitly invokes the logic of `beru resolve`. If a `Beru.lock` file exists, it is parsed and verified against the `Beru.toml`. If it is missing or out of date, the PubGrub algorithm computes a new graph and writes it to disk.

### 2.2. Stage 2: The Fetch Phase
Beru iterates through every node in the locked dependency graph. If the source for a package is a Git repository, it clones the specified `rev`, `tag`, or `branch`. If it is a registry package, it downloads the release tarball via HTTPS, verifying the cryptographic `sha256` signature immediately upon completion to prevent supply-chain attacks.

### 2.3. Stage 3: The Cache Compilation Phase
This is the most computationally expensive phase. Beru checks the global binary cache (`~/.beru/cache/`) for every dependency. A cache hit requires an exact match on:
1. The package name and version.
2. The compiler executable path and version hash (e.g., `/usr/bin/g++ v11.4`).
3. The host architecture (e.g., `x86_64-unknown-linux-gnu`).
4. The requested C++ standard (e.g., `c++20`).

If a cache miss occurs, Beru invokes a completely isolated, out-of-source CMake build for that specific library. It compiles the static or shared objects and installs them—alongside their public header files—into a unique, hashed prefix directory within the cache.

### 2.4. Stage 4: Toolchain Orchestration
Once all dependencies are securely cached, Beru generates the `beru-toolchain.cmake` file in your local project root. 

This file is a dynamically synthesized CMake module. It iterates over the exported `include_dirs` and `cmake_targets` of every dependency in your graph, injecting absolute paths pointing into the `~/.beru/cache/` directory.

### 2.5. Stage 5: The Final Build
Finally, Beru invokes the system's CMake executable against your local project source code. It passes the `-DCMAKE_TOOLCHAIN_FILE=beru-toolchain.cmake` flag, ensuring that your local `find_package` calls or `target_link_libraries` directives successfully locate and link the cached artifacts.

The resulting artifact is placed in the standard `build/` output directory.

---

## 3. Options and Flags

### `[TARGET]` (Optional Positional Argument)
If your `src/` directory contains multiple `.cpp` files, Beru defaults to compiling `main.cpp` (if it exists) or the alphabetically first file. You can override this behavior by explicitly providing a target name.

```bash
beru build day1
```

### `--profile <PROFILE>`
**Default:** `debug`

Selects the build profile defined in your `Beru.toml` manifest to use for this compilation. By default, Beru looks for the `[profile.debug]` section. To invoke a highly optimized release build, you must specify the release profile.

```bash
beru build --profile release
```

*Note: Attributes like static vs. shared libraries, sanitizers, and optimization levels are defined inside the `Beru.toml` profile, ensuring the project builds identically across developer machines rather than relying on disparate CLI flags.*

---

## 4. Examples

**Executing a standard build:**
```bash
beru build
```

**Executing a build while observing the underlying CMake orchestration:**
```bash
BERU_LOG=debug beru build
```
