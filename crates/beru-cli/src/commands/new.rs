use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::path::PathBuf;

/// Arguments for `beru new`.
#[derive(Debug, Args)]
pub struct NewArgs {
    /// Name of the project to create
    pub name: String,

    /// Project type
    #[arg(long, default_value = "executable", value_parser = ["executable", "library", "header-only"])]
    pub r#type: String,

    /// C++ standard to use
    #[arg(long, default_value = "c++17")]
    pub cxx_std: String,
}

pub fn exec(args: NewArgs) -> Result<()> {
    if !beru_manifest::is_valid_package_name(&args.name) {
        if let Some(suggestion) = beru_manifest::suggest_package_name(&args.name) {
            bail!(
                "Invalid package name '{}'. Package names must be at least 2 characters, start with a letter, and contain only lowercase letters, digits, and hyphens (did you mean '{}'?).",
                args.name,
                suggestion
            );
        } else {
            bail!(
                "Invalid package name '{}'. Package names must be at least 2 characters, start with a letter, and contain only lowercase letters, digits, and hyphens.",
                args.name
            );
        }
    }

    let project_dir = PathBuf::from(&args.name);

    if project_dir.exists() {
        bail!(
            "directory '{}' already exists. Use `beru init` to initialize an existing directory.",
            args.name
        );
    }

    println!(
        "{} {} `{}` ({})",
        style("Creating").green().bold(),
        args.r#type,
        args.name,
        args.cxx_std,
    );

    std::fs::create_dir_all(project_dir.join("src")).context("failed to create src directory")?;
    std::fs::create_dir_all(project_dir.join("tests"))
        .context("failed to create tests directory")?;

    if args.r#type == "library" || args.r#type == "header-only" {
        std::fs::create_dir_all(project_dir.join("include").join(&args.name))
            .context("failed to create include directory")?;
    }

    let manifest = generate_manifest(&args.name, &args.r#type, &args.cxx_std);
    std::fs::write(project_dir.join("Beru.toml"), manifest).context("failed to write Beru.toml")?;

    let cmake = generate_cmakelists(&args.name, &args.r#type);
    std::fs::write(project_dir.join("CMakeLists.txt"), cmake)
        .context("failed to write CMakeLists.txt")?;

    match args.r#type.as_str() {
        "executable" => {
            std::fs::write(project_dir.join("src").join("main.cpp"), EXECUTABLE_MAIN)?;
        }
        "library" => {
            let header = library_header(&args.name);
            let source = library_source(&args.name);
            std::fs::write(
                project_dir
                    .join("include")
                    .join(&args.name)
                    .join(format!("{}.hpp", args.name)),
                header,
            )?;
            std::fs::write(
                project_dir.join("src").join(format!("{}.cpp", args.name)),
                source,
            )?;
        }
        "header-only" => {
            let header = header_only_lib(&args.name);
            std::fs::write(
                project_dir
                    .join("include")
                    .join(&args.name)
                    .join(format!("{}.hpp", args.name)),
                header,
            )?;
        }
        _ => {}
    }

    std::fs::write(project_dir.join(".gitignore"), GITIGNORE)?;

    let test = generate_test(&args.name, &args.r#type);
    std::fs::write(project_dir.join("tests").join("test_main.cpp"), test)?;

    println!(
        "{} project `{}`",
        style("Created").green().bold(),
        args.name
    );
    println!();
    println!("  cd {}", args.name);
    println!("  beru build");
    if args.r#type == "executable" {
        println!("  beru run");
    }

    Ok(())
}

pub(crate) fn generate_manifest(name: &str, pkg_type: &str, cxx_std: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
cxx-std = "{cxx_std}"
type = "{pkg_type}"

[dependencies]

[dev-dependencies]

[build]
system = "cmake"
"#
    )
}

pub(crate) fn generate_cmakelists(name: &str, pkg_type: &str) -> String {
    let target_name = name;
    match pkg_type {
        "executable" => format!(
            r#"cmake_minimum_required(VERSION 3.20)
project({target_name} LANGUAGES CXX)

add_executable({target_name} src/main.cpp)
beru_link_dependencies({target_name})
"#
        ),
        "library" => format!(
            r#"cmake_minimum_required(VERSION 3.20)
project({target_name} LANGUAGES CXX)

add_library({target_name} src/{name}.cpp)
beru_link_dependencies({target_name})
target_include_directories({target_name}
    PUBLIC
        $<BUILD_INTERFACE:${{CMAKE_CURRENT_SOURCE_DIR}}/include>
        $<INSTALL_INTERFACE:include>
)

install(TARGETS {target_name}
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
)
install(DIRECTORY include/ DESTINATION include)
"#
        ),
        "header-only" => format!(
            r#"cmake_minimum_required(VERSION 3.20)
project({target_name} LANGUAGES CXX)

add_library({target_name} INTERFACE)
beru_link_dependencies({target_name})
target_include_directories({target_name}
    INTERFACE
        $<BUILD_INTERFACE:${{CMAKE_CURRENT_SOURCE_DIR}}/include>
        $<INSTALL_INTERFACE:include>
)

install(DIRECTORY include/ DESTINATION include)
"#
        ),
        _ => String::new(),
    }
}

pub(crate) fn generate_test(name: &str, _pkg_type: &str) -> String {
    format!(
        r#"// {name} — test suite

#include <cassert>
#include <iostream>

int main() {{
    std::cout << "{name}: all tests passed!" << std::endl;
    return 0;
}}
"#
    )
}

pub(crate) const EXECUTABLE_MAIN: &str = r#"#include <iostream>

int main() {
    std::cout << "Hello from Beru!" << std::endl;
    return 0;
}
"#;

pub(crate) fn library_header(name: &str) -> String {
    let guard = name.replace('-', "_").to_uppercase();
    format!(
        r#"#ifndef {guard}_HPP
#define {guard}_HPP

#include <string>

namespace {ns} {{

/// Returns the library name.
std::string name();

}}

#endif
"#,
        guard = guard,
        ns = name.replace('-', "_"),
    )
}

pub(crate) fn library_source(name: &str) -> String {
    format!(
        r#"#include "{name}/{name}.hpp"

namespace {ns} {{

std::string name() {{
    return "{name}";
}}

}}
"#,
        name = name,
        ns = name.replace('-', "_"),
    )
}

pub(crate) fn header_only_lib(name: &str) -> String {
    let guard = name.replace('-', "_").to_uppercase();
    format!(
        r#"#ifndef {guard}_HPP
#define {guard}_HPP

#include <string>

namespace {ns} {{

/// Returns the library name.
inline std::string name() {{
    return "{name}";
}}

}}

#endif
"#,
        guard = guard,
        ns = name.replace('-', "_"),
        name = name,
    )
}

pub(crate) const GITIGNORE: &str = r#"# Build artifacts
build/
target/

# Beru generated
beru-toolchain.cmake

# IDE
.vscode/
.idea/
*.swp
*.swo
compile_commands.json
"#;
