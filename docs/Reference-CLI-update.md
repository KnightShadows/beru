# Command Reference: `beru update`

The `beru update` command updates dependencies pinned in `Beru.lock` to their latest compatible versions permitted by `Beru.toml`.

---

## 1. Usage Synopsis

```bash
# Update all dependencies
beru update

# Update a specific dependency
beru update <PACKAGE_NAME>
```

## 2. Detailed Description

When dependencies are resolved, Beru records their exact versions and commit pins in `Beru.lock`. Subsequent runs of `beru build` strictly respect these pins to ensure reproducible builds.

The `beru update` command re-evaluates the dependency constraints in `Beru.toml` against the latest recipes available in the registry, local index, and embedded fallbacks, advancing `Beru.lock` without requiring manual file removal.

---

## 3. Arguments

### `[PACKAGE_NAME]` (Optional Positional Argument)
The specific package you wish to update. If omitted, all dependencies are updated.

```bash
beru update fmt
```

---

## 4. Exit Status

* `0`: Success. `Beru.lock` was updated or all packages are already up to date.
* `1`: Failure. `Beru.toml` not found, or the specified package is not a dependency of the project.
