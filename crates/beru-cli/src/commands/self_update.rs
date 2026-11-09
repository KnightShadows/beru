use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use serde::Deserialize;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Arguments for `beru self-update`.
#[derive(Debug, Args)]
pub struct SelfUpdateArgs {
    /// Check for available updates without installing
    #[arg(long)]
    pub check: bool,

    /// Specific version to install (e.g. "0.5.7")
    #[arg(long)]
    pub version: Option<String>,

    /// Force update even if already up to date
    #[arg(long)]
    pub force: bool,

    /// GitHub repository in owner/repo format
    #[arg(long, default_value = "KnightShadows/beru")]
    pub repo: String,
}

#[derive(Deserialize, Debug)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize, Debug)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Execute the `beru self-update` command.
pub fn exec(args: SelfUpdateArgs) -> Result<()> {
    let current_exe = if let Ok(custom_target) = std::env::var("BERU_SELF_UPDATE_TARGET") {
        PathBuf::from(custom_target)
    } else {
        let exe = std::env::current_exe().context("failed to determine current executable path")?;
        std::fs::canonicalize(&exe).unwrap_or(exe)
    };

    // Clean up any leftover .old binary from a previous update
    cleanup_old_binary(&current_exe);

    let current_version = env!("CARGO_PKG_VERSION");
    let target = detect_target()?;

    // Test hook / mirror: override download URL directly
    if let Ok(override_url) = std::env::var("BERU_SELF_UPDATE_URL") {
        println!(
            "{} Using override self-update source: {}",
            style("Info:").cyan().bold(),
            override_url
        );
        let bytes = fetch_override_bytes(&override_url)?;
        let temp_dir = tempfile::tempdir().context("failed to create temporary directory")?;
        let extracted = extract_binary_from_bytes(&bytes, &override_url, temp_dir.path())?;
        replace_current_binary(&current_exe, &extracted)?;
        println!(
            "{} Beru successfully self-updated from override source!",
            style("Success:").green().bold()
        );
        return Ok(());
    }

    if std::env::var("BERU_OFFLINE").is_ok() {
        bail!("Cannot perform self-update in offline mode");
    }

    let repo = std::env::var("BERU_UPDATE_REPO").unwrap_or(args.repo);

    println!(
        "{} Checking for updates for {} ({target})...",
        style("Info:").cyan().bold(),
        repo
    );

    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("beru/{}", current_version))
        .build()
        .context("failed to build HTTP client")?;

    let api_url = if let Some(ref ver) = args.version {
        let tag = if ver.starts_with('v') {
            ver.clone()
        } else {
            format!("v{ver}")
        };
        format!("https://api.github.com/repos/{repo}/releases/tags/{tag}")
    } else {
        format!("https://api.github.com/repos/{repo}/releases/latest")
    };

    let resp = client
        .get(&api_url)
        .send()
        .with_context(|| format!("failed to query GitHub API at {api_url}"))?;

    if !resp.status().is_success() {
        bail!(
            "GitHub API request failed with status {}: {}",
            resp.status(),
            resp.text().unwrap_or_default()
        );
    }

    let release_body = resp.text().context("failed to read GitHub API response")?;
    let release: GitHubRelease =
        serde_json::from_str(&release_body).context("failed to parse GitHub release JSON")?;

    let target_version = release.tag_name.trim_start_matches('v');

    println!("  Current version: v{}", current_version);
    println!("  Available version: v{}", target_version);

    let is_same_version = current_version == target_version;

    if args.check {
        if is_same_version {
            println!(
                "{} Beru is already up to date (v{}).",
                style("Up to date:").green().bold(),
                current_version
            );
        } else {
            println!(
                "{} A new version is available: v{} -> v{}. Run 'beru self-update' to install.",
                style("Update available:").yellow().bold(),
                current_version,
                target_version
            );
        }
        return Ok(());
    }

    if is_same_version && !args.force && args.version.is_none() {
        println!(
            "{} Beru is already up to date (v{}). Use --force to reinstall.",
            style("Notice:").green().bold(),
            current_version
        );
        return Ok(());
    }

    // Find suitable asset for target
    let asset = find_matching_asset(&release.assets, target).with_context(|| {
        format!(
            "no release asset found matching target '{target}'. Available assets: {:?}",
            release.assets.iter().map(|a| &a.name).collect::<Vec<_>>()
        )
    })?;

    println!(
        "{} Downloading {} from {}...",
        style("Downloading").green().bold(),
        asset.name,
        asset.browser_download_url
    );

    let download_resp = client
        .get(&asset.browser_download_url)
        .send()
        .with_context(|| {
            format!(
                "failed to download asset from {}",
                asset.browser_download_url
            )
        })?;

    if !download_resp.status().is_success() {
        bail!(
            "Failed to download asset: HTTP status {}",
            download_resp.status()
        );
    }

    let bytes = download_resp
        .bytes()
        .context("failed to read asset download bytes")?;

    let temp_dir =
        tempfile::tempdir().context("failed to create temporary extraction directory")?;
    let extracted_exe = extract_binary_from_bytes(&bytes, &asset.name, temp_dir.path())?;

    println!(
        "{} Installing into {}...",
        style("Installing").green().bold(),
        current_exe.display()
    );

    replace_current_binary(&current_exe, &extracted_exe)?;

    println!(
        "{} Beru updated successfully to v{}!",
        style("Success:").green().bold(),
        target_version
    );

    Ok(())
}

fn detect_target() -> Result<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Ok("x86_64-unknown-linux-musl")
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        Ok("aarch64-unknown-linux-musl")
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        Ok("x86_64-apple-darwin")
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Ok("aarch64-apple-darwin")
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        Ok("x86_64-pc-windows-msvc")
    }
    #[cfg(not(any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(
            target_os = "macos",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    {
        bail!("unsupported platform architecture for pre-built binaries")
    }
}

fn find_matching_asset<'a>(assets: &'a [GitHubAsset], target: &str) -> Option<&'a GitHubAsset> {
    // Exact match target
    if let Some(asset) = assets.iter().find(|a| a.name.contains(target)) {
        return Some(asset);
    }

    // Fallback on Linux to gnu if musl not present
    if target.contains("unknown-linux-musl") {
        let gnu_target = target.replace("musl", "gnu");
        if let Some(asset) = assets.iter().find(|a| a.name.contains(&gnu_target)) {
            return Some(asset);
        }
    }

    None
}

fn fetch_override_bytes(url: &str) -> Result<Vec<u8>> {
    if let Some(file_path) = url.strip_prefix("file://") {
        let path =
            if cfg!(windows) && file_path.starts_with('/') && file_path.chars().nth(2) == Some(':')
            {
                &file_path[1..]
            } else {
                file_path
            };
        std::fs::read(path).with_context(|| format!("failed to read local override file {path}"))
    } else if Path::new(url).exists() {
        std::fs::read(url).with_context(|| format!("failed to read local override file {url}"))
    } else {
        let client = reqwest::blocking::Client::new();
        let resp = client
            .get(url)
            .send()
            .with_context(|| format!("failed to fetch override URL {url}"))?;
        let bytes = resp.bytes()?.to_vec();
        Ok(bytes)
    }
}

fn extract_binary_from_bytes(bytes: &[u8], asset_name: &str, dest_dir: &Path) -> Result<PathBuf> {
    let binary_name = if cfg!(windows) { "beru.exe" } else { "beru" };

    if asset_name.ends_with(".tar.gz") || asset_name.ends_with(".tgz") {
        let gz = flate2::read::GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(gz);

        for entry in archive.entries().context("invalid tar archive")? {
            let mut entry = entry.context("invalid tar entry")?;
            let path = entry.path().context("invalid entry path")?.to_path_buf();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if file_name == binary_name || file_name == "beru" {
                let out_path = dest_dir.join(binary_name);
                let mut out_file = File::create(&out_path)?;
                std::io::copy(&mut entry, &mut out_file)?;
                return Ok(out_path);
            }
        }
        bail!("Could not find '{}' binary inside tar archive", binary_name);
    } else if asset_name.ends_with(".zip") {
        // Write zip archive to temporary location
        let zip_path = dest_dir.join("download.zip");
        std::fs::write(&zip_path, bytes)?;

        #[cfg(windows)]
        {
            let extract_script = format!(
                "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                zip_path.display(),
                dest_dir.display()
            );
            let status = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &extract_script])
                .status()
                .context("failed to invoke PowerShell Expand-Archive")?;

            if !status.success() {
                bail!("Failed to extract zip archive via PowerShell");
            }

            let candidate = dest_dir.join(binary_name);
            if candidate.exists() {
                return Ok(candidate);
            }

            // Search nested directories
            for entry in walkdir(dest_dir)? {
                if entry.file_name() == Some(std::ffi::OsStr::new(binary_name)) {
                    return Ok(entry);
                }
            }
            bail!(
                "Could not find '{}' inside extracted zip archive",
                binary_name
            );
        }

        #[cfg(not(windows))]
        {
            let status = std::process::Command::new("unzip")
                .arg("-q")
                .arg(&zip_path)
                .arg("-d")
                .arg(dest_dir)
                .status()
                .context("failed to execute unzip command")?;

            if !status.success() {
                bail!("Failed to extract zip archive");
            }

            let candidate = dest_dir.join(binary_name);
            if candidate.exists() {
                return Ok(candidate);
            }
            bail!("Could not find '{}' inside extracted zip", binary_name);
        }
    } else {
        // Raw executable binary
        let out_path = dest_dir.join(binary_name);
        std::fs::write(&out_path, bytes)?;
        Ok(out_path)
    }
}

#[cfg(windows)]
fn walkdir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if let Ok(nested) = walkdir(&p) {
                    results.extend(nested);
                }
            } else {
                results.push(p);
            }
        }
    }
    Ok(results)
}

fn replace_current_binary(current_exe: &Path, new_binary: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        let old_exe = current_exe.with_extension("old");
        if old_exe.exists() {
            let _ = std::fs::remove_file(&old_exe);
        }

        // On Windows, running exe cannot be directly overwritten, but it CAN be renamed!
        std::fs::rename(current_exe, &old_exe).with_context(|| {
            format!(
                "failed to rename currently running executable '{}' to '{}'",
                current_exe.display(),
                old_exe.display()
            )
        })?;

        if let Err(e) = std::fs::copy(new_binary, current_exe) {
            // Attempt rollback if copy fails
            let _ = std::fs::rename(&old_exe, current_exe);
            return Err(e).context("failed to copy new binary into place");
        }

        // Attempt immediate cleanup of .old binary (will succeed if OS permits or next invocation will clean it)
        let _ = std::fs::remove_file(&old_exe);
        Ok(())
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let tmp_dest = current_exe.with_extension("tmp");
        std::fs::copy(new_binary, &tmp_dest).with_context(|| {
            format!(
                "failed to copy new binary to temporary destination '{}'",
                tmp_dest.display()
            )
        })?;

        let mut perms = std::fs::metadata(&tmp_dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp_dest, perms)?;

        std::fs::rename(&tmp_dest, current_exe).with_context(|| {
            format!(
                "failed to atomically replace current binary '{}'",
                current_exe.display()
            )
        })?;

        Ok(())
    }
}

fn cleanup_old_binary(current_exe: &Path) {
    #[cfg(windows)]
    {
        let old_exe = current_exe.with_extension("old");
        if old_exe.exists() {
            let _ = std::fs::remove_file(old_exe);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = current_exe;
    }
}
