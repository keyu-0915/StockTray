$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

$package = Get-Content "package.json" -Raw -Encoding UTF8 | ConvertFrom-Json
$version = $package.version

$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
if ($cargoCommand) {
  $cargo = $cargoCommand.Source
} else {
  $cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
}

if (-not (Test-Path $cargo)) {
  throw "cargo.exe was not found. Install Rust or add Cargo to PATH."
}

$cargoDir = Split-Path $cargo
if (($env:PATH -split ";") -notcontains $cargoDir) {
  $env:PATH = "$cargoDir;$env:PATH"
}

function Invoke-Checked {
  param(
    [Parameter(Mandatory = $true)]
    [string] $FilePath,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $Arguments
  )

  & $FilePath @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "Command failed with exit code ${LASTEXITCODE}: $FilePath $($Arguments -join ' ')"
  }
}

Invoke-Checked npm run build
Invoke-Checked $cargo fmt --manifest-path "src-tauri\Cargo.toml" --check
Invoke-Checked $cargo check --manifest-path "src-tauri\Cargo.toml"
Invoke-Checked $cargo test --manifest-path "src-tauri\Cargo.toml"
Invoke-Checked $cargo clippy --manifest-path "src-tauri\Cargo.toml" "--" "-D" "warnings"
Invoke-Checked npm run tauri:build

$expectedInstaller = Join-Path $root "src-tauri\target\release\bundle\nsis\StockTray_${version}_x64-setup.exe"
if (Test-Path $expectedInstaller) {
  $installer = Get-Item $expectedInstaller
} else {
  $installer = Get-ChildItem (Join-Path $root "src-tauri\target\release\bundle\nsis") -Filter "StockTray_*_x64-setup.exe" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
}

if (-not $installer) {
  throw "No NSIS installer was produced."
}

$releaseDir = Join-Path $root "releases"
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
$target = Join-Path $releaseDir "StockTray_${version}_x64-setup.exe"
Copy-Item $installer.FullName $target -Force

Write-Host "Packaged $target"
