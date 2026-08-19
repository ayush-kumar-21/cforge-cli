# Install cforge from the latest GitHub release. Run directly:
#   irm https://raw.githubusercontent.com/ayush-kumar-21/cforge-cli/main/install.ps1 | iex
$ErrorActionPreference = "Stop"

$Repo = "ayush-kumar-21/cforge-cli"
$Asset = "cforge-windows-x86_64.exe"
$InstallDir = Join-Path $HOME ".local\bin"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

Write-Host "Downloading cforge ($Asset)..."
Invoke-WebRequest -Uri "https://github.com/$Repo/releases/latest/download/$Asset" `
    -OutFile (Join-Path $InstallDir "cforge.exe")

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to your User PATH."
} else {
    Write-Host "$InstallDir is already on your User PATH."
}

Write-Host ""
Write-Host "cforge installed to $InstallDir\cforge.exe"
Write-Host "Open a new terminal so PATH changes take effect, then run: cforge help"
