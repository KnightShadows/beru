#!/usr/bin/env bash
set -e

REPO="KnightShadows/Beru"
BIN="beru"

echo "Installing Beru..."

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        if [ "$ARCH" = "x86_64" ]; then
            TARGET="x86_64-unknown-linux-musl"
        elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
            TARGET="aarch64-unknown-linux-musl"
        else
            echo "Error: Architecture $ARCH is not supported by pre-built Linux binaries."
            exit 1
        fi
        ;;
    Darwin)
        if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
            TARGET="aarch64-apple-darwin"
        elif [ "$ARCH" = "x86_64" ]; then
            TARGET="x86_64-apple-darwin"
        else
            echo "Error: Architecture $ARCH is not supported by pre-built macOS binaries."
            exit 1
        fi
        ;;
    *)
        echo "Error: Unsupported OS $OS for this script. Please build from source or use install.ps1 on Windows."
        exit 1
        ;;
esac

echo "Fetching latest release data for $TARGET..."
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/$REPO/releases/latest")

if echo "$LATEST_RELEASE" | grep -q "Not Found"; then
    echo "Error: No releases found for $REPO yet. You will need to build from source until the first release is tagged."
    exit 1
fi

DOWNLOAD_URL=$(echo "$LATEST_RELEASE" | grep "browser_download_url" | grep "$TARGET.tar.gz" | cut -d '"' -f 4 | head -n 1)

if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Could not find a binary for $TARGET in the latest release."
    echo "Please build from source."
    exit 1
fi

echo "Downloading $DOWNLOAD_URL..."
TMP_DIR=$(mktemp -d)
curl -sL "$DOWNLOAD_URL" -o "$TMP_DIR/$BIN.tar.gz"

echo "Extracting..."
tar -xzf "$TMP_DIR/$BIN.tar.gz" -C "$TMP_DIR"

INSTALL_DIR="$HOME/.cargo/bin"
if [ ! -d "$INSTALL_DIR" ]; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

echo "Installing to $INSTALL_DIR..."
mv "$TMP_DIR/$BIN" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$BIN"

rm -rf "$TMP_DIR"

# Attempt to add to PATH automatically
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "Adding $INSTALL_DIR to your PATH..."
    for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
        if [ -f "$rc" ]; then
            if ! grep -q "$INSTALL_DIR" "$rc"; then
                echo "" >> "$rc"
                echo "# Added by Beru installer" >> "$rc"
                echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$rc"
            fi
        fi
    done
    echo "Installation complete! Please restart your terminal or run 'source ~/.bashrc' (or your shell's equivalent) to use Beru."
else
    echo "Installation complete! $INSTALL_DIR is already in your PATH."
fi
