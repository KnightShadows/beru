# Guide: Authoring a Beru Recipe

The Beru ecosystem relies on a decentralized, community-driven index of package recipes. A recipe is a `recipe.toml` file that acts as a translator: it tells Beru how to take a legacy, raw CMake library from the internet and package it into the modern Beru dependency graph.

This guide is an exhaustive walkthrough on how to author a robust, production-quality recipe. We will use the popular `{fmt}` formatting library as our case study.

---

## 1. The Anatomy of a Recipe

A `recipe.toml` is composed of four distinct sections, each responsible for a different phase of the orchestration pipeline:
1.  `[package]`: Identification and versioning.
2.  `[source]`: Where to securely download the code.
3.  `[build]`: How to instruct the compiler.
4.  `[export]`: How to expose the resulting artifacts to downstream consumers.

### 1.1. The Package Section

The `[package]` table is trivial but strict. The `name` must exactly match the directory name in the index, and the `version` must exactly match the subdirectory name.

```toml
[package]
name = "fmt"
version = "11.0.2"
```

*   **Rule:** The version string must be a strictly compliant Semantic Version (SemVer). If the upstream library uses a non-standard version like `11.0.2-release_final`, you must normalize it to `11.0.2` or `11.0.2-release.final` to satisfy the PubGrub algorithm.

### 1.2. The Source Section

Beru must download the source code before it can compile it. You have two options: a Git clone, or a compressed tarball archive. 

**Tarballs are strongly preferred.** They are significantly faster to download, they consume less disk space (no `.git` history), and crucially, they can be cryptographically verified to prevent supply-chain attacks.

```toml
[source]
url = "https://github.com/fmtlib/fmt/archive/refs/tags/11.0.2.tar.gz"
sha256 = "6f4db149c953538ed6168e92a832f913d31fc3877b088b9dd6326e133e9d1e39"
```

*   **Computing the SHA256:** You must provide the exact SHA256 checksum of the archive. Beru will refuse to build the package if the checksum mismatches, protecting users from hijacked DNS or compromised GitHub releases.
    You can compute this locally on Linux or macOS using:
    ```bash
    curl -sL <URL> | sha256sum
    ```

If an upstream provider does not offer release archives, you may fallback to Git:

```toml
[source]
git = "https://github.com/fmtlib/fmt.git"
tag = "11.0.2"
```

### 1.3. The Build Section

This section instructs Beru on the underlying build system used by the library. Beru supports `cmake` (default) and `custom` shell command pipelines.

```toml
[build]
system = "cmake"
cmake-args = ["-DFMT_DOC=OFF", "-DFMT_TEST=OFF", "-DFMT_INSTALL=ON"]
```

When `system = "cmake"` is specified, Beru will execute the equivalent of:
```bash
cmake -S <source_dir> -B <build_dir> -DCMAKE_INSTALL_PREFIX=<cache_dir> <cmake-args>
cmake --build <build_dir>
cmake --install <build_dir>
```

**Header-Only Packages:**
If the library consists purely of `.h` or `.hpp` files and requires no compilation (e.g., `nlohmann_json`), set `type = "header-only"` under `[package]`.

**Custom Build Systems:**
For libraries using custom shell scripts or build tools (like Make or `./configure`), set `system = "custom"` and provide a `commands` array with `{install_dir}` and `{jobs}` placeholders:

```toml
[build]
system = "custom"
commands = [
    "./bootstrap.sh --prefix={install_dir}",
    "./b2 install --prefix={install_dir} -j{jobs}"
]
```

### 1.4. The Export Section (The Critical Step)

This is where most recipes fail. After Beru compiles the library and installs it to the hidden cache directory, it must generate a toolchain file for the downstream user so their project can find the library.

To do this, Beru needs to know what include directories, CMake package names, and CMake targets the library generated.

```toml
[export]
include-dirs = ["include"]
cmake-package = "fmt"
cmake-targets = ["fmt::fmt"]
```

*   **`include-dirs`:** An array of directory paths relative to the installation root where the public header files reside. By standard convention, this is almost always `["include"]`.
*   **`cmake-package`:** The package name passed to `find_package(<name> REQUIRED)`.
*   **`cmake-targets`:** This is the exact string a user would pass to `target_link_libraries()` in raw CMake. You must inspect the upstream library's documentation (or its `*Config.cmake` files) to find the correct exported target name. For `{fmt}`, it is `fmt::fmt`. For `spdlog`, it is `spdlog::spdlog`.

---

## 2. Putting it all together

The final, production-ready recipe for `{fmt}` looks like this:

```toml
# ~/.beru/index/fmt/11.0.2/recipe.toml

[package]
name = "fmt"
version = "11.0.2"

[source]
url = "https://github.com/fmtlib/fmt/archive/refs/tags/11.0.2.tar.gz"
sha256 = "6f4db149c953538ed6168e92a832f913d31fc3877b088b9dd6326e133e9d1e39"

[build]
system = "cmake"

[export]
include_dirs = ["include"]
cmake_targets = ["fmt::fmt"]
```

## 3. Validating the Recipe Locally

Before submitting your recipe to the global index, you must prove that it actually compiles and links correctly on your machine.

1.  Create the appropriate directory in your local index cache:
    ```bash
    mkdir -p ~/.beru/index/fmt/11.0.2
    ```
2.  Copy your new `recipe.toml` into that directory.
3.  Create a completely new, blank Beru project somewhere else on your filesystem:
    ```bash
    beru new recipe_test --type executable
    cd recipe_test
    ```
4.  Add your new package to the `Beru.toml`:
    ```toml
    [dependencies]
    fmt = "11.0.2"
    ```
5.  Write a minimal `src/main.cpp` that actually includes a header from the library and calls a function. This proves that the `include_dirs` and `cmake_targets` are correct.
6.  Run `beru run`. 

If the project compiles, links, and executes successfully, your recipe is solid and ready for the world. Proceed to the [Publishing Guide](Guide-Publishing-To-The-Registry.md).
