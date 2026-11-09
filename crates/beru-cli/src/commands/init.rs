use anyhow::{Context, Result};
use clap::Args;
use console::style;

/// Arguments for `beru init`.
#[derive(Debug, Args)]
pub struct InitArgs {
    /// Project type
    #[arg(long, default_value = "executable", value_parser = ["executable", "library", "header-only"])]
    pub r#type: String,

    /// C++ standard to use
    #[arg(long, default_value = "c++17")]
    pub cxx_std: String,
}

pub fn exec(args: InitArgs) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let raw_name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-project");

    // Sanitize into a valid Beru package name using manifest rules
    let name = if beru_manifest::is_valid_package_name(raw_name) {
        raw_name.to_string()
    } else if let Some(suggested) = beru_manifest::suggest_package_name(raw_name) {
        suggested
    } else {
        "my-project".to_string()
    };

    if cwd.join("Beru.toml").exists() {
        println!(
            "{} Beru.toml already exists in this directory.",
            style("Warning:").yellow().bold()
        );
        return Ok(());
    }

    println!(
        "{} Beru in existing directory ({})",
        style("Initializing").green().bold(),
        name,
    );

    // Create directories if they don't exist
    let _ = std::fs::create_dir_all(cwd.join("src"));
    let _ = std::fs::create_dir_all(cwd.join("tests"));

    if args.r#type == "library" || args.r#type == "header-only" {
        let _ = std::fs::create_dir_all(cwd.join("include").join(&name));
    }

    // Write Beru.toml
    let manifest = super::new::generate_manifest(&name, &args.r#type, &args.cxx_std);
    std::fs::write(cwd.join("Beru.toml"), manifest).context("failed to write Beru.toml")?;

    // Write CMakeLists.txt if it doesn't exist
    if !cwd.join("CMakeLists.txt").exists() {
        let cmake = super::new::generate_cmakelists(&name, &args.r#type);
        let _ = std::fs::write(cwd.join("CMakeLists.txt"), cmake);
    }

    // Write CMakePresets.json if it doesn't exist
    if !cwd.join("CMakePresets.json").exists() {
        let presets = super::new::generate_cmake_presets();
        let _ = std::fs::write(cwd.join("CMakePresets.json"), presets);
    }

    // Write source files if they don't exist
    match args.r#type.as_str() {
        "executable" => {
            let main_path = cwd.join("src").join("main.cpp");
            if !main_path.exists() {
                let _ = std::fs::write(main_path, super::new::EXECUTABLE_MAIN);
            }
        }
        "library" => {
            let header_path = cwd
                .join("include")
                .join(&name)
                .join(format!("{}.hpp", name));
            if !header_path.exists() {
                let header = super::new::library_header(&name);
                let _ = std::fs::write(header_path, header);
            }
            let source_path = cwd.join("src").join(format!("{}.cpp", name));
            if !source_path.exists() {
                let source = super::new::library_source(&name);
                let _ = std::fs::write(source_path, source);
            }
        }
        "header-only" => {
            let header_path = cwd
                .join("include")
                .join(&name)
                .join(format!("{}.hpp", name));
            if !header_path.exists() {
                let header = super::new::header_only_lib(&name);
                let _ = std::fs::write(header_path, header);
            }
        }
        _ => {}
    }

    let gitignore_path = cwd.join(".gitignore");
    if !gitignore_path.exists() {
        let _ = std::fs::write(gitignore_path, super::new::GITIGNORE);
    }

    let test_path = cwd.join("tests").join("test_main.cpp");
    if !test_path.exists() {
        let test = super::new::generate_test(&name, &args.r#type);
        let _ = std::fs::write(test_path, test);
    }

    println!(
        "{} Beru project created. Run `beru build` to build your project.",
        style("Done:").green().bold()
    );

    Ok(())
}
