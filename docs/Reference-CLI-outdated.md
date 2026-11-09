# Command Reference: `beru outdated`

The `beru outdated` command inspects dependencies in `Beru.lock` and displays which packages have newer versions available in the registry index.

---

## 1. Usage Synopsis

```bash
beru outdated
```

## 2. Detailed Description

`beru outdated` queries all currently locked dependencies in `Beru.lock` and compares them against the latest versions found in:
1. The global registry index (`~/.beru/index`)
2. Embedded recipe fallbacks shipped inside the Beru binary
3. Project-local recipes (`recipes/`)

It outputs an aligned table detailing:
* **Package:** The name of the dependency.
* **Current:** The currently locked version in `Beru.lock`.
* **Latest:** The highest compatible or available version in the registry.
* **Status:** Upgrade severity (`Update available (major)`, `Update available (minor)`, `Update available (patch)`, or `Up to date`).

---

## 3. Example Output

```text
Package  Current  Latest  Status
fmt      11.0.0   11.0.2  Update available (patch)
spdlog   1.12.0   1.14.1  Update available (minor)
gtest    1.14.0   1.14.0  Up to date

Run `beru update` to update dependencies.
```

---

## 4. Exit Status

* `0`: Success.
* `1`: Failure. `Beru.lock` was not found.
