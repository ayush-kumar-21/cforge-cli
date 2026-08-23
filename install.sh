#!/bin/sh
# Install cforge from the latest GitHub release. Run directly:
#   curl -fsSL https://raw.githubusercontent.com/ayush-kumar-21/cforge-cli/main/install.sh | sh
set -e

REPO="ayush-kumar-21/cforge-cli"
INSTALL_DIR="$HOME/.local/bin"

case "$(uname -s)" in
    Darwin) OS_KIND=macos ;;
    Linux)  OS_KIND=linux ;;
    *)
        echo "Error: unsupported OS for this script: $(uname -s)"
        echo "On Windows, use install.ps1 from PowerShell instead."
        exit 1
        ;;
esac

case "$OS_KIND-$(uname -m)" in
    macos-arm64)   ASSET="cforge-macos-arm64" ;;
    macos-x86_64)  ASSET="cforge-macos-x86_64" ;;
    linux-x86_64)  ASSET="cforge-linux-x86_64" ;;
    *)
        echo "Error: no prebuilt binary for $OS_KIND/$(uname -m)."
        echo "Build from source instead: clone https://github.com/$REPO and run 'cargo build --release'."
        exit 1
        ;;
esac

# Pick whichever SHA-256 tool this platform ships: sha256sum on Linux
# (coreutils), shasum on macOS. Verification is mandatory, so bail with a
# clear message rather than silently installing something unchecked.
if command -v sha256sum >/dev/null 2>&1; then
    sha256_of() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum >/dev/null 2>&1; then
    sha256_of() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
    echo "Error: neither sha256sum nor shasum found; cannot verify the download."
    echo "Install coreutils (or use the manual instructions in the README) and re-run."
    exit 1
fi

mkdir -p "$INSTALL_DIR"

# Download to a temp file, not straight over the installed binary: a failed
# or tampered download must never replace a working cforge.
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
TMP_BIN="$TMP_DIR/cforge"

# Which release to install: releases/latest first (github.com, not the
# rate-limited API — a redirect to .../releases/tag/<TAG> when a
# non-prerelease release exists). cforge is pre-1.0 (see SECURITY.md), so
# right now there isn't one, and that redirects to the bare /releases list
# instead — caught by the case pattern requiring "/tag/" in the target.
# Falls back to the API's release list, whose first entry is the newest
# release regardless of prerelease status; that's subject to the
# unauthenticated API's 60-requests-per-hour-per-IP limit, so it's only
# paid while there's no stable release to redirect to. Once cforge ships
# one, this reverts to the cheap path on its own — no script change needed.
REDIRECT="$(curl -s -o /dev/null -w '%{redirect_url}' "https://github.com/$REPO/releases/latest")"
case "$REDIRECT" in
    */releases/tag/*) TAG="${REDIRECT##*/releases/tag/}" ;;
    *) TAG="" ;;
esac
if [ -z "$TAG" ]; then
    TAG="$(curl -s "https://api.github.com/repos/$REPO/releases" 2>/dev/null \
        | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
fi
if [ -z "$TAG" ]; then
    echo "Error: could not determine which release to install (GitHub may be rate-limiting this IP)."
    echo "Download manually from https://github.com/$REPO/releases and place the binary on your PATH."
    exit 1
fi

BASE="https://github.com/$REPO/releases/download/$TAG"
echo "Downloading cforge $TAG ($ASSET)..."
curl -fsSL "$BASE/$ASSET" -o "$TMP_BIN"

# Fail closed: a checksum that can't be fetched is not "unverified but
# fine" — whoever can substitute the binary can also 404 the checksum.
echo "Verifying checksum..."
if ! EXPECTED="$(curl -fsSL "$BASE/$ASSET.sha256")"; then
    echo "Error: could not fetch the published checksum for $ASSET."
    echo "Refusing to install an unverified binary."
    exit 1
fi
EXPECTED="$(printf '%s' "$EXPECTED" | tr -d '[:space:]' | tr '[:upper:]' '[:lower:]')"
ACTUAL="$(sha256_of "$TMP_BIN")"
if [ "$EXPECTED" != "$ACTUAL" ]; then
    echo "Error: checksum mismatch for $ASSET — refusing to install."
    echo "  expected $EXPECTED"
    echo "  actual   $ACTUAL"
    echo "The download was corrupted or tampered with. Nothing has been changed."
    exit 1
fi

chmod +x "$TMP_BIN"
mv "$TMP_BIN" "$INSTALL_DIR/cforge"

# No 'local': this script is documented as 'curl ... | sh', so the shebang
# above is bypassed and it runs under whatever /bin/sh is. 'local' is a
# bash/dash extension, undefined in POSIX sh. Only 'rc' would be scoped and
# nothing else uses that name, so plain assignment is equivalent here.
add_path_line() {
    rc="$1"
    [ -f "$rc" ] || return 0
    grep -q '# cforge PATH' "$rc" 2>/dev/null && return 0
    {
        echo ''
        echo '# cforge PATH'
        echo 'export PATH="$HOME/.local/bin:$PATH"'
    } >> "$rc"
    echo "Added $INSTALL_DIR to PATH in $rc"
}

add_path_line "$HOME/.zshrc"
add_path_line "$HOME/.bashrc"
add_path_line "$HOME/.bash_profile"
add_path_line "$HOME/.profile"

echo
echo "cforge installed to $INSTALL_DIR/cforge (platform: $OS_KIND)"
echo "Open a new terminal (or 'source ~/.zshrc' / equivalent) so PATH changes take effect."
echo "Then run: cforge --help"
