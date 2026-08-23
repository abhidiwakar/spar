# Build an unsigned Store MSIX from a Tauri Windows release binary.
# Run on Windows after `npm run tauri build`. Microsoft signs the upload.

param(
    [string]$Publisher = $env:MSIX_PUBLISHER,
    [string]$IdentityName = $env:MSIX_IDENTITY_NAME,
    [string]$DisplayName = $env:MSIX_DISPLAY_NAME,
    [string]$PublisherDisplayName = $env:MSIX_PUBLISHER_DISPLAY_NAME,
    [string]$Arch = "x64",
    [switch]$SkipIfUnconfigured
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$IdentityPath = Join-Path $Root "msix\identity.json"
$TemplatePath = Join-Path $Root "msix\Package.appxmanifest.template"
$TauriConf = Join-Path $Root "src-tauri\tauri.conf.json"
$ReleaseDir = Join-Path $Root "src-tauri\target\release"
$LayoutDir = Join-Path $ReleaseDir "msix-layout"
$OutDir = Join-Path $Root "src-tauri\target\release\bundle\msix"

if (-not (Test-Path $IdentityPath)) {
    throw "Missing $IdentityPath"
}

$identity = Get-Content $IdentityPath -Raw | ConvertFrom-Json
if (-not $IdentityName) { $IdentityName = $identity.identityName }
if (-not $Publisher) { $Publisher = $identity.publisher }
if (-not $DisplayName) { $DisplayName = $identity.displayName }
if (-not $PublisherDisplayName) { $PublisherDisplayName = $identity.publisherDisplayName }
$IdentityName = "$IdentityName".Trim()
$DisplayName = "$DisplayName".Trim()
$Publisher = "$Publisher".Trim()
$PublisherDisplayName = "$PublisherDisplayName".Trim()

if (-not $IdentityName) { throw "Set identityName in msix/identity.json or MSIX_IDENTITY_NAME." }
if (-not $DisplayName) { throw "Set displayName in msix/identity.json to the reserved Store name." }
if (-not $PublisherDisplayName) { throw "Set publisherDisplayName in msix/identity.json." }
if (-not $Publisher -or $Publisher -notmatch '^CN=') {
    $hint = @"
Set the Partner Center publisher in msix/identity.json or MSIX_PUBLISHER.
Partner Center → your MSIX product → Product identity → Package/Identity/Publisher
It looks like CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX
See docs/msix-store.md
"@
    if ($SkipIfUnconfigured) {
        Write-Host $hint
        exit 0
    }
    throw $hint
}

$tauri = Get-Content $TauriConf -Raw | ConvertFrom-Json
$semver = [string]$tauri.version
if ($semver -notmatch '^\d+\.\d+\.\d+$') {
    throw "src-tauri/tauri.conf.json version must be major.minor.patch, got '$semver'"
}
$msixVersion = "$semver.0"

$exe = $null
foreach ($name in @("Spar.exe", "learndsa.exe")) {
    $candidate = Join-Path $ReleaseDir $name
    if (Test-Path $candidate) {
        $exe = $candidate
        break
    }
}
if (-not $exe) {
    throw "No Spar.exe or learndsa.exe in $ReleaseDir. Run ``npm run tauri build`` on Windows first."
}

function Find-MakeAppx {
    $cmd = Get-Command makeappx.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $kits = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
    if (Test-Path $kits) {
        $found = Get-ChildItem $kits -Recurse -Filter makeappx.exe -ErrorAction SilentlyContinue |
            Where-Object { $_.Directory.Name -eq "x64" } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($found) { return $found.FullName }
    }
    return $null
}

$makeAppx = Find-MakeAppx
if (-not $makeAppx) {
    throw "makeappx.exe not found. Install the Windows 10/11 SDK (or use a windows-latest GitHub runner)."
}

if (Test-Path $LayoutDir) { Remove-Item $LayoutDir -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $LayoutDir "Assets") | Out-Null

Copy-Item $exe (Join-Path $LayoutDir (Split-Path $exe -Leaf))
Get-ChildItem $ReleaseDir -File -Filter *.dll | ForEach-Object {
    Copy-Item $_.FullName $LayoutDir
}
$resources = Join-Path $ReleaseDir "resources"
if (Test-Path $resources) {
    Copy-Item $resources (Join-Path $LayoutDir "resources") -Recurse
}

$iconDir = Join-Path $Root "src-tauri\icons"
$assets = @(
    "StoreLogo.png",
    "Square44x44Logo.png",
    "Square71x71Logo.png",
    "Square150x150Logo.png",
    "Square310x310Logo.png",
    "Wide310x150Logo.png"
)
foreach ($asset in $assets) {
    $src = Join-Path $iconDir $asset
    if (-not (Test-Path $src)) { throw "Missing Store asset $src" }
    Copy-Item $src (Join-Path $LayoutDir "Assets\$asset")
}

$manifest = Get-Content $TemplatePath -Raw
$manifest = $manifest.
    Replace("__IDENTITY_NAME__", $IdentityName).
    Replace("__DISPLAY_NAME__", $DisplayName).
    Replace("__PUBLISHER__", $Publisher).
    Replace("__PUBLISHER_DISPLAY_NAME__", $PublisherDisplayName).
    Replace("__VERSION__", $msixVersion).
    Replace("__ARCH__", $Arch).
    Replace("__EXECUTABLE__", (Split-Path $exe -Leaf))
$utf8 = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText((Join-Path $LayoutDir "AppxManifest.xml"), $manifest, $utf8)

New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
$package = Join-Path $OutDir ("Spar_{0}_{1}.msix" -f $msixVersion, $Arch)
if (Test-Path $package) { Remove-Item $package -Force }

& $makeAppx pack /d $LayoutDir /p $package /o
if ($LASTEXITCODE -ne 0) { throw "makeappx failed with exit $LASTEXITCODE" }

Write-Host "Unsigned Store package: $package"
Write-Host "Upload this .msix in Partner Center. Do not sign it first."
