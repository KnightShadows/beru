# `beru self-update`

The `beru self-update` command downloads and installs the latest official release of the Beru binary from GitHub Releases, replacing the currently executing executable in place.

On Windows, it handles running executable file locks by renaming the active binary to `.old`, installing the new release, and removing the old binary. On Linux and macOS, it atomically replaces the binary via permissions and standard filesystem replacement.

---

### Usage

```bash
beru self-update [OPTIONS]
```

---

### Options

* `--check`: Checks GitHub Releases for a newer version without downloading or installing it.
* `--version <VERSION>`: Downloads and installs a specific target version (e.g. `--version 0.5.7`).
* `--force`: Forces a reinstall even if the current binary is already up to date.
* `--repo <OWNER/REPO>`: Target GitHub repository (defaults to `KnightShadows/beru`).
* `--offline`: Fails fast with an error message because self-update requires network access.

---

### Environment Variables

* `BERU_UPDATE_REPO`: Overrides the default repository owner/repo.
* `BERU_SELF_UPDATE_URL`: Overrides download source (can point to a local file path, `file://` URI, or custom mirror URL for hermetic environments and offline caches).
* `BERU_OFFLINE`: When set, self-update fails immediately without attempting network queries.

---

### Exit Status

* `0`: Success. Beru was checked or updated.
* `1`: Failure. Network error, release not found, or unsupported platform.

