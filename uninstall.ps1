# cforge uninstaller for native Windows (PowerShell). Run from anywhere:
#   .\uninstall.ps1
$ErrorActionPreference = "Stop"

$InstallDir = Join-Path $HOME ".local\bin"
$Target = Join-Path $InstallDir "cforge.exe"

if (Test-Path $Target) {
    Remove-Item -Force $Target
    Write-Host "Removed $Target"
} else {
    Write-Host "cforge is not installed at $Target; nothing to remove there."
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -like "*$InstallDir*") {
    $newPath = ($userPath -split ';' | Where-Object { $_ -ne $InstallDir }) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Removed $InstallDir from your User PATH."
}

Write-Host ""
Write-Host "cforge uninstalled. Your projects and their CMakeLists.txt files are untouched."
