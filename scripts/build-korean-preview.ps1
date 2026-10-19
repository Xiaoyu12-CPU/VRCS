[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
    throw "Build the Korean preview on Windows x64."
}
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$desktopRoot = Join-Path $repoRoot "apps\desktop\src-tauri"
$generatedConfig = Join-Path $desktopRoot "tauri.preview.generated.conf.json"
$targetRoot = Join-Path $repoRoot ".build"
$artifactRoot = Join-Path $repoRoot "release-artifacts"
$previousPreview = $env:VRCS_PREVIEW
$previousTarget = $env:CARGO_TARGET_DIR

Push-Location $repoRoot
try {
    if (-not [string]::IsNullOrWhiteSpace($env:TAURI_UPDATER_PUBLIC_KEY)) {
        throw "Do not configure the official updater key for the independent preview."
    }
    $env:VRCS_PREVIEW = "1"
    $env:CARGO_TARGET_DIR = $targetRoot
    $config = Get-Content -LiteralPath (Join-Path $desktopRoot "tauri.release.conf.json") -Raw | ConvertFrom-Json
    $config.identifier = "app.vrcs.korean-preview"
    $config | Add-Member -Force -NotePropertyName "productName" -NotePropertyValue "VRCS Korean Preview"
    $config | Add-Member -Force -NotePropertyName "mainBinaryName" -NotePropertyValue "vrcs-korean-preview"
    $config.bundle.createUpdaterArtifacts = $false
    [System.IO.File]::WriteAllText($generatedConfig, ($config | ConvertTo-Json -Depth 20), [System.Text.UTF8Encoding]::new($false))

    $installerRoot = Join-Path $targetRoot "release\bundle\nsis"
    if (Test-Path -LiteralPath $installerRoot) {
        Remove-Item -LiteralPath $installerRoot -Recurse -Force
    }
    & npm --workspace apps/desktop run tauri -- build --features vulkan --config src-tauri/tauri.preview.generated.conf.json --bundles nsis --ci --no-sign
    if ($LASTEXITCODE -ne 0) { throw "Korean preview build failed."
    }
    $executable = Join-Path $targetRoot "release\vrcs-korean-preview.exe"
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw "Preview executable was not produced."
    }
    $selfTest = Start-Process -FilePath $executable -ArgumentList "--release-self-test" -Wait -PassThru -WindowStyle Hidden
    if ($selfTest.ExitCode -ne 0) { throw "Preview self-test failed: $($selfTest.ExitCode)" }
    $installers = @(Get-ChildItem -LiteralPath $installerRoot -Filter "*.exe" -File)
    if ($installers.Count -ne 1) { throw "Expected one preview installer, found $($installers.Count)." }
    $version = (Get-Content -LiteralPath (Join-Path $desktopRoot "tauri.conf.json") -Raw | ConvertFrom-Json).version
    New-Item -ItemType Directory -Path $artifactRoot -Force | Out-Null
    $artifact = Join-Path $artifactRoot "VRCS-Korean-Preview-$version-windows-x64.exe"
    Copy-Item -LiteralPath $installers[0].FullName -Destination $artifact -Force
    $hash = (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash.ToLowerInvariant()
    [System.IO.File]::WriteAllText("$artifact.sha256", "$hash  $([System.IO.Path]::GetFileName($artifact))`n", [System.Text.UTF8Encoding]::new($false))
    [System.IO.File]::WriteAllText((Join-Path $artifactRoot "VRCS-Korean-Preview-README.txt"), @"
VRCS Korean Preview $version (Windows x64)

Independent preview with Korean OCR, dictionary lookup and learning improvements.
This installer is unsigned. Its updater is disabled.
Settings, history, models and saved credentials are separate from standard VRCS.
You will need to select your devices, download models and configure services again.
Exit standard VRCS before starting this preview: both use the SteamVR overlay names.

For Korean signs: Vision settings > OCR > Screen text language > Korean (includes English).
Download that model bundle, then choose your translation target separately.
Learning settings > Dictionary provides one-click install for Korean-Chinese and Korean-Japanese KRDICT dictionaries.
Your own compatible Yomitan ZIP dictionaries can still be imported manually.
Several possible dictionary forms may be shown; use the sentence context to decide.

Windows build and automated checks do not replace headset/DirectML field testing.
"@, [System.Text.UTF8Encoding]::new($false))
    Write-Host "Korean preview installer: $artifact"
}
finally {
    if (Test-Path -LiteralPath $generatedConfig) { Remove-Item -LiteralPath $generatedConfig -Force }
    $env:VRCS_PREVIEW = $previousPreview
    $env:CARGO_TARGET_DIR = $previousTarget
    Pop-Location
}
