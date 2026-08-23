# Install cforge from the latest GitHub release. Run directly:
#   irm https://raw.githubusercontent.com/ayush-kumar-21/cforge-cli/main/install.ps1 | iex
$ErrorActionPreference = "Stop"

$Repo = "ayush-kumar-21/cforge-cli"
$Asset = "cforge-windows-x86_64.exe"
$InstallDir = Join-Path $HOME ".local\bin"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

# Which release to install: releases/latest first (github.com, not the
# rate-limited API -- redirects to .../releases/tag/<Tag> when a
# non-prerelease release exists). cforge is pre-1.0 (see SECURITY.md), so
# right now there isn't one, and that redirects to the bare /releases list
# instead -- caught by the regex requiring "/tag/" in the Location header.
# Falls back to the API's release list, whose first entry is the newest
# release regardless of prerelease status; that's subject to the
# unauthenticated API's 60-requests-per-hour-per-IP limit, so it's only
# paid while there's no stable release to redirect to. Once cforge ships
# one, this reverts to the cheap path on its own -- no script change needed.
#
# HttpWebRequest rather than Invoke-WebRequest: its exception shape on a
# non-2xx response differs between Windows PowerShell 5.1 and PowerShell 7,
# and this script has to work under both. HttpWebRequest/HttpWebResponse
# are the same stable .NET type on both.
function Resolve-ReleaseTag {
    $req = [System.Net.HttpWebRequest]::Create("https://github.com/$Repo/releases/latest")
    $req.AllowAutoRedirect = $false
    $req.Method = "HEAD"
    try {
        $resp = $req.GetResponse()
        $location = $resp.Headers["Location"]
        $resp.Close()
    } catch [System.Net.WebException] {
        $location = $_.Exception.Response.Headers["Location"]
    }
    if ($location -match '/releases/tag/([^/]+)$') {
        return $Matches[1]
    }
    try {
        $releases = Invoke-RestMethod -UseBasicParsing -Uri "https://api.github.com/repos/$Repo/releases"
        if ($releases -and $releases.Count -gt 0) {
            return $releases[0].tag_name
        }
    } catch {
        return $null
    }
    return $null
}

$Tag = Resolve-ReleaseTag
if (-not $Tag) {
    throw "Could not determine which release to install (GitHub may be rate-limiting this IP). Download manually from https://github.com/$Repo/releases and place the binary on your PATH."
}
$Base = "https://github.com/$Repo/releases/download/$Tag"

# Download to a temp file, not straight over the installed binary: a failed
# or tampered download must never replace a working cforge.
$TmpBin = Join-Path ([System.IO.Path]::GetTempPath()) "cforge-$([guid]::NewGuid()).exe"

try {
    Write-Host "Downloading cforge $Tag ($Asset)..."
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
