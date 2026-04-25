#Requires -Version 5.1
<#
.SYNOPSIS
  Cross-CLI installer for all-cli releases (Windows).
.DESCRIPTION
  Downloads a prebuilt binary from GitHub Releases and verifies SHA256.
.PARAMETER Cli
  CLI name (matches the <cli-name> prefix in release tags).
.PARAMETER Version
  Version like "v0.1.0" or "latest" (default).
.PARAMETER InstallDir
  Install directory; defaults to %LOCALAPPDATA%\Programs\<cli>.
.PARAMETER Repo
  GitHub repo (default: yuebanhome/all-cli).
.PARAMETER DryRun
  Print plan and exit without downloading.

  Exit codes: 0 success | 1 generic | 2 unsupported platform | 3 download failure | 4 hash mismatch
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory=$true)][string]$Cli,
  [string]$Version = 'latest',
  [string]$InstallDir = '',
  [string]$Repo = 'yuebanhome/all-cli',
  [switch]$DryRun
)

$ErrorActionPreference = 'Stop'

function Detect-Target {
  switch ($env:PROCESSOR_ARCHITECTURE) {
    'AMD64' { return 'x86_64-pc-windows-msvc' }
    default {
      Write-Host "Unsupported architecture: $($env:PROCESSOR_ARCHITECTURE)" -ForegroundColor Red
      exit 2
    }
  }
}

function Resolve-CliVersion {
  param([string]$Cli, [string]$Version, [string]$Repo)
  if ($Version -ne 'latest') { return $Version }
  $api = "https://api.github.com/repos/$Repo/releases?per_page=100"
  try {
    $releases = Invoke-RestMethod -Uri $api -Headers @{ 'User-Agent' = 'all-cli-installer' }
  } catch {
    Write-Host "Failed to query GitHub API at $api (network error or rate limit): $_" -ForegroundColor Red
    exit 3
  }
  $tag = $releases | Where-Object { $_.tag_name -like "$Cli-v*" } | Select-Object -First 1 -ExpandProperty tag_name
  if (-not $tag) { Write-Host "No release tagged $Cli-v* in $Repo" -ForegroundColor Red; exit 3 }
  return $tag.Substring("$Cli-".Length)
}

if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\$Cli" }

$target  = Detect-Target
$ver     = Resolve-CliVersion -Cli $Cli -Version $Version -Repo $Repo
if ($ver -notlike 'v*') { $ver = "v$ver" }
$archive = "$Cli-$ver-$target.zip"
$url     = "https://github.com/$Repo/releases/download/$Cli-$ver/$archive"
$sumsUrl = "https://github.com/$Repo/releases/download/$Cli-$ver/SHA256SUMS"

Write-Host "CLI       : $Cli"
Write-Host "Version   : $ver"
Write-Host "Target    : $target"
Write-Host "Archive   : $url"
Write-Host "InstallTo : $InstallDir"

if ($DryRun) { Write-Host "(dry run, not downloading)"; return }

$tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ([guid]::NewGuid())) -Force
try {
  Write-Host "Downloading archive..."
  try {
    Invoke-WebRequest -Uri $url -OutFile (Join-Path $tmp $archive) -UseBasicParsing
  } catch { Write-Host "Download failed: $url`n$_" -ForegroundColor Red; exit 3 }

  Write-Host "Downloading SHA256SUMS..."
  try {
    Invoke-WebRequest -Uri $sumsUrl -OutFile (Join-Path $tmp 'SHA256SUMS') -UseBasicParsing
  } catch { Write-Host "Download failed: $sumsUrl`n$_" -ForegroundColor Red; exit 3 }

  $expected = $null
  foreach ($line in (Get-Content (Join-Path $tmp 'SHA256SUMS'))) {
    $parts = $line -split '\s+', 2
    if ($parts.Length -eq 2 -and $parts[1].Trim() -eq $archive) {
      $expected = $parts[0].ToLower()
      break
    }
  }
  if (-not $expected) { Write-Host "SHA256SUMS missing entry for $archive" -ForegroundColor Red; exit 4 }
  $actual = (Get-FileHash -Algorithm SHA256 -Path (Join-Path $tmp $archive)).Hash.ToLower()
  if ($expected -ne $actual) {
    Write-Host "SHA256 mismatch for $archive`n  expected: $expected`n  actual:   $actual" -ForegroundColor Red
    exit 4
  }
  Write-Host "SHA256 OK"

  Expand-Archive -Path (Join-Path $tmp $archive) -DestinationPath $tmp -Force
  New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
  $exe = "$Cli.exe"
  Move-Item -Force -Path (Join-Path $tmp $exe) -Destination (Join-Path $InstallDir $exe)
  Write-Host "Installed: $(Join-Path $InstallDir $exe)"

  $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  $onPath = ($userPath -split ';' | Where-Object { $_ -ieq $InstallDir })
  if (-not $onPath) {
    Write-Host ""
    Write-Host "Note: $InstallDir is not on PATH. Add it via:"
    Write-Host "  setx PATH `"%PATH%;$InstallDir`""
  }

  & (Join-Path $InstallDir $exe) --version
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
