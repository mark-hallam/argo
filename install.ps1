# argo installer — Windows PowerShell
#
# Usage (PowerShell 5.1+, built into every Windows 10/11 install):
#   irm https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.ps1 | iex
#
# Honors:
#   $env:ARGO_VERSION       — pin to a specific tag (default: latest release)
#   $env:ARGO_INSTALL_DIR   — destination (default: %LOCALAPPDATA%\argo\bin)
#
# What it does:
#   1. Detects your Windows architecture (x64 only — arm64 falls through)
#   2. Downloads the matching .zip from the latest GitHub release
#   3. Extracts argo.exe to %LOCALAPPDATA%\argo\bin (user-scope; no admin needed)
#   4. Adds that dir to your USER PATH so `argo --version` Just Works in new shells
#   5. Tells you to run `argo setup` to continue
#
# Linux/macOS users: use install.sh instead (curl one-liner).

$ErrorActionPreference = 'Stop'

$Repo = 'mark-hallam/Argo-Agent'
$InstallDir = if ($env:ARGO_INSTALL_DIR) {
    $env:ARGO_INSTALL_DIR
} else {
    Join-Path $env:LOCALAPPDATA 'argo\bin'
}

# ---- detect architecture --------------------------------------------
# Use PROCESSOR_ARCHITECTURE so this works on PowerShell 5.1 (the
# default shell on Windows 10) without depending on .NET Core APIs.
# Values: AMD64 (64-bit Intel), ARM64, x86 (32-bit Intel — unsupported).

$arch = $env:PROCESSOR_ARCHITECTURE
$target = switch ($arch) {
    'AMD64' { 'x86_64-pc-windows-msvc' }
    default {
        Write-Host "argo: no pre-built binary for Windows/$arch yet."
        Write-Host "      build from source: https://github.com/$Repo#build-from-source"
        exit 1
    }
}

# ---- pick a release tag ---------------------------------------------

$tag = if ($env:ARGO_VERSION) {
    $env:ARGO_VERSION
} else {
    try {
        $latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
        $latest.tag_name
    } catch {
        Write-Host "argo: couldn't reach the GitHub API. Check your network or pin a version:"
        Write-Host '      $env:ARGO_VERSION = "v0.1.0"'
        Write-Host "      iwr https://raw.githubusercontent.com/$Repo/main/install.ps1 -UseBasicParsing | iex"
        exit 1
    }
}

if (-not $tag) {
    Write-Host "argo: no release found. The maintainer needs to run:"
    Write-Host '      git tag v0.1.0 && git push --tags'
    exit 1
}

$asset = "argo-$target.zip"
$url   = "https://github.com/$Repo/releases/download/$tag/$asset"

Write-Host "argo: installing $tag for $target"
Write-Host "      from $url"

# ---- download + extract ---------------------------------------------

$tmpRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("argo-install-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $tmpRoot -Force | Out-Null

try {
    $zipPath = Join-Path $tmpRoot $asset
    try {
        Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
    } catch {
        Write-Host "argo: download failed. Available assets at:"
        Write-Host "      https://github.com/$Repo/releases/tag/$tag"
        exit 1
    }

    Expand-Archive -Path $zipPath -DestinationPath $tmpRoot -Force
    $exe = Get-ChildItem -Path $tmpRoot -Filter 'argo.exe' -Recurse | Select-Object -First 1
    if (-not $exe) {
        Write-Host "argo: archive did not contain argo.exe"
        exit 1
    }

    # ---- install ----------------------------------------------------
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    $dest = Join-Path $InstallDir 'argo.exe'
    Move-Item -Path $exe.FullName -Destination $dest -Force

    # ---- add to user PATH if missing --------------------------------
    $userPath = [System.Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $userPath) { $userPath = '' }
    $pathParts = $userPath.Split(';') | Where-Object { $_ -ne '' }
    if ($pathParts -notcontains $InstallDir) {
        $newUserPath = if ($userPath) { "$userPath;$InstallDir" } else { $InstallDir }
        [System.Environment]::SetEnvironmentVariable('Path', $newUserPath, 'User')
        $PathUpdated = $true
    } else {
        $PathUpdated = $false
    }

    Write-Host ""
    Write-Host "argo $tag installed to $dest"
    Write-Host ""

    if ($PathUpdated) {
        Write-Host "Added $InstallDir to your USER PATH."
        Write-Host "Open a new PowerShell window (or restart your terminal) for the change to take effect."
    } else {
        Write-Host "$InstallDir is already on your PATH."
    }

    Write-Host ""
    Write-Host "Next: run ``argo setup`` to configure your workspace."
}
finally {
    Remove-Item -Path $tmpRoot -Recurse -Force -ErrorAction SilentlyContinue
}
