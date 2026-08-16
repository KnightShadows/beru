# Command Reference: `beru add`

The `beru add` command allows you to add a new dependency directly to your `Beru.toml` manifest without manually editing the file. It is the C++ equivalent of `cargo add` or `npm install`.

---

## 1. Usage Synopsis

```bash
beru add <PACKAGE_NAME> [OPTIONS]
```

## 2. Detailed Description

When you execute `beru add`, Beru parses your local `Beru.toml` file into an Abstract Syntax Tree (AST) using `toml_edit`. It safely injects the requested dependency into the `[dependencies]` table while preserving your exact file formatting, comments, and structure.

By default, the command expects a valid semantic version. However, Beru's robust resolution engine allows adding dependencies via Git repositories or local paths as well.

### 2.1. Version Resolution
Currently, Beru requires an explicit version if fetching from the registry, but future implementations will query the index to automatically append the latest compatible semantic version.

---

## 3. Options and Flags

### `PACKAGE_NAME` (Required Positional Argument)
The name of the package you wish to add. 

```bash
beru add fmt
```

You can also use the shorthand `name@version` syntax to specify a version inline:

```bash
beru add fmt@11.0.2
```

### `--version <VERSION>`
Specifies the exact semantic version of the package.

```bash
beru add fmt --version 11.0.2
```

### `--git <URL>`
Specifies a Git repository URL instead of fetching from the registry.

```bash
beru add spdlog --git https://github.com/gabime/spdlog.git
```

### `--tag <TAG>`
Used in conjunction with `--git`. Specifies the Git tag to checkout.

```bash
beru add spdlog --git https://github.com/gabime/spdlog.git --tag v1.14.1
```

### `--rev <COMMIT_HASH>`
Used in conjunction with `--git`. Specifies the exact Git commit hash to checkout.

### `--branch <BRANCH>`
Used in conjunction with `--git`. Specifies the branch to checkout.

```bash
beru add spdlog --git https://github.com/gabime/spdlog.git --branch develop
```

### `--path <PATH>`
Specifies a local filesystem path to a dependency. This is highly useful for monorepos or local package development.

```bash
beru add my-local-lib --path ../my-local-lib
```

---

## 4. Examples

**Adding a registry dependency:**
```bash
beru add fmt --version 11.0.2
```

**Adding a Git dependency pinned to a tag:**
```bash
beru add spdlog --git https://github.com/gabime/spdlog.git --tag v1.14.1
```

**Adding a local path dependency:**
```bash
beru add math-utils --path ../libs/math-utils
```
