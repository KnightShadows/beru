# Command Reference: `beru install` & `beru uninstall`

The `beru install` command builds an executable C++ project in Release mode and installs the compiled binary into the global Beru binary directory (`~/.beru/bin`), allowing it to be executed globally.

`beru uninstall` removes an installed binary from the global binary directory.

---

## 1. Usage Synopsis

```bash
# Install current executable project
beru install

# Install an executable project from a specific path
beru install --path /path/to/project

# Uninstall a binary
beru uninstall <PACKAGE_NAME>
```

---

## 2. Detailed Description

### `beru install`
`beru install` reads `Beru.toml` from the target project directory and validates that the project is of type `executable` (`type = "executable"`). It then:
1. Orchestrates a release build of the project and its dependencies.
2. Locates the compiled binary within the `build/` directory.
3. Copies the executable to `~/.beru/bin/<name>` (or `<name>.exe` on Windows).
4. On Unix platforms, sets executable file permissions (`0o755`).
5. Warns if `~/.beru/bin` is not currently in the user's `PATH`.

### `beru uninstall`
`beru uninstall <PACKAGE_NAME>` searches `~/.beru/bin/` for the binary (checking both bare names on Unix and `.exe` extensions on Windows) and safely removes it.

---

## 3. Exit Status

* `0`: Success. The binary was installed or uninstalled.
* `1`: Failure. Either the target is not an executable project, build failed, or the binary to uninstall was not found.

