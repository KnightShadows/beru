$ErrorActionPreference = "Stop"

$Repo = "KnightShadows/Beru"
$BinName = "beru.exe"

Write-Host "Installing Beru for Windows..."

$ReleaseUrl = "https://api.github.com/repos/$Repo/releases/latest"
try {
    $Release = Invoke-RestMethod -Uri $ReleaseUrl
} catch {
    Write-Error "No releases found for $Repo or failed to fetch release data. You will need to build from source."
    exit 1
}

$Asset = $Release.assets | Where-Object { $_.name -like "*x86_64-pc-windows-msvc.zip" } | Select-Object -First 1

if ($null -eq $Asset) {
    Write-Error "Could not find a Windows binary in the latest release. Please build from source."
    exit 1
}

$DownloadUrl = $Asset.browser_download_url
$TempDir = Join-Path $env:TEMP (New-Guid).ToString()
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

$ZipPath = Join-Path $TempDir "beru.zip"

Write-Host "Downloading $DownloadUrl..."
Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath

Write-Host "Extracting..."
Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force

# Find beru.exe — it may be in a subdirectory inside the archive
$BinFile = Get-ChildItem -Path $TempDir -Filter $BinName -Recurse -File | Select-Object -First 1
if ($null -eq $BinFile) {
    Write-Error "Could not find $BinName in the downloaded archive."
    Remove-Item -Path $TempDir -Recurse -Force
    exit 1
}

$InstallDir = Join-Path $env:USERPROFILE ".cargo\bin"
if (-not (Test-Path $InstallDir)) {
    $InstallDir = Join-Path $env:USERPROFILE ".local\bin"
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

Write-Host "Installing to $InstallDir..."
Move-Item -Path $BinFile.FullName -Destination (Join-Path $InstallDir $BinName) -Force

Remove-Item -Path $TempDir -Recurse -Force

$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "Adding $InstallDir to your PATH..."
    $NewPath = if ($null -eq $UserPath -or $UserPath -eq "") { $InstallDir } else { "$UserPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    Write-Host "Installation complete! Please restart your PowerShell terminal to use Beru."
} else {
    Write-Host "Installation complete! $InstallDir is already in your PATH."
}
