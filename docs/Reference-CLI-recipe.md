# Command Reference: `beru recipe`

The `beru recipe` command provides developer tooling to scaffold and validate package recipes for the Beru ecosystem.

---

## 1. Usage Synopsis

```bash
# Scaffold a new recipe
beru recipe init <PACKAGE_NAME> [OPTIONS]

# Validate an existing recipe for packaging correctness
beru recipe lint [PATH]
```

---

## 2. Subcommands

### 2.1. `beru recipe init`
Generates a starter `recipe.toml` file with standardized `[package]`, `[source]`, `[build]`, and `[export]` tables.

```bash
# Scaffold with Git repository source
beru recipe init my-lib --version 1.0.0 --git https://github.com/example/my-lib.git --tag v1.0.0

# Scaffold with Tarball archive source
beru recipe init my-lib --version 1.0.0 --url https://example.com/my-lib-1.0.0.tar.gz --sha256 <64-hex-sha256>

# Scaffold a header-only library
beru recipe init my-headers --header-only
```

#### Flags and Options:
* `-v, --version <SEMVER>`: Package version string (default: `1.0.0`).
* `--git <URL>`: Upstream Git repository URL.
* `--tag <TAG>`: Immutable Git tag to pin.
* `--url <URL>`: Release archive download URL (`.tar.gz`, `.tgz`).
* `--sha256 <HASH>`: 64-character SHA-256 hash for archive verification.
* `--header-only`: Declare the package type as `header-only`.
* `--cmake-package <NAME>`: CMake package name for `find_package()`.
* `--cmake-target <TARGET>`: CMake target name (e.g. `foo::foo`).
* `-o, --output <PATH>`: Output path for `recipe.toml`. Defaults to `recipes/<pkg>/<ver>/recipe.toml` if `recipes/` exists, or `./recipe.toml`.

---

### 2.2. `beru recipe lint`
Audits an existing `recipe.toml` file against the Beru recipe specification.

```bash
beru recipe lint path/to/recipe.toml
```

Checks performed:
1. **Name Format**: Enforces valid lowercase letters, digits, and hyphens.
2. **SemVer Compliance**: Validates semantic version syntax.
3. **Source Integrity**: Ensures exactly one source is defined (Git or Tarball), verifies SHA-256 is 64 valid hex characters, and warns if Git tag is missing.
4. **Build System**: Verifies `cmake` or `custom` build configuration.
5. **Exports**: Ensures consumers are provided with header paths or CMake targets.

---

## 3. Exit Status

* `0`: Success. The recipe was scaffolded or passed lint validation.
* `1`: Failure. Validation failed or invalid arguments were supplied.
