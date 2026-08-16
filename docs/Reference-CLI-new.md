# Command Reference: `beru new`

The `beru new` command is the standard entry point for starting a fresh C++ project. It eliminates the tedious boilerplate of setting up directory structures, writing initial CMake configurations, and creating `.gitignore` files.

---

## 1. Usage Synopsis

```bash
beru new [OPTIONS] <NAME>
```

## 2. Detailed Description

When invoked, `beru new` performs the following operations:

1.  **Directory Creation:** It creates a new directory matching the `<NAME>` argument. If a directory with that name already exists, the command will abort with a fatal error to prevent overwriting existing files. (If you wish to initialize an existing directory, use [`beru init`](Reference-CLI-init.md) instead).
2.  **Manifest Generation:** It generates a `Beru.toml` manifest at the root of the new directory, pre-populated with the project name, a default version of `0.1.0`, and the specified `type` and `cxx-std`.
3.  **Source Scaffolding:** It creates a `src/` directory containing a minimal, compilable source file (e.g., `main.cpp` for executables, or `<name>.cpp` for libraries).
4.  **Header Scaffolding:** If the project type is `library` or `header-only`, it creates an `include/<NAME>/` directory containing a minimal public header file, establishing best practices for namespace isolation immediately.
5.  **Test Scaffolding:** It creates a `tests/` directory with a minimal `test_main.cpp` file.
6.  **CMake Generation:** It generates a foundational `CMakeLists.txt` file tailored to the requested project type. While Beru orchestrates dependencies automatically, this file allows the user to write custom compilation logic for their own source files if needed.
7.  **Version Control Setup:** It drops a `.gitignore` file configured to ignore Beru's hidden orchestration directory (`.beru/`), standard build directories (`build/`, `target/`), and common IDE cache files.

---

## 3. Options and Flags

### `[NAME]` (Positional Argument)
**Required.** The name of the project. This will be used as the directory name, the package name in `Beru.toml`, the CMake project name, and the output binary name. 

*Validation:* The name must consist only of lowercase alphanumeric characters and hyphens, must start with a letter, and be at least 2 characters long.

### `--type <TYPE>`
**Default:** `executable`

Dictates the architectural shape of the scaffolded project and the resulting CMake targets.

*   **`executable`**: Scaffolds a standalone application. Generates a `src/main.cpp` with a standard `int main()` entry point.
*   **`library`**: Scaffolds a compiled static/shared library. Generates a `src/<name>.cpp` implementation file and an `include/<name>/<name>.hpp` public header.
*   **`header-only`**: Scaffolds a library that requires no compilation. Generates only the `include/` directory structure and sets the `type` in the manifest accordingly.

### `--cxx-std <STD>`
**Default:** `c++17`

Specifies the C++ standard the project will enforce. Beru propagates this standard down the dependency graph to ensure ABI compatibility across all compiled libraries.

*Valid options:* `c++11`, `c++14`, `c++17`, `c++20`, `c++23`, `c++26`.

---

## 4. Examples

**Scaffolding a modern C++20 web server application:**
```bash
beru new my_web_server --type executable --cxx-std c++20
```

**Scaffolding a foundational, header-only mathematics library:**
```bash
beru new fast_math_utils --type header-only
```
