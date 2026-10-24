# eTamil installer for Windows.
#
# Copies the compiler and its standard library into your user profile and puts
# etamil on PATH. No administrator rights, no Rust, no Visual Studio.
#
#   powershell -ExecutionPolicy Bypass -File install.ps1

$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$target = Join-Path $env:LOCALAPPDATA "Programs\eTamil"

Write-Host "Installing eTamil to $target"

New-Item -ItemType Directory -Force $target | Out-Null
Copy-Item "$here\etamil.exe" $target -Force
# The language server, for editors; packages from before it existed lack it.
if (Test-Path "$here\etamil-lsp.exe") { Copy-Item "$here\etamil-lsp.exe" $target -Force }
foreach ($dir in "nUlakam", "examples") {
    if (Test-Path "$here\$dir") {
        Remove-Item "$target\$dir" -Recurse -Force -ErrorAction SilentlyContinue
        Copy-Item "$here\$dir" $target -Recurse -Force
    }
}

# --- PATH ---
# Read the raw value so %USERPROFILE%-style entries are not expanded, and write
# it back with its original type. Using [Environment]::SetEnvironmentVariable
# here would flatten those entries permanently.
$key = Get-Item 'HKCU:\Environment'
$raw = $key.GetValue('Path', '', 'DoNotExpandEnvironmentNames')
$kind = $key.GetValueKind('Path')

if ($raw -split ';' -notcontains $target) {
    $new = if ([string]::IsNullOrEmpty($raw)) { $target } else { "$raw;$target" }
    Set-ItemProperty 'HKCU:\Environment' -Name Path -Value $new -Type $kind
    Write-Host "Added to PATH"
} else {
    Write-Host "Already on PATH"
}

# ETAMIL_PATH lets  இறக்கு "nUlakam/paNam/paNam.qmz"  resolve from anywhere.
Set-ItemProperty 'HKCU:\Environment' -Name ETAMIL_PATH -Value $target -Type String
Write-Host "ETAMIL_PATH set to $target"

Write-Host ""
Write-Host "Done. Open a NEW terminal, then:"
Write-Host "    etamil --version"
Write-Host "    etamil --vm `"$target\examples\basic_samples\example.qmz`""
Write-Host ""
Write-Host "To uninstall: remove $target, then delete the PATH entry and"
Write-Host "ETAMIL_PATH from Settings > Environment Variables."
