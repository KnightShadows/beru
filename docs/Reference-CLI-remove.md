# Command Reference: `beru remove`

The `beru remove` command (aliased as `beru rm`) removes a dependency from your `Beru.toml` manifest and updates `Beru.lock` without requiring manual file edits.

---

## 1. Usage Synopsis

```bash
beru remove <PACKAGE_NAME>
# or using the shorthand alias:
beru rm <PACKAGE_NAME>
```

## 2. Detailed Description

When you execute `beru remove`, Beru parses your project's `Beru.toml` file into an Abstract Syntax Tree (AST) using `toml_edit`. It safely deletes the requested package entry from `[dependencies]` or `[dev-dependencies]` while strictly preserving comments, formatting, and surrounding tables.

After removing the dependency from the manifest, Beru updates `Beru.lock` to keep locked dependencies in sync.

---

## 3. Arguments

### `<PACKAGE_NAME>` (Required Positional Argument)
The exact name of the package to remove.

```bash
beru remove fmt
beru rm spdlog
```

---

## 4. Exit Status

* `0`: Success. The dependency was removed from `Beru.toml` and `Beru.lock` was updated.
* `1`: Failure. Either `Beru.toml` does not exist or the specified package was not found in the manifest.
