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

if ($env:RGAA_RELEASE_URL_BASE) {
    Write-Host "  WARNING: RGAA_RELEASE_URL_BASE override active (test-only)" -ForegroundColor Yellow
    $rgaaUrl = "$env:RGAA_RELEASE_URL_BASE/rgaa-rs-${Version}-x86_64-pc-windows-msvc.zip"
}

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

# Claude Code plugin
Write-Step "Installing Claude Code plugin..."
$PluginDir = "$env:USERPROFILE\.claude\plugins\rgaa-audit"
try {
    $pluginTmp = Join-Path $TmpDir "rgaa-plugin-fetch"
    New-Item -ItemType Directory -Force -Path $pluginTmp | Out-Null
    $pluginTarball = Join-Path $pluginTmp "repo.tar.gz"
    Invoke-WebRequest -Uri "https://codeload.github.com/${Repo}/tar.gz/${Version}" -OutFile $pluginTarball -UserAgent "rgaa-install"
    tar -xzf $pluginTarball -C $pluginTmp
    $repoRoot = Get-ChildItem -Path $pluginTmp -Directory | Where-Object { $_.Name -like "Holo-RGAA-*" } | Select-Object -First 1
    if ($repoRoot -and (Test-Path (Join-Path $repoRoot.FullName "claude-plugin"))) {
        if (Test-Path $PluginDir) { Remove-Item $PluginDir -Recurse -Force }
        New-Item -ItemType Directory -Force -Path (Split-Path $PluginDir) | Out-Null
        Copy-Item (Join-Path $repoRoot.FullName "claude-plugin") $PluginDir -Recurse -Force
        Write-Host "  Plugin installed: $PluginDir" -ForegroundColor Green
    } else {
        Write-Host "  WARNING: plugin not in tarball; continuing without plugin." -ForegroundColor Yellow
    }
} catch {
    Write-Host "  WARNING: plugin download failed; continuing without plugin." -ForegroundColor Yellow
}

# Default config
$ConfigPath = ".rgaa\config.yaml"
if (-not (Test-Path $ConfigPath)) {
    Write-Step "Creating default config..."
    New-Item -ItemType Directory -Force -Path ".rgaa" | Out-Null
    $configYaml = @"
url_profiles:
  default:
    url: https://example.test
    viewport: desktop

viewport_profiles:
  desktop:
    width: 1000
    height: 1080
  mobile:
    width: 375
    height: 812

guided_tests: []

standards:
  - wcag
  - rgai

policy:
  min_compliance: 80.0
  required_criteria: []

evidence_dir: .rgaa/evidence
remote_endpoint: null
upload_consent: false
"@
    Set-Content -Path $ConfigPath -Value $configYaml -Encoding UTF8
    Write-Host "  Default config created: $ConfigPath" -ForegroundColor Green
} else {
    Write-Host "  Config exists: $ConfigPath" -ForegroundColor Green
}

# Verify versions
Write-Step "Verifying..."
foreach ($b in @("rgaa.exe","rgaa-cli.exe")) {
    & (Join-Path $InstallDir $b) --version
    if ($LASTEXITCODE -ne 0) { Write-Host "  ERROR: $b --version failed" -ForegroundColor Red; exit 1 }
}
foreach ($b in @("rgaa-api.exe","rgaa-mcp.exe")) {
    & (Join-Path $InstallDir $b) --help 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-Host "  ERROR: $b --help failed" -ForegroundColor Red; exit 1 }
}
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
