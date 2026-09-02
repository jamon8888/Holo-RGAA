#Requires -Version 5.0
param(
    [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"
$Repo = "your-org/rgaa-cli"
$InstallDir = "$env:LOCALAPPDATA\rgaa\bin"
$TmpDir = [System.IO.Path]::GetTempPath()

function Get-GithubRelease {
    param([string]$Tag)
    $uri = if ($Tag -eq "latest") {
        "https://api.github.com/repos/$Repo/releases/latest"
    } else {
        "https://api.github.com/repos/$Repo/releases/tags/v$Tag"
    }
    $headers = @{ "Accept" = "application/vnd.github+json" }
    Invoke-RestMethod -Uri $uri -Headers $headers
}

Write-Host "" 2>&1
Write-Host "==> Fetching release info..." -ForegroundColor Cyan

$release = Get-GithubRelease -Tag $Version
$tag = $release.tag_name.TrimStart('v')
Write-Host "  Latest version: v$tag"

$arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "aarch64" } else { "x86_64" }
$platform = "windows-$arch"
$assetName = "rgaa-cli-${tag}-${platform}.tar.gz"
$url = "https://github.com/${Repo}/releases/download/v${tag}/${assetName}"

Write-Host "  Platform: $platform"
Write-Host ""

Write-Host "==> Downloading $assetName..." -ForegroundColor Cyan
$outPath = Join-Path $TmpDir $assetName
Invoke-WebRequest -Uri $url -OutFile $outPath -UserAgent "rgaa-install"

Write-Host "==> Extracting..." -ForegroundColor Cyan
$extractDir = Join-Path $TmpDir "rgaa-extract"
New-Item -ItemType Directory -Force -Path $extractDir | Out-Null
tar -xzf $outPath -C $extractDir

Write-Host "==> Installing to $InstallDir..." -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item "$extractDir\rgaa.exe" "$InstallDir\" -Force
Remove-Item $outPath
Remove-Item $extractDir -Recurse -Force

$pathEntry = $InstallDir
if ($env:Path -notlike "*$InstallDir*") {
    Write-Host ""
    Write-Host "==> Adding $InstallDir to PATH..." -ForegroundColor Cyan
    $currentScope = if ([System.Environment]::GetEnvironmentVariable("Path", "User") -like "*$InstallDir*") { "User" } else { "Process" }
    [System.Environment]::SetEnvironmentVariable("Path", "$pathEntry;$($env:Path)", $currentScope)
    $env:Path = "$pathEntry;$($env:Path)"
}

Write-Host ""
Write-Host "  Installed! Run: rgaa tui" -ForegroundColor Green
