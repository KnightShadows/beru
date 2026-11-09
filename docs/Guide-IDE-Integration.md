# Guide: IDE and Plain CMake Integration

Beru is designed to work seamlessly with modern C++ IDEs and existing build infrastructure. Under the hood, Beru generates a standard CMake toolchain file (`beru-toolchain.cmake`) and a `CMakePresets.json` file. This architecture provides rich code completion, syntax highlighting, and debugging in your favorite editor without requiring proprietary IDE plugins.

---

## 1. How Beru Integrates with CMake

When you scaffold a project with `beru new` or initialize an existing directory with `beru init`, Beru creates:

1. **`CMakeLists.txt`**: A clean, standard CMake file guarded with `if(COMMAND beru_link_dependencies)` so that plain `cmake` invocations continue to work standalone.
2. **`CMakePresets.json`**: Standard CMake preset configuration that automatically passes `-DCMAKE_TOOLCHAIN_FILE=beru-toolchain.cmake` and enables `CMAKE_EXPORT_COMPILE_COMMANDS ON`.
3. **`beru-toolchain.cmake`**: Generated when you run `beru build`, `beru resolve`, or `beru run`. This file contains the include paths, linker flags, and imported CMake targets for all your resolved dependencies.

Before opening your project in an IDE for the first time, run `beru build` (or `beru resolve`) to generate `beru-toolchain.cmake` and compile your dependencies:

```bash
beru build
```

---

## 2. CLion Setup

JetBrains CLion natively supports CMake Presets and custom toolchain files.

### Using CMake Presets (Recommended)
1. Open CLion and choose **File → Open**, selecting your project directory.
2. CLion detects `CMakePresets.json` automatically and displays the `default` preset.
3. Click **Load** or select the `default` preset under **Settings → Build, Execution, Deployment → CMake**.
4. CLion will configure the project using `beru-toolchain.cmake`, index all dependency headers, and provide full code completion.

### Manual Toolchain File Setup
If you prefer manual configuration:
1. Go to **Settings/Preferences → Build, Execution, Deployment → CMake**.
2. Select your active CMake profile (e.g., `Debug`).
3. In the **CMake options** field, add:
   ```text
   -DCMAKE_TOOLCHAIN_FILE=beru-toolchain.cmake
   ```
4. Click **Apply** and **OK**.

---

## 3. Visual Studio Code Setup

VS Code provides first-class CMake and C++ support through Microsoft's official extensions.

### Prerequisites
Install the following extensions from the VS Code Marketplace:
* **C/C++** (`ms-vscode.cpptools`) or **clangd** (`llvm-vs-code-extensions.vscode-clangd`)
* **CMake Tools** (`ms-vscode.cmake-tools`)

### Configuration Steps
1. Open your project folder in VS Code (`code .`).
2. The **CMake Tools** extension will prompt you to select a configure preset. Choose **Default Config** (`default`).
3. If not prompted automatically, open the Command Palette (`Ctrl+Shift+P` or `Cmd+Shift+P`) and run:
   ```text
   CMake: Select Configure Preset → Default Config
   ```
4. Because `CMAKE_EXPORT_COMPILE_COMMANDS` is enabled, CMake generates `build/compile_commands.json`. Language servers like `clangd` or Microsoft C/C++ IntelliSense will automatically use this file for navigation and autocomplete across all Beru dependencies.

---

## 4. Visual Studio Setup

Visual Studio 2019 and 2022 include built-in support for CMake projects and `CMakePresets.json`.

1. Launch Visual Studio and select **Open a local folder**.
2. Select your Beru project directory.
3. Visual Studio detects `CMakePresets.json` and selects the `default` configuration preset in the top toolbar dropdown.
4. Open the Solution Explorer in CMake Targets View or Folder View.
5. Build or debug targets directly via **Build → Build All** or by pressing `F5`.

---

## 5. Working with Plain CMake

Beru projects do not require the `beru` binary for subsequent builds once dependencies are locked and the toolchain is generated.

### Building via CMake Presets
```bash
# Configure using the default preset
cmake --preset default

# Build
cmake --build --preset default
```

### Building via Explicit Toolchain Argument
```bash
# Configure
cmake -S . -B build -DCMAKE_TOOLCHAIN_FILE=beru-toolchain.cmake

# Compile
cmake --build build
```

### Standalone Plain CMake (Without Beru)
If a collaborator clones the repository and does not yet have Beru or dependencies installed, plain CMake commands run cleanly without error:
```bash
cmake -S . -B build
cmake --build build
```
The `if(COMMAND beru_link_dependencies)` guard ensures CMake does not fail with an "Unknown CMake command" error.
