#!/bin/bash
set -e

REPO="myferr/meow"
ARCH="linux-x86_64"

if [[ "$(uname)" == "Darwin" ]]; then
    case "$(uname -m)" in
        x86_64) ARCH="macos-x86_64" ;;
        arm64)  ARCH="macos-aarch64" ;;
        *) echo "Unsupported macOS arch"; exit 1 ;;
    esac
elif [[ "$(uname)" == "Linux" ]]; then
    ARCH="linux-x86_64"
else
    echo "Unsupported OS"; exit 1
fi

echo "Fetching latest release info..."
URL=$(curl -s https://api.github.com/repos/$REPO/releases/latest \
    | grep "browser_download_url" \
    | grep "meow-$ARCH" \
    | cut -d '"' -f 4)

if [[ -z "$URL" ]]; then
    echo "No binary found for $ARCH"
    exit 1
fi

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

echo "Downloading $URL to $BIN_DIR/meow"
curl -L "$URL" -o "$BIN_DIR/meow"
chmod +x "$BIN_DIR/meow"

if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "⚠️ Add $BIN_DIR to your PATH (e.g. in ~/.bashrc or ~/.zshrc):"
    echo 'export PATH="$HOME/.local/bin:$PATH"'
fi

echo "✅ Installed meow to $BIN_DIR/meow"
