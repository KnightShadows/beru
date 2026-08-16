# Reference: The recipe.toml Specification

The `recipe.toml` file is the atomic unit of the Beru package index. Unlike `Beru.toml`, which describes a local project you are actively developing, a `recipe.toml` acts as a historical record and instructional blueprint. It teaches the Beru orchestrator how to fetch, unpack, compile, and expose a specific version of a third-party C++ library.

Because these recipes are distributed globally via the Beru index, they must adhere to a strict structural schema. This chapter details every valid field within a recipe.

---

## 1. The `[package]` Section

This section identifies the artifact. It must perfectly match the directory structure of the index (`<package-name>/<package-version>/recipe.toml`).

| Field | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `name` | String | **Yes** | - | The canonical name of the library. Must consist of lowercase alphanumeric characters and hyphens. |
| `version` | String | **Yes** | - | The exact Semantic Versioning (SemVer) string representing this specific release. |
| `type` | Enum | No | `"library"` | The package type: `"library"` or `"header-only"`. |
| `description` | String | No | - | A concise summary of the package. |
| `license` | String | No | - | An SPDX license identifier (e.g. `"MIT"`, `"Apache-2.0"`). |
| `homepage` | String | No | - | The project's homepage or repository URL. |

**Validation Rule:** The PubGrub version solver relies entirely on this `version` string. If the upstream repository uses non-standard versions (like `v1.2_final`), the recipe author must normalize it to a valid SemVer format (like `1.2.0-final`) in this field.

---

## 2. The `[source]` Section

The source section tells the Beru fetcher where to acquire the raw C++ code. The orchestrator requires exactly one mutually exclusive sourcing strategy: a compressed archive URL, or a Git repository.

### 2.1. Archive Sources (Strongly Recommended)

Using compressed `.tar.gz` archives is the gold standard for package management. Archives are immutable, fast to download, and their integrity can be mathematically verified.

| Field | Type | Description |
| :--- | :--- | :--- |
| `url` | String | The direct HTTP/HTTPS URL to the release tarball. |
| `sha256` | String | A 64-character hexadecimal string representing the SHA-256 cryptographic hash of the downloaded archive. |

**Security Imperative:** The `sha256` field is critical. When Beru downloads the tarball from the `url`, it immediately hashes the file in memory. If the computed hash does not match the string provided in the recipe, Beru immediately aborts the build and deletes the file. This protects the ecosystem against man-in-the-middle attacks, DNS hijacking, or compromised upstream servers replacing release binaries with malicious payloads.

### 2.2. Git Sources

If a library does not publish release archives, you may instruct Beru to perform a Git clone.

| Field | Type | Description |
| :--- | :--- | :--- |
| `git` | String | The URL of the Git repository. |
| `tag` | String | The specific Git tag to checkout after cloning. |

**Immutability Rule:** You should only ever point a recipe to a specific, immutable `tag`. Pointing a recipe in the global index to a moving branch (like `main` or `master`) is strictly prohibited, as it destroys the guarantee of reproducible builds.

---

## 3. The `[build]` Section

This table dictates the compilation backend that Beru will spin up to process the downloaded source code.

| Field | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `system` | Enum | No | `"cmake"` | The build system required to compile the library. Supported values: `"cmake"`, `"custom"`. |
| `cmake-args` | Array of Strings | No | `[]` | Extra arguments passed to CMake configure (e.g. `["-DFMT_TEST=OFF"]`). |
| `commands` | Array of Strings | No | `[]` | Shell commands to execute sequentially when `system = "custom"`. Supports `{install_dir}` and `{jobs}` template placeholders. |

### 3.1. Deep Dive: `system` Mechanics
*   **`cmake`**: Beru will execute a full out-of-source CMake configure and build phase. It generates a temporary `build/` directory, executes `cmake -S . -B build <cmake-args>`, followed by `cmake --build build`, and finally `cmake --install build` targeting an isolated prefix inside the global Beru cache.
*   **`custom`**: Beru executes the shell commands defined in `commands` inside the source directory. Recipes using `system = "custom"` must define an `[export]` section with `link-libs` or `include-dirs`.

---

## 4. The `[export]` Section

The export section bridges the gap between the isolated, compiled library residing in `~/.beru/cache/` and the user's active project. 

Without this section, Beru would successfully compile the library, but downstream projects would fail to link against it because they wouldn't know the include paths or the target names.

| Field | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `include-dirs` | Array of Strings | `["include"]` | The relative paths within the installation prefix that contain the public header (`.h`, `.hpp`) files. |
| `link-libs` | Array of Strings | `[]` | Library names to link against directly (e.g., `["fmt"]`). |
| `cmake-package` | String | None | The CMake package name for `find_package()` calls (e.g., `"fmt"`). |
| `cmake-targets` | Array of Strings | `[]` | The exact, namespaced CMake targets that the library exports (e.g., `["fmt::fmt"]`). |

### 4.1. Correctly Identifying Targets

Finding the correct string for `cmake-targets` and `cmake-package` requires inspecting the library's `CMakeLists.txt` or its generated `*Config.cmake` files.

If the library was built with standard modern CMake practices, it will usually define an alias target. For example, the `spdlog` library defines:
`add_library(spdlog::spdlog ALIAS spdlog)`

In this case, your recipe should define:
```toml
[export]
include-dirs = ["include"]
cmake-package = "spdlog"
cmake-targets = ["spdlog::spdlog"]
```

When Beru synthesizes the local toolchain for the user's project, it will inject CMake code that explicitly calls `find_package(spdlog REQUIRED)` and links against `spdlog::spdlog`, ensuring all transitive linker flags, definitions, and include paths are correctly propagated to the final executable.
