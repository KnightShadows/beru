# `beru fmt`

The `beru fmt` command automatically formats all C++ source and header files (`.cpp`, `.hpp`, `.cc`, `.cxx`, `.h`, `.c`) in the current project or workspace using `clang-format`.

---

### Usage

```bash
beru fmt [OPTIONS]
```

---

### Options

* `--check`: Checks whether all C++ files are formatted without modifying them. Exits with code `0` if all files match formatting rules, or code `1` if any files require formatting. Ideal for CI build validation.
* `-p, --package <PACKAGE>`: Format only a specific member package within a monorepo workspace.
* `-v, --verbose`: Prints detailed status for each formatted file.

---

### Prerequisites

`beru fmt` requires `clang-format` on your system `PATH`. If missing, install it via:
* **Windows**: `winget install LLVM.LLVM`
* **macOS**: `brew install clang-format`
* **Linux**: `sudo apt-get install -y clang-format` (or run `beru setup --install-prereqs`)

---

### Exit Status

* `0`: Success. All files formatted (or verified clean under `--check`).
* `1`: Failure. Formatting check failed or `clang-format` returned an error.
