# `beru setup`

The `beru setup` command inspects the host system for required and recommended C++ development prerequisites, similar to `rustup` for the Rust ecosystem.

It checks:
1. **git**: Required for cloning the package registry index and fetching git-based dependencies.
2. **cmake**: Required for C++ build orchestration (verifies version $\ge$ 3.20).
3. **C++ compiler**: Probes for `clang++`, `g++`, or Microsoft Visual C++ (`cl.exe`).
4. **ninja**: Recommended for fast, parallel build generation.
5. **PATH**: Checks if `~/.beru/bin` is in the user's environment `PATH`.

---

### Usage

```bash
beru setup [OPTIONS]
```

---

### Options

* `--check`: Runs the diagnostic check and exits with code `0` if all required prerequisites are satisfied, or code `1` if any required tool is missing. Ideal for CI pipelines and install scripts.
* `--install-prereqs`: Displays and runs automated package manager installation commands for missing tools (`apt-get`, `dnf`, `pacman`, `brew`, `winget`).
* `--add-to-path`: Automatically adds `~/.beru/bin` to the user's persistent environment `PATH` (via Windows registry or Unix shell profile files like `~/.bashrc`, `~/.zshrc`, `~/.profile`).

---

### Examples

#### Inspect Toolchain Status
```bash
beru setup
```

Output:
```text
=== Beru Environment & Toolchain Setup ===

  git             [OK]     git version 2.43.0
  cmake           [OK]     cmake version 3.28.3 (>= 3.20 supported)
  C++ compiler    [OK]     g++ (GCC) 13.2.0
  ninja           [OK]     ninja 1.11.1
  PATH            [OK]     ~/.beru/bin is in PATH

Success: All prerequisites are met! You are ready to build with Beru.
```

#### Automated Prerequisite Verification in CI
```bash
beru setup --check
```

#### Configure PATH Automatically
```bash
beru setup --add-to-path
```

---

### Exit Status

* `0`: Success (or all required tools present under `--check`).
* `1`: Failure (missing required tools when run with `--check`).

