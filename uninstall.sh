#!/bin/bash
set -e

case "$(uname -s)" in
    Darwin) SED_INPLACE=(-i '') ;;
    *)      SED_INPLACE=(-i) ;;
esac

INSTALL_DIR="$HOME/.local/bin"
TARGET="$INSTALL_DIR/cforge"

if [ -f "$TARGET" ]; then
    rm -f "$TARGET"
    echo "Removed $TARGET"
else
    echo "cforge is not installed at $TARGET; nothing to remove there."
fi

for rc in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.profile"; do
    [ -f "$rc" ] || continue
    if grep -q '# cforge PATH' "$rc" 2>/dev/null; then
        sed "${SED_INPLACE[@]}" -e '/# cforge PATH/d' -e '\|export PATH="\$HOME/\.local/bin:\$PATH"|d' "$rc"
        echo "Removed cforge PATH entry from $rc"
    fi
done

echo
echo "cforge uninstalled. Your projects and their CMakeLists.txt files are untouched."
