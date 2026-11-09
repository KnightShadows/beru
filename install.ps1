<#
.SYNOPSIS
    Installs the Beru C++ Package Manager on Windows.
.DESCRIPTION
    Downloads the latest pre-compiled Beru binary, installs it to $HOME\.beru\bin,
    configures the User PATH environment variable, and runs `beru setup` to inspect
    and configure toolchain prerequisites.
#>

$ErrorActionPreference = "Stop"

$Repo = "KnightShadows/beru"
$BinName = "beru.exe"
$InstallDir = Join-Path $HOME ".beru\bin"

Write-Host "Installing Beru C++ Package Manager..." -ForegroundColor Cyan

# 1. Architecture Check
if (-not [Environment]::Is64BitOperatingSystem) {
    Write-Error "Error: 32-bit Windows is not supported. Beru requires 64-bit Windows."
    exit 1
}

$Target = "x86_64-pc-windows-msvc"

# 2. Fetch Release Info
Write-Host "Fetching latest release data for $Target..." -ForegroundColor Gray
$ReleaseUrl = "https://api.github.com/repos/$Repo/releases/latest"
try {
    $Release = Invoke-RestMethod -Uri $ReleaseUrl -Headers @{ "User-Agent" = "beru-installer" }
} catch {
    Write-Warning "Could not fetch latest release: $_"
    Write-Warning "If no pre-built release is available yet, please build from source via 'cargo install --path crates/beru-cli'."
    exit 1
}

# 3. Locate Target Asset
$Asset = $Release.assets | Where-Object { $_.name -like "*$Target*" } | Select-Object -First 1
if (-not $Asset) {
    Write-Error "Error: No release asset found matching target $Target in release $($Release.tag_name)."
    exit 1
}

$DownloadUrl = $Asset.browser_download_url
$TempDir = Join-Path $env:TEMP ("beru-install-" + [Guid]::NewGuid().ToString().Substring(0, 8))
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
$ArchiveFile = Join-Path $TempDir $Asset.name

try {
    Write-Host "Downloading $($Asset.name)..." -ForegroundColor Gray
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ArchiveFile -UseBasicParsing

    Write-Host "Extracting..." -ForegroundColor Gray
    if ($Asset.name.EndsWith(".zip")) {
        Expand-Archive -Path $ArchiveFile -DestinationPath $TempDir -Force
    } elseif ($Asset.name.EndsWith(".tar.gz") -or $Asset.name.EndsWith(".tgz")) {
        tar -xzf $ArchiveFile -C $TempDir
    } else {
        Copy-Item -Path $ArchiveFile -Destination (Join-Path $TempDir $BinName) -Force
    }

    $ExtractedExe = Get-ChildItem -Path $TempDir -Recurse -Filter $BinName | Select-Object -First 1
    if (-not $ExtractedExe) {
        Write-Error "Error: Failed to find $BinName in downloaded release archive."
        exit 1
    }

    # 4. Install into $HOME\.beru\bin
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    $DestExe = Join-Path $InstallDir $BinName
    Copy-Item -Path $ExtractedExe.FullName -Destination $DestExe -Force
    Write-Host "Installed Beru to $DestExe" -ForegroundColor Green

    # 5. Configure User PATH
    $UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if (-not $UserPath) {
        $UserPath = ""
    }
    $PathEntries = $UserPath -split ";"
    if ($PathEntries -notcontains $InstallDir) {
        Write-Host "Adding $InstallDir to user PATH..." -ForegroundColor Cyan
        $NewPath = if ($UserPath.Length -gt 0 -and -not $UserPath.EndsWith(";")) { "$UserPath;$InstallDir" } else { "$UserPath$InstallDir" }
        [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
        $env:PATH = "$env:PATH;$InstallDir"
    } else {
        Write-Host "$InstallDir is already in user PATH." -ForegroundColor Gray
    }

    # 6. Run beru setup to verify and automatically configure prerequisites
    Write-Host "`nRunning initial environment and prerequisite setup..." -ForegroundColor Cyan
    & $DestExe setup --install-prereqs

} finally {
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "`nInstallation successful! Please restart your terminal to ensure PATH updates take effect." -ForegroundColor Green
