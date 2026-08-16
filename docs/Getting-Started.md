# Getting Started with Beru

This chapter will guide you through the process of setting up Beru on your machine, scaffolding your very first C++ project, understanding the generated files, and executing a complete build. By the end of this chapter, you will have a fully functioning C++ application managed entirely by Beru.

---

## 1. Prerequisites and System Requirements

Because Beru is an orchestrator, it relies on foundational tools already present on your system to perform the heavy lifting of cloning repositories and invoking the compiler. Before installing Beru, ensure your system meets the following prerequisites.

### 1.1. Git (Version 2.x+)
Beru uses Git extensively. It uses it to clone the central package index, to fetch source code for Git-based dependencies, and to read metadata.
*   **Verification:** Run `git --version` in your terminal. Ensure you are running at least version 2.0.

### 1.2. CMake (Version 3.20+)
While Beru abstracts CMake away from the user, it leverages CMake under the hood to configure and compile third-party libraries. Version 3.20 is strictly required because it introduced critical features for target exporting and modern dependency tracking that Beru relies upon.
*   **Verification:** Run `cmake --version`. 
*   **Installation:** If you do not have it, install it via your system package manager (`apt`, `brew`, `pacman`) or download it directly from [cmake.org](https://cmake.org/download/).

### 1.3. A C++ Compiler
Beru does not bundle a compiler. It will use the default C++ compiler available on your system's `PATH`.
*   **Linux:** GCC 9+ or Clang 10+ (`sudo apt install build-essential`).
*   **macOS:** Apple Clang (installed via `xcode-select --install`).
*   **Windows:** MSVC 2019 or later (installed via Visual Studio Build Tools, ensuring the "Desktop development with C++" workload is selected).

---

## 2. Installing Beru

Beru is distributed as a standalone, statically linked binary. Installation is automated via simple shell scripts.

### 2.1. Installation on Linux and macOS

Open your terminal and execute the following command. This will download the latest release from the official repository and install the binary (typically in `~/.cargo/bin` or `~/.local/bin`).

```bash
curl -fsSL https://raw.githubusercontent.com/KnightShadows/Beru/main/install.sh | bash
```

**Modifying your PATH:**
The installation script will automatically check if the target directory is in your `PATH` and append it to your shell configuration (e.g., `~/.bashrc`, `~/.zshrc`, or `~/.profile`) if needed:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### 2.2. Installation on Windows

Open a PowerShell window (it does not need to be Administrator) and execute:

```powershell
irm https://raw.githubusercontent.com/KnightShadows/Beru/main/install.ps1 | iex
```

The script will download the `.exe` and place it in `C:\Users\<YourUsername>\.beru\bin`. It will automatically attempt to add this directory to your User `PATH` environment variable. You may need to restart your terminal for the changes to take effect.

### 2.3. Verifying the Installation

To confirm Beru is installed correctly and accessible, run:

```bash
beru --version
```

You should see output similar to `beru 0.3.1`. If you receive a "command not found" error, verify that the `bin` directory was correctly added to your `PATH`.

---

## 3. Scaffolding Your First Project

With Beru installed, we can now create a new project. We will use the `beru new` command, which automatically generates a standardized directory structure and the necessary boilerplate files.

In your terminal, navigate to the directory where you keep your code, and run:

```bash
beru new hello_beru --type executable --cxx-std c++20
```

*Note: The `--type` flag defaults to `executable`, and `--cxx-std` defaults to `c++17`. We have explicitly provided them here for educational purposes.*

Navigate into the newly created directory:

```bash
cd hello_beru
```

### 3.1. Understanding the Project Layout

If you list the contents of the `hello_beru` directory, you will see the following structure:

```text
hello_beru/
├── Beru.toml           # The project manifest
├── CMakeLists.txt      # A minimal CMake file for the local project
├── .gitignore          # Pre-configured to ignore build artifacts
├── src/
│   └── main.cpp        # Your application's entry point
└── tests/
    └── test_main.cpp   # A stub for your test suite
```

### 3.2. The Manifest: Beru.toml

Open the `Beru.toml` file in your favorite text editor. It should look like this:

```toml
[package]
name = "hello_beru"
version = "0.1.0"
cxx-std = "c++20"
type = "executable"

[dependencies]

[dev-dependencies]

[build]
system = "cmake"
```

This file is the heart of your project. It replaces hundreds of lines of complex CMake logic. Here is what the key fields mean:
*   `name`: The name of your executable. This is also the name Beru uses to cache the project if it is a library.
*   `version`: A strict Semantic Versioning (SemVer) string.
*   `cxx-std`: Instructs the compiler to enforce the C++20 standard. Crucially, Beru will ensure that any dependencies you add later are *also* compiled in a C++20 compatible way, preventing subtle ABI (Application Binary Interface) bugs.
*   `[dependencies]`: This table is currently empty, but this is where you will list the third-party libraries your project needs.

---

## 4. Building and Running the Project

Now that we understand the structure, let's compile the application.

```bash
beru run
```

The `run` command is a convenience wrapper. It tells Beru to build the project and, if the build is successful and the project is an `executable`, immediately execute the resulting binary.

### 4.1. What happens during a build?

When you execute a build command, Beru performs a precise orchestration sequence:

1.  **Resolution:** Beru reads `Beru.toml`. Since there are no dependencies yet, this step finishes instantly. (If there were dependencies, it would invoke the PubGrub algorithm).
2.  **Toolchain Generation:** Beru synthesizes a `beru-toolchain.cmake` file in your project root. This file contains the precise include paths and linker flags for any dependencies.
3.  **Compilation:** Beru invokes your system's CMake to read the local `CMakeLists.txt`, injecting the synthesized toolchain. CMake then invokes your compiler (GCC/Clang/MSVC) to compile `src/main.cpp`.
4.  **Execution:** Finally, Beru locates the output binary (typically located in `build/hello_beru` on Unix or `build/Debug/hello_beru.exe` on Windows) and spawns it.

**Expected Output:**

```text
  Configuring hello_beru v0.1.0
     Building hello_beru
      Running `build/hello_beru`
Hello from Beru!
```

Congratulations! You have successfully compiled and executed a C++ application managed by Beru.

### 4.2. Testing and Fast Syntax Checks

If you want to rapidly iterate without waiting for the linker, you can use the `check` command. It runs a fast syntax-only compilation over your project:

```bash
beru check
```

When you are ready to run your test suite, Beru automatically wraps CTest for you, executing your tests in parallel across all available CPU cores:

```bash
beru test
```

### 4.3. Ad-Hoc Execution (Zero-Configuration Targets)

Beru brings the effortless script-execution experience of Cargo or UV to C++. 

If you want to run a quick test script or a standalone file (e.g., for competitive programming or Advent of Code), you do **not** need to manually edit your `CMakeLists.txt`. Simply pass the file path to `beru run`:

```bash
beru run src/my_script.cpp
```

Beru will detect that `my_script.cpp` is not in your `CMakeLists.txt` and will safely auto-append it for you. Crucially, it will invoke a special `beru_link_dependencies` macro, which means `my_script.cpp` immediately has access to every library you've added to `Beru.toml`!

---

## 5. Cleaning Up

During the build, CMake generated various temporary files (object files, Makefiles, or Ninja build files) and placed them inside the `build/` directory.

To reset your project to a pristine state, simply run:

```bash
beru clean
```

This command safely deletes the `build/` directory and generated CMake files (`beru-toolchain.cmake`, `beru-override.cmake`).

In the next chapter, we will look at how Beru's workflow compares to traditional C++ workflows, preparing you to migrate your existing projects.
