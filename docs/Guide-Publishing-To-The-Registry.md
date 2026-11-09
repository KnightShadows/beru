# Guide: Publishing to the Global Registry

The true power of Beru lies in its community. The global package index is not a walled garden controlled by a central authority; it is an open-source Git repository hosted on GitHub. 

When you author a recipe for a C++ library that doesn't yet exist in the ecosystem, publishing it is as simple as opening a Pull Request. Once merged, every Beru user worldwide gains instant access to that library via `beru index update`.

This chapter outlines the exact workflow and review criteria for contributing to the Beru index.

---

## 1. The Git Workflow

The official Beru index is located at:  
**[https://github.com/KnightShadows/beru_index](https://github.com/KnightShadows/beru_index)**

### 1.1. Forking and Cloning
To contribute, you must first fork the repository to your personal GitHub account. Once forked, clone it to your local machine:

```bash
git clone https://github.com/<YOUR_GITHUB_USERNAME>/beru_index.git
cd beru_index
```

### 1.2. Directory Structure Conventions
The index enforces a strict, hierarchical directory structure to ensure fast direct lookups by the package manager. The structure is `<package_name>/<semantic_version>/recipe.toml`.

For example, if you are adding version `1.14.1` of the `spdlog` library, you must create the necessary directories:

```bash
mkdir -p spdlog/1.14.1
```

### 1.3. Adding the Recipe
Create the `recipe.toml` file within that version directory. Ensure you have followed all the best practices outlined in the [Authoring a Recipe](Guide-Writing-A-Recipe.md) chapter, paying special attention to cryptographic checksums and accurate CMake target exports.

```bash
# Example
touch spdlog/1.14.1/recipe.toml
```

---

## 2. Pre-Flight Checks

Do not submit untested recipes. The CI pipeline for the index is rigorous, and reviewers expect high-quality submissions. Before committing, verify the following:

1.  **SemVer Compliance:** Ensure the folder name `1.14.1` matches the `version = "1.14.1"` field inside the TOML file exactly.
2.  **Checksum Accuracy:** If using a `.tar.gz` source, deliberately change the last character of your `sha256` string and run a local build. Ensure Beru fiercely rejects it. Then change it back. This confirms your checksum logic is active.
3.  **Local Compilation:** Copy your recipe to your actual `~/.beru/index/` directory, create a blank project, depend on it, and run `beru build`. **If it does not link locally, it will be rejected.**

---

## 3. Submitting the Pull Request

Once you are confident in your recipe, commit the changes to a feature branch and push to your fork.

```bash
git checkout -b add-spdlog-1.14.1
git add spdlog/1.14.1/recipe.toml
git commit -m "feat: add recipe for spdlog 1.14.1"
git push origin add-spdlog-1.14.1
```

Navigate to the original `KnightShadows/beru_index` repository on GitHub and click "Compare & pull request".

### 3.1. The Review Criteria

The maintainers of the Beru index will evaluate your Pull Request against the following strict criteria:

*   **Immutability:** Recipes must point to immutable release artifacts. Pointing a `url` to a `master.zip` or pointing a Git source to a moving `branch` is strictly prohibited, as it destroys the reproducibility guarantee of the `Beru.lock` file. You must use explicit release tags or tarballs.
*   **Target Accuracy:** The `cmake_targets` array must contain the official targets intended by the upstream authors. Do not invent target names.
*   **Security:** The `sha256` field is mandatory for all archive downloads. PRs omitting the checksum will be automatically rejected by CI.

Once your PR is reviewed and merged into the `main` branch, the deployment is instantaneous. The next time a developer anywhere in the world runs `beru index update`, your package will materialize on their machine, ready to be linked into their next great C++ project.
