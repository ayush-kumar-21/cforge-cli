#!/bin/bash
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

mkdir -p "$INSTALL_DIR"
echo "Downloading cforge ($ASSET)..."
curl -fsSL "https://github.com/$REPO/releases/latest/download/$ASSET" -o "$INSTALL_DIR/cforge"
chmod +x "$INSTALL_DIR/cforge"

add_path_line() {
    local rc="$1"
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
echo "Then run: cforge help"
