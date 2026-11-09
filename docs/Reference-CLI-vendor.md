# `beru vendor`

The `beru vendor` command packages all project dependencies locally into the project directory (by default under `./vendor/<pkg>-<version>/`), enabling offline, hermetic, and air-gapped enterprise builds.

---

### Usage

```bash
beru vendor [PATH] [OPTIONS]
```

---

### Arguments

* `[PATH]`: Directory where dependencies will be vendored (defaults to `vendor` inside the project root).

---

### Options

* `--no-delete`: Do not delete existing vendored package directories prior to copying files.

---

### Behavior

1. Reads dependencies from `Beru.lock` (bails if `Beru.lock` is missing; run `beru resolve` first).
2. For each locked dependency, downloads or resolves the exact pinned source tree and recipe definition.
3. Recursively copies the source files into `<vendor_dir>/<name>-<version>/` (skipping `.git`, `build`, and `.beru`).
4. Embeds `recipe.toml` within the package directory for self-contained recipe metadata.
5. In subsequent builds (`beru build` or `beru build --offline`), Beru automatically detects vendored directories and prioritizes them over network or global caches without needing internet access.

---

### Examples

Vendor all locked dependencies into the default `./vendor` directory:
```bash
beru vendor
```

Vendor dependencies into a custom path:
```bash
beru vendor third_party/vendor
```

Build offline using vendored dependencies:
```bash
beru build --offline
```

---

### Exit Status

* `0`: Dependencies vendored successfully (or none to vendor).
* `1`: Error resolving dependencies or writing vendored sources.

