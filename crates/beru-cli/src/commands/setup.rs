use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::io::{self, IsTerminal, Write};
use std::process::Command;

/// Arguments for `beru setup`.
#[derive(Debug, Args)]
pub struct SetupArgs {
    /// Only check prerequisites and exit with code 0 (success) or 1 (failure)
    #[arg(long)]
    pub check: bool,

    /// Automatically install missing prerequisites without prompting
    #[arg(long = "install-prereqs")]
    pub install_prereqs: bool,

    /// Do not attempt to install missing prerequisites
    #[arg(long = "no-install")]
    pub no_install: bool,

    /// Automatically accept all installation prompts
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,

    /// Add ~/.beru/bin to user PATH environment variable
    #[arg(long = "add-to-path")]
    pub add_to_path: bool,
}

#[derive(Debug, Clone)]
struct ToolCheck {
    name: &'static str,
    required: bool,
    found: bool,
    details: String,
}

fn prompt_confirm(prompt: &str, default_yes: bool) -> bool {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("{} {} ", prompt, suffix);
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return default_yes;
    }

    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() {
        return default_yes;
    }

    trimmed == "y" || trimmed == "yes"
}

/// Execute the `beru setup` command.
pub fn exec(args: SetupArgs) -> Result<()> {
    if args.add_to_path {
        add_beru_to_path()?;
        println!(
            "{} Successfully configured Beru bin directory in user PATH.",
            style("Success:").green().bold()
        );
        return Ok(());
    }

    println!(
        "\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        style(" ██████╗ ███████╗██████╗ ██╗   ██╗").cyan().bold(),
        style(" ██╔══██╗██╔════╝██╔══██╗██║   ██║").cyan().bold(),
        style(" ██████╔╝█████╗  ██████╔╝██║   ██║").cyan().bold(),
        style(" ██╔══██╗██╔══╝  ██╔══██╗██║   ██║").cyan().bold(),
        style(" ██████╔╝███████╗██║  ██║╚██████╔╝").cyan().bold(),
        style(" ╚═════╝ ╚══════╝╚═╝  ╚═╝ ╚═════╝ ").cyan().bold(),
        style(" [ 01100010 01100101 01110010 01110101 ]").dim(),
    );

    println!(
        "\n{}",
        style("=== Beru Environment & Toolchain Setup ===")
            .cyan()
            .bold()
    );

    let git_check = check_git();
    let cmake_check = check_cmake();
    let compiler_check = check_compiler();
    let ninja_check = check_ninja();
    let path_check = check_path();

    let checks = vec![
        git_check,
        cmake_check,
        compiler_check,
        ninja_check,
        path_check,
    ];

    println!();
    for check in &checks {
        let status_str = if check.found {
            style("[OK]").green().bold()
        } else if check.required {
            style("[FAIL]").red().bold()
        } else {
            style("[WARN]").yellow().bold()
        };

        println!(
            "  {:<15} {:<8} {}",
            style(check.name).bold(),
            status_str,
            check.details
        );
    }
    println!();

    let missing_required: Vec<&ToolCheck> =
        checks.iter().filter(|c| c.required && !c.found).collect();
    let missing_optional: Vec<&ToolCheck> =
        checks.iter().filter(|c| !c.required && !c.found).collect();

    if !missing_required.is_empty() {
        println!(
            "{} Some required prerequisites are missing or outdated.",
            style("Notice:").red().bold()
        );
        print_remediation(&missing_required);

        if args.check {
            bail!("Prerequisites check failed: missing required C++ tools");
        }

        let should_install = if args.no_install {
            false
        } else if args.install_prereqs || args.yes {
            true
        } else if io::stdin().is_terminal() {
            prompt_confirm(
                "Would you like Beru to automatically download and install the missing prerequisites?",
                true,
            )
        } else {
            false
        };

        if should_install {
            println!(
                "\n{} Downloading and installing missing prerequisites...\n",
                style("Setting up:").cyan().bold()
            );
            run_prereq_install(&checks)?;

            println!(
                "\n{} Re-evaluating environment after installation...\n",
                style("Checking:").cyan().bold()
            );

            let rechecks = vec![
                check_git(),
                check_cmake(),
                check_compiler(),
                check_ninja(),
                check_path(),
            ];

            for check in &rechecks {
                let status_str = if check.found {
                    style("[OK]").green().bold()
                } else if check.required {
                    style("[FAIL]").red().bold()
                } else {
                    style("[WARN]").yellow().bold()
                };

                println!(
                    "  {:<15} {:<8} {}",
                    style(check.name).bold(),
                    status_str,
                    check.details
                );
            }
            println!();

            let still_missing: Vec<&ToolCheck> =
                rechecks.iter().filter(|c| c.required && !c.found).collect();
            if still_missing.is_empty() {
                println!(
                    "{} All required prerequisites are met! You are ready to build with Beru.\n",
                    style("Success:").green().bold()
                );
            } else {
                println!(
                    "{} Some tools could not be automatically installed. Please review the manual steps above.\n",
                    style("Notice:").yellow().bold()
                );
            }
        } else if !args.no_install {
            println!(
                "  -> Run {} to automatically download and install missing prerequisites.\n",
                style("beru setup --install-prereqs").cyan().bold()
            );
        }
        return Ok(());
    }

    if !missing_optional.is_empty() {
        println!(
            "{} All required tools are present, but some recommended tools are missing.",
            style("Notice:").yellow().bold()
        );
        for item in &missing_optional {
            if item.name == "PATH" {
                println!(
                    "  -> Run {} to add ~/.beru/bin to your PATH.",
                    style("beru setup --add-to-path").cyan()
                );
            } else if item.name == "ninja" {
                println!("  -> Installing Ninja is recommended for optimal build speeds.");
            }
        }
        println!();

        if (args.install_prereqs || args.yes) && !args.no_install && !args.check {
            run_prereq_install(&checks)?;
        }
    } else {
        println!(
            "{} All prerequisites are met! You are ready to build with Beru.\n",
            style("Success:").green().bold()
        );
    }

    Ok(())
}

fn check_git() -> ToolCheck {
    match which::which("git") {
        Ok(path) => {
            let output = Command::new(&path).arg("--version").output();
            let details = match output {
                Ok(out) if out.status.success() => {
                    String::from_utf8_lossy(&out.stdout).trim().to_string()
                }
                _ => format!("Found at {}", path.display()),
            };
            ToolCheck {
                name: "git",
                required: true,
                found: true,
                details,
            }
        }
        Err(_) => ToolCheck {
            name: "git",
            required: true,
            found: false,
            details: "Not found on PATH (required for package index and git dependencies)"
                .to_string(),
        },
    }
}

fn check_cmake() -> ToolCheck {
    match which::which("cmake") {
        Ok(path) => {
            let output = Command::new(&path).arg("--version").output();
            if let Ok(out) = output
                && out.status.success()
            {
                let text = String::from_utf8_lossy(&out.stdout);
                if let Some((major, minor)) = parse_cmake_version(&text) {
                    let is_supported = major > 3 || (major == 3 && minor >= 20);
                    let first_line = text.lines().next().unwrap_or("").trim();
                    if is_supported {
                        return ToolCheck {
                            name: "cmake",
                            required: true,
                            found: true,
                            details: format!("{first_line} (>= 3.20 supported)"),
                        };
                    } else {
                        return ToolCheck {
                            name: "cmake",
                            required: true,
                            found: false,
                            details: format!(
                                "{first_line} - OUTDATED (Beru requires CMake >= 3.20)"
                            ),
                        };
                    }
                }
            }
            ToolCheck {
                name: "cmake",
                required: true,
                found: true,
                details: format!("Found at {}", path.display()),
            }
        }
        Err(_) => ToolCheck {
            name: "cmake",
            required: true,
            found: false,
            details: "Not found on PATH (required for C++ build orchestration)".to_string(),
        },
    }
}

fn parse_cmake_version(output: &str) -> Option<(u64, u64)> {
    for line in output.lines() {
        if line.contains("cmake version") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for part in parts {
                let segments: Vec<&str> = part.split('.').collect();
                if segments.len() >= 2
                    && let (Ok(major), Ok(minor)) =
                        (segments[0].parse::<u64>(), segments[1].parse::<u64>())
                {
                    return Some((major, minor));
                }
            }
        }
    }
    None
}

fn check_compiler() -> ToolCheck {
    // Probe compilers in order: cl (MSVC), clang++, g++, c++
    let candidates = if cfg!(windows) {
        vec!["cl", "clang++", "g++"]
    } else {
        vec!["clang++", "g++", "c++"]
    };

    for candidate in candidates {
        if let Ok(path) = which::which(candidate) {
            let details = if candidate == "cl" {
                // cl /? exits with banner in stderr/stdout
                let out = Command::new(&path).arg("/?").output();
                match out {
                    Ok(o) => {
                        let text = String::from_utf8_lossy(&o.stdout);
                        let first = text.lines().next().unwrap_or("Microsoft C/C++ Compiler");
                        first.trim().to_string()
                    }
                    _ => format!("Microsoft (R) C/C++ Compiler ({})", path.display()),
                }
            } else {
                let out = Command::new(&path).arg("--version").output();
                match out {
                    Ok(o) if o.status.success() => {
                        let text = String::from_utf8_lossy(&o.stdout);
                        text.lines().next().unwrap_or("").trim().to_string()
                    }
                    _ => format!("Found at {}", path.display()),
                }
            };

            return ToolCheck {
                name: "C++ compiler",
                required: true,
                found: true,
                details,
            };
        }
    }

    ToolCheck {
        name: "C++ compiler",
        required: true,
        found: false,
        details: "No C++ compiler found (need g++, clang++, or MSVC cl.exe)".to_string(),
    }
}

fn check_ninja() -> ToolCheck {
    match which::which("ninja") {
        Ok(path) => {
            let output = Command::new(&path).arg("--version").output();
            let details = match output {
                Ok(out) if out.status.success() => {
                    format!("ninja {}", String::from_utf8_lossy(&out.stdout).trim())
                }
                _ => format!("Found at {}", path.display()),
            };
            ToolCheck {
                name: "ninja",
                required: false,
                found: true,
                details,
            }
        }
        Err(_) => ToolCheck {
            name: "ninja",
            required: false,
            found: false,
            details: "Not found (recommended for ultra-fast builds)".to_string(),
        },
    }
}

fn check_path() -> ToolCheck {
    let cache = match beru_core::cache::BeruCache::default_location() {
        Ok(c) => c,
        Err(_) => {
            return ToolCheck {
                name: "PATH",
                required: false,
                found: false,
                details: "Could not determine ~/.beru location".to_string(),
            };
        }
    };

    let bin_dir = cache.bin_dir();
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let paths = std::env::split_paths(&path_var);

    for p in paths {
        if p == bin_dir {
            return ToolCheck {
                name: "PATH",
                required: false,
                found: true,
                details: format!("{} is in PATH", bin_dir.display()),
            };
        }
    }

    ToolCheck {
        name: "PATH",
        required: false,
        found: false,
        details: format!("{} is NOT in PATH", bin_dir.display()),
    }
}

fn print_remediation(_missing: &[&ToolCheck]) {
    println!("\nTo install missing prerequisites on your operating system:\n");

    #[cfg(target_os = "linux")]
    {
        if which::which("apt-get").is_ok() {
            println!("  Debian/Ubuntu:");
            println!(
                "    sudo apt-get update && sudo apt-get install -y build-essential cmake git ninja-build"
            );
        } else if which::which("dnf").is_ok() {
            println!("  Fedora/RHEL:");
            println!("    sudo dnf install -y gcc-c++ cmake git ninja-build");
        } else if which::which("pacman").is_ok() {
            println!("  Arch Linux:");
            println!("    sudo pacman -S --noconfirm base-devel cmake git ninja");
        } else {
            println!(
                "  Please install git, cmake (>= 3.20), a C++ compiler (g++/clang++), and ninja via your package manager."
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        println!("  macOS (Homebrew):");
        println!("    brew install cmake ninja git gcc");
    }

    #[cfg(target_os = "windows")]
    {
        println!("  Windows (winget):");
        println!("    winget install --id Kitware.CMake -e");
        println!("    winget install --id Ninja-build.Ninja -e");
        println!("    winget install --id Git.Git -e");
        println!();
        println!("  For Visual Studio C++ Compiler (MSVC):");
        println!(
            "    winget install Microsoft.VisualStudio.2022.BuildTools --override \"--passive --add Microsoft.VisualStudio.Workload.VCTools\""
        );
    }

    println!();
}

fn run_prereq_install(checks: &[ToolCheck]) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        if which::which("winget").is_err() {
            println!(
                "{} 'winget' was not found on PATH. Please install the App Installer from the Microsoft Store or install tools manually using the commands above.",
                style("Notice:").yellow().bold()
            );
            return Ok(());
        }

        println!(
            "{} Automated installation via Windows Package Manager (winget)...",
            style("Progress:").cyan().bold()
        );

        for check in checks {
            if check.found {
                continue;
            }

            match check.name {
                "git" => {
                    println!("  {} Git for Windows...", style("Installing").cyan().bold());
                    let mut cmd = Command::new("winget");
                    cmd.args([
                        "install",
                        "--id",
                        "Git.Git",
                        "-e",
                        "--accept-package-agreements",
                        "--accept-source-agreements",
                    ]);
                    let _ = cmd.status();
                }
                "cmake" => {
                    println!("  {} Kitware CMake...", style("Installing").cyan().bold());
                    let mut cmd = Command::new("winget");
                    cmd.args([
                        "install",
                        "--id",
                        "Kitware.CMake",
                        "-e",
                        "--accept-package-agreements",
                        "--accept-source-agreements",
                    ]);
                    let _ = cmd.status();
                }
                "ninja" => {
                    println!(
                        "  {} Ninja Build System...",
                        style("Installing").cyan().bold()
                    );
                    let mut cmd = Command::new("winget");
                    cmd.args([
                        "install",
                        "--id",
                        "Ninja-build.Ninja",
                        "-e",
                        "--accept-package-agreements",
                        "--accept-source-agreements",
                    ]);
                    let _ = cmd.status();
                }
                "C++ compiler" => {
                    println!(
                        "  {} LLVM Clang C++ Compiler...",
                        style("Installing").cyan().bold()
                    );
                    let mut cmd = Command::new("winget");
                    cmd.args([
                        "install",
                        "--id",
                        "LLVM.LLVM",
                        "-e",
                        "--accept-package-agreements",
                        "--accept-source-agreements",
                    ]);
                    let status = cmd.status();
                    if status.is_err() || !status.as_ref().map(|s| s.success()).unwrap_or(false) {
                        println!(
                            "  {} Alternatively, install Visual Studio C++ Build Tools with:\n    winget install Microsoft.VisualStudio.2022.BuildTools --override \"--passive --add Microsoft.VisualStudio.Workload.VCTools\"",
                            style("Note:").yellow().bold()
                        );
                    }
                }
                "PATH" => {
                    println!(
                        "  {} User PATH environment variable...",
                        style("Configuring").cyan().bold()
                    );
                    let _ = add_beru_to_path();
                }
                _ => {}
            }
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        let has_xcode_tools = Command::new("xcode-select")
            .arg("-p")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !has_xcode_tools {
            println!(
                "  {} Xcode Command Line Tools...",
                style("Triggering").cyan().bold()
            );
            let _ = Command::new("xcode-select").arg("--install").status();
        }

        if which::which("brew").is_ok() {
            let mut pkgs = Vec::new();
            for check in checks {
                if check.found {
                    continue;
                }
                match check.name {
                    "git" => pkgs.push("git"),
                    "cmake" => pkgs.push("cmake"),
                    "ninja" => pkgs.push("ninja"),
                    "C++ compiler" if !has_xcode_tools => pkgs.push("gcc"),
                    "PATH" => {
                        let _ = add_beru_to_path();
                    }
                    _ => {}
                }
            }

            if !pkgs.is_empty() {
                println!(
                    "  {} missing packages via Homebrew ({})...",
                    style("Installing").cyan().bold(),
                    pkgs.join(", ")
                );
                let _ = Command::new("brew").arg("install").args(&pkgs).status();
            }
        } else {
            println!(
                "{} Homebrew ('brew') not found. Please install Homebrew from https://brew.sh",
                style("Notice:").yellow().bold()
            );
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        println!(
            "{} Automated package manager installation...",
            style("Progress:").cyan().bold()
        );

        let is_root = std::env::var("USER").map(|u| u == "root").unwrap_or(false);

        if which::which("apt-get").is_ok() {
            println!(
                "  {} packages via apt-get...",
                style("Installing").cyan().bold()
            );
            let script = if is_root {
                "apt-get update && apt-get install -y build-essential cmake git ninja-build clang-format"
            } else {
                "sudo apt-get update && sudo apt-get install -y build-essential cmake git ninja-build clang-format"
            };
            let _ = Command::new("sh").args(["-c", script]).status();
        } else if which::which("dnf").is_ok() {
            println!(
                "  {} packages via dnf...",
                style("Installing").cyan().bold()
            );
            let script = if is_root {
                "dnf install -y gcc-c++ cmake git ninja-build clang-tools-extra"
            } else {
                "sudo dnf install -y gcc-c++ cmake git ninja-build clang-tools-extra"
            };
            let _ = Command::new("sh").args(["-c", script]).status();
        } else if which::which("pacman").is_ok() {
            println!(
                "  {} packages via pacman...",
                style("Installing").cyan().bold()
            );
            let script = if is_root {
                "pacman -S --noconfirm base-devel cmake git ninja clang"
            } else {
                "sudo pacman -S --noconfirm base-devel cmake git ninja clang"
            };
            let _ = Command::new("sh").args(["-c", script]).status();
        } else {
            println!(
                "{} Supported package manager not detected (apt-get, dnf, pacman). Please install prerequisites manually.",
                style("Notice:").yellow().bold()
            );
        }

        for check in checks {
            if check.name == "PATH" && !check.found {
                let _ = add_beru_to_path();
            }
        }

        Ok(())
    }
}

fn add_beru_to_path() -> Result<()> {
    let cache = beru_core::cache::BeruCache::default_location()?;
    cache.ensure_dirs()?;
    let bin_dir = cache.bin_dir();

    #[cfg(windows)]
    {
        let bin_str = bin_dir.to_string_lossy().to_string();
        let script = format!(
            "$current = [Environment]::GetEnvironmentVariable('PATH', 'User'); \
             if ($current -notlike '*{0}*') {{ \
                 [Environment]::SetEnvironmentVariable('PATH', \"$current;{0}\", 'User'); \
             }}",
            bin_str
        );

        let status = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status()
            .context("failed to update user PATH via PowerShell")?;

        if !status.success() {
            bail!("PowerShell PATH update command failed");
        }
        Ok(())
    }

    #[cfg(unix)]
    {
        let home = dirs::home_dir().context("could not locate home directory")?;
        let bin_str = bin_dir.to_string_lossy();
        let export_line = format!("\n# Added by Beru\nexport PATH=\"{}:$PATH\"\n", bin_str);

        let candidate_rcs = [
            home.join(".bashrc"),
            home.join(".zshrc"),
            home.join(".profile"),
        ];

        let mut updated_any = false;
        for rc in &candidate_rcs {
            if rc.exists() {
                let content = std::fs::read_to_string(rc).unwrap_or_default();
                if !content.contains(&*bin_str) {
                    use std::io::Write;
                    let mut file = std::fs::OpenOptions::new().append(true).open(rc)?;
                    file.write_all(export_line.as_bytes())?;
                    updated_any = true;
                }
            }
        }

        if !updated_any {
            // Default to writing ~/.profile
            let profile = home.join(".profile");
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(profile)?;
            file.write_all(export_line.as_bytes())?;
        }

        Ok(())
    }
}
