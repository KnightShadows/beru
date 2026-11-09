# Command Reference: `beru index update`

The `beru index update` command synchronizes your local machine with the decentralized global package registry. It is the mechanism by which you discover new libraries and new versions of existing libraries published by the community.

---

## 1. Usage Synopsis

```bash
beru index update [OPTIONS]
```

## 2. Detailed Description

Unlike traditional package managers that rely on centralized databases and REST APIs, Beru's architecture is radically decentralized. The entire global registry is simply a Git repository containing lightweight `recipe.toml` metadata across hundreds of package recipes.

The Beru binary embeds offline recipes for core libraries (such as `fmt`, `spdlog`, `nlohmann_json`, and `gtest`). To discover and resolve third-party libraries across the entire ecosystem, running `beru index update` performs a shallow clone of the official repository (`https://github.com/KnightShadows/beru_index.git`) into the `~/.beru/index/` directory on your hard drive.

### 2.1. The Need for Synchronization
Because resolution operations (`beru build`, `beru resolve`) operate entirely offline by reading the `~/.beru/index/` directory, they are exceptionally fast. However, they are fundamentally ignorant of the outside world.

If the author of `{fmt}` publishes version `11.1.0` to GitHub today, and a community member merges a PR adding that recipe to the official Beru index tomorrow, your machine will not know about it. If you add `fmt = "11.1.0"` to your `Beru.toml`, `beru build` will fail, complaining that the version does not exist.

You must run `beru index update` periodically. 

### 2.2. Execution Mechanics
Under the hood, this command simply navigates to `~/.beru/index/` and executes a `git pull --ff-only`. If the directory is missing or corrupted, it will delete the folder and execute a fresh `git clone --depth 1`.

---

## 3. Options and Flags

### `--url <URL>`
**Default:** `https://github.com/KnightShadows/beru_index.git`

This flag allows you to completely override the source of the package registry. It is the cornerstone feature for enterprise environments.

By pointing the `--url` flag to a private Git repository hosted on your internal network (e.g., an internal GitLab or Bitbucket server), you can force your local Beru orchestrator to resolve dependencies exclusively against your proprietary, internally audited C++ libraries.

---

## 4. Examples

**Updating the public registry to discover the latest library versions:**
```bash
beru index update
```

**Switching to a private enterprise registry:**
```bash
beru index update --url git@gitlab.internal.company.com:core-tech/beru_registry.git
```
