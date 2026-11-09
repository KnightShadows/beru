# Command Reference: `beru run`

The `beru run` command is a convenience utility designed to accelerate the edit-compile-test loop for application developers. It executes a full project build and, upon success, immediately spawns the resulting binary in the current terminal.

---

## 1. Usage Synopsis

```bash
beru run
```

## 2. Detailed Description

During active development of an executable, developers traditionally run `cmake --build build` followed immediately by `./build/my_app`. 

`beru run` collapses this workflow.

1.  **Implicit Build:** The command first invokes the exact same orchestration pipeline as `beru build`. It resolves dependencies, fetches tarballs, compiles the cache, synthesizes the toolchain, and invokes CMake.
2.  **Binary Discovery:** Upon a successful exit code from the underlying CMake invocation, Beru intelligently searches the `build/` output directory for the compiled artifact. It parses the `name` field from the `Beru.toml` manifest to locate the correct file (accounting for platform-specific extensions, like `.exe` on Windows).
3.  **Process Spawning:** Finally, Beru spawns the located binary as a child process, attaching it directly to your terminal's `stdout`, `stdin`, and `stderr` streams so you can interact with it normally.

### 2.1. Project Type Restrictions
This command is strictly reserved for projects defined with `type = "executable"` in their `Beru.toml`. 

If you attempt to invoke `beru run` within a project defined as `type = "library"` or `type = "header-only"`, the command will abort with an error. Libraries do not produce an executable entry point (`main()`) and therefore cannot be "run" in this manner. (If you wish to execute a library's test suite, you must currently run the compiled test binary manually from the `build/` directory).

---

## 3. Options and Flags

### `--profile <PROFILE>`
**Default:** `debug`

Selects the build profile to use. This operates identically to `beru build --profile`. If you wish to benchmark your application, you should always run the release profile.

```bash
beru run --profile release
```

### `[ARGS]...` (Trailing Arguments)
Any positional arguments appended to the command will be passed transparently to your spawned executable.

```bash
beru run --profile release -- --config prod.json --port 8080
```

*Note on Targets:* If your `src/` directory contains multiple executables (e.g., `day1.cpp` and `day2.cpp`), Beru will attempt to parse the first trailing argument as a target. For example, `beru run day2` will compile and execute `src/day2.cpp`.

### 3.1. Ad-Hoc Execution (Zero-Configuration Targets)

Beru brings the effortless script-execution experience of Cargo or UV to C++. 

If you pass a `.cpp` file to `beru run`, Beru will execute it as an isolated target.

**1. Inline Manifests**
Scripts can declare their own dependencies using an inline `/// beru` manifest block at the top of the file. This allows scripts to run anywhere, even outside of a Beru project. The block accepts standard `[dependencies]` and `[package]` fields (though package metadata is optional). Note that `#include` directives are not auto-detected; dependencies must be explicitly declared in the inline manifest block or inherited from an enclosing project.
```cpp
// /// beru
// [dependencies]
// fmt = "10.2.1"
// ///
#include <fmt/core.h>
int main() { fmt::print("Hello, {}!\n", "beru"); }
```

**2. Execution Context & Fallback**
Beru determines a script's dependencies based on its context:
- **Inline:** If an inline manifest is present, it is used exclusively.
- **Project Fallback:** If no inline manifest is found, but the script is inside a Beru project, it runs using the surrounding project's dependencies. A warning is emitted to stderr notifying you of this fallback behavior.
- **Zero-Context:** If no inline manifest is found and the script is not in a project, it compiles against the standard library only.

**3. Isolation**
Beru safely compiles ad-hoc scripts in a dedicated global cache directory. **Your project's `CMakeLists.txt` is never modified** when running ad-hoc scripts.

**4. Global Cache**
Ad-hoc builds are cached globally based on the script's contents and ABI profile. Repeated runs of an unchanged script will result in an instant cache hit, skipping the dependency resolution and build phases entirely.

---

## 4. Examples

**Building and executing a fresh executable project:**
```bash
beru new my-cli --type executable
cd my-cli
beru run
```
*Expected Output:*
```text
  Configuring my-cli v0.1.0
     Building my-cli
      Running `build/my-cli`
Hello from Beru!
```
