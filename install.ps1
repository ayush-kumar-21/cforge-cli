# Install cforge from the latest GitHub release. Run directly:
#   irm https://raw.githubusercontent.com/ayush-kumar-21/cforge-cli/main/install.ps1 | iex
$ErrorActionPreference = "Stop"

$Repo = "ayush-kumar-21/cforge-cli"
$Asset = "cforge-windows-x86_64.exe"
$InstallDir = Join-Path $HOME ".local\bin"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

$Base = "https://github.com/$Repo/releases/latest/download"

# Download to a temp file, not straight over the installed binary: a failed
# or tampered download must never replace a working cforge.
$TmpBin = Join-Path ([System.IO.Path]::GetTempPath()) "cforge-$([guid]::NewGuid()).exe"

try {
    Write-Host "Downloading cforge ($Asset)..."
    Invoke-WebRequest -UseBasicParsing -Uri "$Base/$Asset" -OutFile $TmpBin

    # Fail closed: a checksum that can't be fetched is not "unverified but
    # fine" - whoever can substitute the binary can also 404 the checksum.
    Write-Host "Verifying checksum..."
    try {
        $Expected = (Invoke-WebRequest -UseBasicParsing -Uri "$Base/$Asset.sha256").Content
    } catch {
        throw "Could not fetch the published checksum for $Asset. Refusing to install an unverified binary."
    }
    $Expected = $Expected.Trim().ToLower()
    $Actual = (Get-FileHash -Algorithm SHA256 $TmpBin).Hash.ToLower()
    if ($Expected -ne $Actual) {
        throw ("Checksum mismatch for {0} - refusing to install.`n  expected {1}`n  actual   {2}`n" -f $Asset, $Expected, $Actual) +
              "The download was corrupted or tampered with. Nothing has been changed."
    }

    Move-Item -Force $TmpBin (Join-Path $InstallDir "cforge.exe")
} finally {
    if (Test-Path $TmpBin) { Remove-Item -Force $TmpBin }
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to your User PATH."
} else {
    Write-Host "$InstallDir is already on your User PATH."
}

Write-Host ""
Write-Host "cforge installed to $InstallDir\cforge.exe"
Write-Host "Open a new terminal so PATH changes take effect, then run: cforge --help"
