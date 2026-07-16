$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$package = Get-Content (Join-Path $root "package.json") -Raw -Encoding UTF8 | ConvertFrom-Json
$packageLockHeader = Get-Content (Join-Path $root "package-lock.json") -Encoding UTF8 | Select-Object -First 12 | Out-String
$tauriConfig = Get-Content (Join-Path $root "src-tauri\tauri.conf.json") -Raw -Encoding UTF8 | ConvertFrom-Json
$cargoToml = Get-Content (Join-Path $root "src-tauri\Cargo.toml") -Raw -Encoding UTF8
$version = $package.version

$failed = @()
if ($env:GITHUB_REF_NAME -and $env:GITHUB_REF_NAME -ne "v$version") { $failed += "Git tag $env:GITHUB_REF_NAME (expected v$version)" }
if (-not (Test-Path (Join-Path $root "docs\market\stable.json"))) { $failed += "docs/market/stable.json" }
if ([regex]::Matches($packageLockHeader, "`"version`"\s*:\s*`"$([regex]::Escape($version))`"").Count -ne 2) { $failed += "package-lock.json" }
if ($cargoToml -notmatch "(?m)^version\s*=\s*`"$([regex]::Escape($version))`"") { $failed += "src-tauri/Cargo.toml" }
if ($tauriConfig.version -ne $version) { $failed += "src-tauri/tauri.conf.json" }
$readme = Get-Content (Join-Path $root "README.md") -Raw -Encoding UTF8
$website = Get-Content (Join-Path $root "docs\index.html") -Raw -Encoding UTF8
if ($readme -notmatch "``$([regex]::Escape($version))``") { $failed += "README.md" }
if (-not (Select-String -Path (Join-Path $root "RELEASES.md") -SimpleMatch "## $version " -Quiet -Encoding UTF8)) { $failed += "RELEASES.md" }
if ($website -notmatch "id=`"download-title`"[^>]*>[^<]*v$([regex]::Escape($version))<") { $failed += "docs/index.html" }

if ($failed.Count -gt 0) {
  throw "Version $version is not synchronized in: $($failed -join ', ')"
}

Write-Host "Version $version is synchronized across application, release notes, and website."
