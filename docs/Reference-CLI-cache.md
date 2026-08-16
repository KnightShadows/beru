# Command Reference: `beru cache`

The `beru cache` command allows you to inspect and manage Beru's global binary and source cache (`~/.beru/cache/`).

---

## 1. Usage Synopsis

```bash
beru cache <COMMAND>
```

---

## 2. Subcommands

### 2.1. `beru cache size`

Calculates and displays disk usage for each category of the global cache, as well as the total disk footprint.

```bash
$ beru cache size
  sources    128.4 MB
  builds     412.0 MB
  adhoc      18.2 MB
  total      558.6 MB
```

### 2.2. `beru cache clean`

Safely removes cached data from the global cache. When invoked without flags, it cleans all cached categories (sources, git clones, precompiled builds, and ad-hoc builds).

```bash
beru cache clean [OPTIONS]
```

#### Flags for `clean`:

*   **`--sources`**: Remove only downloaded source archives (tarballs) and git repositories (`~/.beru/cache/sources/` and `~/.beru/cache/git/`).
*   **`--builds`**: Remove only precompiled dependency artifacts (`~/.beru/cache/builds/`).
*   **`--adhoc`**: Remove only the compilation artifacts for standalone/ad-hoc single-file scripts (`~/.beru/cache/adhoc/`).

---

## 3. Examples

**Inspecting total cache usage:**
```bash
beru cache size
```

**Freeing up disk space by cleaning only downloaded tarballs and git clones:**
```bash
beru cache clean --sources
```

**Clearing only ad-hoc single-file script compilation caches:**
```bash
beru cache clean --adhoc
```

**Wiping all global caches completely:**
```bash
beru cache clean
```
