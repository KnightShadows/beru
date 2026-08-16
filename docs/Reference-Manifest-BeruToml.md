# Reference: The Beru.toml Manifest

The `Beru.toml` file is the definitive source of truth for any Beru project. Whether you are building a lightweight header-only utility or a sprawling suite of microservices, this single file dictates the project's identity, its compiler requirements, its exact dependency graph, and its build profile configurations.

Drawing heavy inspiration from Rust's `Cargo.toml`, the manifest uses the TOML format for its human-readable syntax and strong typing.

This chapter provides an exhaustive technical reference for every field parsed by the Beru orchestrator.

---

## 1. The `[package]` Section

The `[package]` table is mandatory. It defines the core identity of the artifact being produced.

| Field | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `name` | String | **Yes** | - | The name of the package. This is used as the final output binary name, the CMake project name, and the internal cache key. It must consist only of lowercase alphanumeric characters and hyphens, and must start with a letter. |
| `version` | String | **Yes** | - | A strict Semantic Versioning (SemVer) string (e.g., `1.2.3`). Pre-release identifiers (e.g., `1.2.3-alpha.1`) are permitted. |
| `type` | Enum | No | `"library"` | Dictates the final build artifact. Valid options are `"executable"`, `"library"`, or `"header-only"`. |
| `cxx-std` | Enum | No | `"c++17"` | The exact C++ standard required to compile this code. Valid options are `"c++11"`, `"c++14"`, `"c++17"`, `"c++20"`, `"c++23"`, `"c++26"`. Beru aggressively enforces this constraint down the dependency tree. |
| `description` | String | No | - | A concise, one-line summary of the package. |
| `license` | String | No | - | An SPDX license identifier (e.g., `"MIT OR Apache-2.0"`). |
| `authors` | Array of Strings | No | `[]` | A list of authors and their contact information. |
| `repository` | String | No | - | A URL pointing to the source code repository. |

### 1.1. Deep Dive: Package `type`

The `type` field fundamentally alters how Beru orchestrates the underlying CMake generation.
*   **`executable`**: Beru will synthesize a `CMakeLists.txt` containing an `add_executable()` directive targeting all `.cpp` files in the `src/` directory. It cannot be depended upon by other Beru projects.
*   **`library`**: The default. Beru generates an `add_library()` directive. Crucially, it sets up CMake interface rules so that the `include/` directory is automatically exposed to any project that links against it.
*   **`header-only`**: Beru generates an `add_library(... INTERFACE)` directive. It skips the expensive source compilation phase entirely, serving only to propagate the `include/` paths down the dependency graph.

---

## 2. Dependency Management Sections

Beru processes three distinct dependency tables. All tables use identical syntax to declare requirements.

### 2.1. `[dependencies]`
The primary dependency table. Libraries listed here are required to compile and run your code. If your project is a `library` and another project depends on you, these dependencies are transitively compiled and linked into their project.

**Version Constraints (Registry Dependencies):**
When declaring a registry dependency, Beru defaults to **exact pins**. Supplying `"11.0.2"` guarantees exactly version `11.0.2`. To allow flexible resolution, you must explicitly opt-in using SemVer prefixes:
*   `=1.2.3` or `1.2.3`: Exact version pin.
*   `^1.2.3`: Caret requirement. Allows updates that do not modify the left-most non-zero digit (e.g., `>=1.2.3, <2.0.0`).
*   `~1.2.3`: Tilde requirement. Allows patch-level updates only (e.g., `>=1.2.3, <1.3.0`).
*   `>=1.2.3`, `<2.0.0`: Inequality bounds.
*   `*`: Wildcard requirement. Allows absolutely any version (the resolver will select the highest available).

```toml
[dependencies]
fmt = "11.0.2"                                   # Exact pin
spdlog = "^1.14.0"                               # SemVer caret range
json = { git = "https://github.com/nlohmann/json.git", tag = "v3.11.3" } # A Git dependency
my-local-math = { path = "../my-local-math" }    # A Path dependency
```

### 2.2. `[dev-dependencies]`
Libraries listed here are **only** compiled when building or testing the current package directly. They are explicitly excluded from the transitive dependency graph. 
*   **Best Practice:** Always place testing frameworks (GoogleTest, Catch2, doctest) or benchmark suites (Google Benchmark) in `[dev-dependencies]`. Failing to do so forces downstream users to compile your heavy test frameworks unnecessarily.

```toml
[dev-dependencies]
gtest = { git = "https://github.com/google/googletest.git", tag = "v1.15.0" }
```

---

## 3. The `[build]` Section

While Beru defaults to sensible build orchestration strategies, the `[build]` table allows you to tweak the underlying behavior of the build engine.

| Field | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `system` | Enum | No | `"cmake"` | The backend build system to invoke. Valid options are `"cmake"` (default) and `"custom"`. With `"custom"`, build commands are specified in a `commands` array and executed via shell. Template variables `{install_dir}` and `{jobs}` are expanded automatically. Recipes using `system = "custom"` must declare an `[export]` section. |
| `cmake-minimum` | String | No | - | Specifies the lowest version of CMake required to orchestrate this package. If the user's system CMake is older, Beru will abort the build with a descriptive error before touching the filesystem. |
| `shared-libs` | Boolean | No | `false` | Instructs the orchestrator to attempt building `.so` (Linux), `.dylib` (macOS), or `.dll` (Windows) shared objects instead of static archives. |

### 3.1. Deep Dive: `shared-libs` vs Static Linking

By default, Beru compiles all dependencies as static archives (`.a` or `.lib`). This aligns with the modern C++ philosophy of producing a single, monolithic, easily deployable executable without runtime dependency headaches (the dreaded "DLL hell").

If you set `shared-libs = true`, Beru injects `BUILD_SHARED_LIBS=ON` into the CMake configuration phase for all dependencies. 
*   **Warning:** Dynamically linking C++ libraries can be fraught with peril. You must ensure that the runtime environment where your application executes has the correct `.so`/`.dll` files in its RPATH or `LD_LIBRARY_PATH`. Beru does not currently bundle shared objects for deployment.

---

## 4. The `[profile.*]` Sections

Build profiles allow you to override compiler flags depending on the intent of the build (e.g., a fast debug build for local development vs. a highly optimized release build for production).

While Beru handles standard CMake build types automatically, you can declare explicit overrides.

| Field | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `optimization` | String | *Compiler Default* | The optimization level: `"0"`, `"1"`, `"2"`, `"3"`, `"s"` (size), `"z"` (extreme size). |
| `lto` | Boolean | `false` | Enables Link-Time Optimization (LTO). This dramatically increases link times but allows the compiler to inline functions across translation units, yielding significant performance gains. |
| `sanitizers` | Array of Strings | `[]` | Injects Clang/GCC sanitizer flags (e.g., `-fsanitize=address`). |

### 4.1. Example Configuration

```toml
[profile.debug]
optimization = "0"
sanitizers = ["address", "undefined"]

[profile.release]
optimization = "3"
lto = true
```

When invoking a release build, Beru will read the `[profile.release]` table and translate `optimization = "3"` into the appropriate compiler-specific flag (`-O3` for GCC/Clang, or `/O2` for MSVC), while simultaneously instructing the linker to perform inter-procedural optimization (`lto = true`).
