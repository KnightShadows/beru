# `beru search`

The `beru search` command searches for packages across the local package registry index clone (`~/.beru/index/`), bundled recipes, project-local recipes, and embedded fallback packages.

---

### Usage

```bash
beru search <QUERY> [OPTIONS]
```

---

### Options

* `<QUERY>`: Search term matched case-insensitively against package names and descriptions.
* `--limit <N>`: Maximum number of packages to display (default: `25`).

---

### Example

```bash
$ beru search json

Found 1 package(s) matching 'json':

  Package        Latest  Type         Description
  -------------  ------  -----------  -----------------------------------
  nlohmann_json  3.11.3  header-only  JSON for Modern C++
```

---

### Exit Status

* `0`: Success (packages listed or no matches notice displayed).
* `1`: Failure.
