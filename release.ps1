# Builds a signed Agate installer and the latest.json the in-app updater reads.
#
#   .\release.ps1                  build only (files in src-tauri\target\release\bundle\nsis)
#   .\release.ps1 -Publish         build and create the GitHub release v<version> with gh
#   .\release.ps1 -Notes "..."     release notes shown in the app
#
# Bump "version" in src-tauri\tauri.conf.json (and Cargo.toml) before releasing.
# The private key stays outside the repository; without it the update cannot be signed.
param(
    [string]$Notes = "",
    [string]$Key = "$env:USERPROFILE\.tauri\agate-updater.key",
    [string]$Repo = "MooseDevPJS/agate",
    [switch]$Publish
)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

if (-not (Test-Path $Key)) { throw "Signing key not found: $Key" }
$conf = Get-Content "src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
$version = $conf.version
$tag = "v$version"

$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $Key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
# The CLI logs progress on stderr: judge it by its exit code only (Windows PowerShell 5.1
# would otherwise stop at the first stderr line).
$ErrorActionPreference = "Continue"
& ".\.tools\tauri-cli\node_modules\.bin\tauri.cmd" build --bundles nsis 2>&1 | ForEach-Object { "$_" }
$built = $LASTEXITCODE
$ErrorActionPreference = "Stop"
if ($built -ne 0) { throw "Build failed" }

$dir = "src-tauri\target\release\bundle\nsis"
$setup = "Agate_${version}_x64-setup.exe"
$sig = (Get-Content "$dir\$setup.sig" -Raw).Trim()
$latest = [ordered]@{
    version   = $version
    notes     = $Notes
    pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    platforms = [ordered]@{
        "windows-x86_64" = [ordered]@{
            signature = $sig
            url       = "https://github.com/$Repo/releases/download/$tag/$setup"
        }
    }
}
$json = $latest | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText((Join-Path (Resolve-Path $dir) "latest.json"), $json, (New-Object Text.UTF8Encoding $false))
Write-Host "Built $setup and latest.json in $dir"

if ($Publish) {
    $title = "Agate $version"
    if ($Notes) { gh release create $tag "$dir\$setup" "$dir\latest.json" --repo $Repo --title $title --notes $Notes }
    else { gh release create $tag "$dir\$setup" "$dir\latest.json" --repo $Repo --title $title --generate-notes }
    if ($LASTEXITCODE -ne 0) { throw "gh release failed" }
    Write-Host "Published $tag"
}
