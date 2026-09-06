#Requires -Version 5.0
# install.ps1 — One-command installer for rgaa-rs on Windows (x86_64)
#
# Usage:
#   irm https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/rgaa-rs/install.ps1 | iex
#   .\install.ps1 -Version latest        # bleeding edge (default)
#   .\install.ps1 -Version v0.1.0        # tagged release
#   .\install.ps1 -Uninstall             # remove installed files
param(
    [string]$Version = "latest",
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"
$Repo = "jamon8888/Holo-RGAA"
$ObscuraRepo = "h4ckf0r0day/obscura"
$ObscuraVersion = "0.2.2"
$InstallDir = "$env:LOCALAPPDATA\rgaa\bin"
$TmpDir = [System.IO.Path]::GetTempPath()
$McpConfig = "$env:USERPROFILE\.claude\mcp.json"

function Write-Step([string]$msg) {
    Write-Host ""
    Write-Host "==> $msg" -ForegroundColor Cyan
}

function Uninstall-Rgaa {
    Write-Step "Uninstalling rgaa-rs..."
    foreach ($bin in @("rgaa.exe", "rgaa-cli.exe", "rgaa-api.exe", "rgaa-mcp.exe", "obscura.exe", "obscura-worker.exe")) {
        $p = Join-Path $InstallDir $bin
        if (Test-Path $p) { Remove-Item $p -Force; Write-Host "  Removed $bin" }
    }
    Write-Host "  Uninstall complete." -ForegroundColor Green
}

if ($Uninstall) { Uninstall-Rgaa; exit 0 }

# Only x86_64 has prebuilt binaries (openssl-sys can't cross-compile ARM).
if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
    Write-Host "No prebuilt binaries for Windows ARM64. Build from source (WSL + install.sh --build)." -ForegroundColor Red
    exit 1
}

# Resolve download URLs. NOTE: our `latest` release is a prerelease, which
# /releases/latest skips — address the tag explicitly instead of the API.
$rgaaAsset = if ($Version -eq "latest") {
    "rgaa-rs-latest-x86_64-pc-windows-msvc.zip"
} else {
    "rgaa-rs-${Version}-x86_64-pc-windows-msvc.zip"
}
$rgaaUrl = "https://github.com/${Repo}/releases/download/${Version}/${rgaaAsset}"
$obscuraUrl = "https://github.com/${ObscuraRepo}/releases/download/v${ObscuraVersion}/obscura-x86_64-windows.zip"

Write-Host ""
Write-Host "  Platform: windows-x86_64"
Write-Host "  rgaa:     $rgaaUrl"
Write-Host "  obscura:  $obscuraUrl"
Write-Host ""

Write-Step "Downloading rgaa ($Version)..."
$outPath = Join-Path $TmpDir $rgaaAsset
try {
    Invoke-WebRequest -Uri $rgaaUrl -OutFile $outPath -UserAgent "rgaa-install"
} catch {
    Write-Host "Download failed. Binaries may not be uploaded yet for $Version." -ForegroundColor Red
    exit 1
}

Write-Step "Extracting..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Expand-Archive -Path $outPath -DestinationPath $InstallDir -Force
Remove-Item $outPath -Force

Write-Step "Downloading obscura $ObscuraVersion..."
$obscuraOut = Join-Path $TmpDir "obscura-x86_64-windows.zip"
try {
    Invoke-WebRequest -Uri $obscuraUrl -OutFile $obscuraOut -UserAgent "rgaa-install"
    $obscuraTmp = Join-Path $TmpDir "obscura-extract"
    New-Item -ItemType Directory -Force -Path $obscuraTmp | Out-Null
    Expand-Archive -Path $obscuraOut -DestinationPath $obscuraTmp -Force
    Copy-Item (Join-Path $obscuraTmp "obscura.exe") $InstallDir -Force
    Copy-Item (Join-Path $obscuraTmp "obscura-worker.exe") $InstallDir -Force
    Remove-Item $obscuraOut -Force
    Remove-Item $obscuraTmp -Recurse -Force
} catch {
    Write-Host "  WARNING: obscura download failed; browser automation unavailable." -ForegroundColor Yellow
}

# Verify versions
Write-Step "Verifying..."
& (Join-Path $InstallDir "rgaa.exe") --version
$obscuraBin = Join-Path $InstallDir "obscura.exe"
if (Test-Path $obscuraBin) {
    $ov = & $obscuraBin --version 2>$null
    if ($ov -notlike "*obscura $ObscuraVersion*") {
        Write-Host "  WARNING: obscura version mismatch: '$ov' (want obscura $ObscuraVersion)" -ForegroundColor Yellow
    } else {
        Write-Host "  obscura version: $ov"
    }
}

# MCP config for Claude Code
Write-Step "Configuring MCP server..."
$mcpDir = Split-Path $McpConfig
New-Item -ItemType Directory -Force -Path $mcpDir | Out-Null
$rgaaMcp = (Join-Path $InstallDir "rgaa-mcp.exe").Replace('\', '\\')
$obscuraPath = (Join-Path $InstallDir "obscura.exe").Replace('\', '\\')
$mcpJson = @"
{
  "mcpServers": {
    "rgaa-mcp": {
      "command": "$rgaaMcp",
      "env": {
        "RGAA_OBSCURA_BIN": "$obscuraPath"
      }
    }
  }
}
"@
if (Test-Path $McpConfig) {
    Write-Host "  MCP config exists at $McpConfig — merge the rgaa-mcp entry manually."
} else {
    Set-Content -Path $McpConfig -Value $mcpJson -Encoding UTF8
    Write-Host "  MCP config written to $McpConfig"
}

if ($env:Path -notlike "*$InstallDir*") {
    Write-Host ""
    Write-Step "Adding $InstallDir to PATH..."
    [System.Environment]::SetEnvironmentVariable("Path", "$InstallDir;$env:Path", "User")
    $env:Path = "$InstallDir;$env:Path"
}

Write-Host ""
Write-Host "  Installed! Restart your terminal, then run: rgaa" -ForegroundColor Green
