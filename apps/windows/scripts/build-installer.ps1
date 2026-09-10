$ErrorActionPreference = 'Stop'
$featureArgs = if ($env:NOTCH_CHANNEL -eq 'stable') { @('--features', 'stable-channel') } else { @() }
Push-Location (Join-Path $PSScriptRoot '..')
try {
    # The helper must exist before Tauri resolves bundle resources.
    cargo build --locked --release --package notch-hook @featureArgs
    if ($LASTEXITCODE -ne 0) { throw 'Hook build failed.' }
    Push-Location notch
    try {
        # Invoke the native shim: PowerShell's npm shim consumes the -- separator.
        tauri.cmd build --ci --bundles nsis @featureArgs -- --locked
        if ($LASTEXITCODE -ne 0) { throw 'Installer build failed.' }
    } finally { Pop-Location }
} finally { Pop-Location }
