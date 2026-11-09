# Concepts: The Decentralized Registry & Index

A package manager is only as good as the packages it can provide. Historically, package ecosystems like npm, PyPI, and RubyGems have relied on massive, centralized architectures. They require load-balanced HTTP APIs, highly available databases, and CDN infrastructure to serve metadata and tarballs to millions of users globally.

Beru rejects this centralized paradigm entirely. Inspired by the architectural brilliance of CocoaPods and early Homebrew, Beru leverages a **decentralized Git index** to manage its registry metadata.

This chapter explains the mechanics of the Beru index and why this decentralized approach provides superior security, speed, and enterprise flexibility.

---

## 1. The Anatomy of the Index

In the Beru ecosystem, the "registry" is not a website. It is simply a Git repository containing 701 `recipe.toml` files across 223 packages.

The official, public index is hosted on GitHub at `https://github.com/KnightShadows/beru_index.git`.

### 1.1. The Directory Structure
If you browse this repository, you will not find any C++ source code. Instead, you will find a highly structured hierarchy of metadata files, known as recipes. 

To ensure `beru resolve` can perform fast direct lookups on the filesystem without parsing unnecessary files, the index is strictly organized:

```text
beru_index/
├── fmt/
│   ├── 9.1.0/
│   │   └── recipe.toml
│   ├── 10.2.1/
│   │   └── recipe.toml
│   └── 11.0.2/
│       └── recipe.toml
├── spdlog/
│   └── 1.14.1/
│       └── recipe.toml
└── ... and other packages across the registry
```

Each `recipe.toml` acts as a blueprint. It contains the upstream source location—either a release tarball URL with a cryptographic `sha256` checksum, or a Git repository with a release tag that Beru cryptographically pins to a commit SHA in `Beru.lock`—along with the build instructions.

### 1.2. The Local Clone (`~/.beru/index/`)
Running `beru index update` performs a shallow clone (`git clone --depth 1`) of this repository into the `~/.beru/index/` directory in your home folder. For offline usage, Beru also bundles and embeds core recipes directly inside the binary.

This local clone is the secret to Beru's extreme speed. When the PubGrub algorithm needs to know what versions of `fmt` exist to satisfy a constraint, **Beru does not make a single HTTP request.** It simply reads the subdirectories within `~/.beru/index/fmt/`. 

Dependency resolution is entirely offline, making it instantaneous and completely immune to network latency or upstream API outages.

---

## 2. Synchronization and the Pull Model

Because the index is stored locally, it is inherently static. If a maintainer publishes a new recipe for `fmt 11.1.0` to the GitHub repository today, your local machine will not know about it.

To sync your machine with the global ecosystem, you must explicitly run:
```bash
beru index update
```

This command simply navigates to `~/.beru/index/` and performs a fast-forward Git pull. This fetches the latest tree of `recipe.toml` files, making the new packages available to your local resolver.

---

## 3. The Enterprise Advantage: Private Registries

The most profound advantage of the Git-based index model is how easily it supports enterprise environments.

In heavily regulated industries (finance, aerospace, defense), developers are often completely firewalled from the public internet. They cannot reach npmjs.com or GitHub. Furthermore, companies have hundreds of proprietary, internal C++ libraries that they wish to share across teams, but cannot publish to a public index.

Setting up a private registry for npm or Python requires deploying complex Docker containers (like Artifactory or Verdaccio), configuring databases, and managing reverse proxies.

**Setting up a private registry in Beru takes 30 seconds.**

1.  Create a blank Git repository on your company's internal GitLab/Bitbucket server (e.g., `git@gitlab.internal.company.com:core-tech/beru-registry.git`).
2.  Commit your proprietary `recipe.toml` files to it, structured exactly like the public index.
3.  Instruct your engineers to run:
    ```bash
    beru index update --url git@gitlab.internal.company.com:core-tech/beru-registry.git
    ```

Beru will clone the private repository into the `~/.beru/index/` folder. The developer can now depend on `internal-auth-lib` in their `Beru.toml`, and Beru will resolve and build it perfectly, without ever sending a packet to the public internet.
