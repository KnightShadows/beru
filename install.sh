#!/usr/bin/env bash
set -e

REPO="KnightShadows/Beru"
BIN="beru"

echo "Installing Beru..."

BERU_STANDALONE="${BERU_STANDALONE:-0}"
for arg in "$@"; do
    case "$arg" in
        --standalone|--user)
            BERU_STANDALONE=1
            ;;
    esac
done

OS="$(uname -s)"
ARCH="$(uname -m)"

echo "Fetching latest release data..."
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/$REPO/releases/latest")

if echo "$LATEST_RELEASE" | grep -q "Not Found"; then
    echo "Error: No releases found for $REPO yet. You will need to build from source until the first release is tagged."
    exit 1
fi

# Detect native package manager for Linux x86_64
PKG_INSTALLED=false
if [ "$OS" = "Linux" ] && [ "$ARCH" = "x86_64" ] && [ "$BERU_STANDALONE" != "1" ]; then
    PKG_TYPE=""
    if [ -f /etc/os-release ]; then
        # shellcheck disable=SC1091
        . /etc/os-release
        case "$ID $ID_LIKE" in
            *fedora*|*rhel*|*centos*|*suse*|*amzn*|*rocky*|*alma*)
                PKG_TYPE="rpm"
                ;;
            *debian*|*ubuntu*|*mint*)
                PKG_TYPE="deb"
                ;;
            *arch*|*manjaro*)
                PKG_TYPE="pacman"
                ;;
        esac
    fi

    if [ -z "$PKG_TYPE" ]; then
        if command -v rpm >/dev/null 2>&1 && (command -v dnf >/dev/null 2>&1 || command -v yum >/dev/null 2>&1 || command -v zypper >/dev/null 2>&1 || [ -f /etc/redhat-release ]); then
            PKG_TYPE="rpm"
        elif command -v dpkg >/dev/null 2>&1 || command -v apt-get >/dev/null 2>&1; then
            PKG_TYPE="deb"
        elif command -v pacman >/dev/null 2>&1; then
            PKG_TYPE="pacman"
        fi
    fi

    if [ -n "$PKG_TYPE" ]; then
        PKG_URL=""
        case "$PKG_TYPE" in
            rpm)
                PKG_URL=$(echo "$LATEST_RELEASE" | grep "browser_download_url" | grep -i "\.rpm\"" | cut -d '"' -f 4 | head -n 1)
                ;;
            deb)
                PKG_URL=$(echo "$LATEST_RELEASE" | grep "browser_download_url" | grep -i "\.deb\"" | cut -d '"' -f 4 | head -n 1)
                ;;
            pacman)
                PKG_URL=$(echo "$LATEST_RELEASE" | grep "browser_download_url" | grep -i "\.pkg\.tar\.zst\"" | cut -d '"' -f 4 | head -n 1)
                ;;
        esac

        if [ -n "$PKG_URL" ]; then
            echo "Detected $PKG_TYPE-based Linux distribution."
            echo "Found native $PKG_TYPE package: $PKG_URL"

            if [ "$(id -u)" = "0" ]; then
                SUDO=""
            elif command -v sudo >/dev/null 2>&1; then
                SUDO="sudo"
            else
                SUDO=""
            fi

            TMP_DIR=$(mktemp -d)
            PKG_FILE="$TMP_DIR/beru_pkg"

            echo "Downloading $PKG_URL..."
            if curl -sL "$PKG_URL" -o "$PKG_FILE"; then
                echo "Installing native $PKG_TYPE package..."
                case "$PKG_TYPE" in
                    rpm)
                        if command -v dnf >/dev/null 2>&1; then
                            if $SUDO dnf install -y "$PKG_FILE"; then
                                PKG_INSTALLED=true
                            fi
                        elif command -v zypper >/dev/null 2>&1; then
                            if $SUDO zypper --no-gpg-checks install -y "$PKG_FILE"; then
                                PKG_INSTALLED=true
                            fi
                        elif command -v yum >/dev/null 2>&1; then
                            if $SUDO yum install -y "$PKG_FILE"; then
                                PKG_INSTALLED=true
                            fi
                        elif command -v rpm >/dev/null 2>&1; then
                            if $SUDO rpm -Uvh --replacepkgs "$PKG_FILE"; then
                                PKG_INSTALLED=true
                            fi
                        fi
                        ;;
                    deb)
                        if command -v apt-get >/dev/null 2>&1; then
                            if $SUDO env DEBIAN_FRONTEND=noninteractive apt-get install -y "$PKG_FILE"; then
                                PKG_INSTALLED=true
                            fi
                        elif command -v dpkg >/dev/null 2>&1; then
                            if $SUDO dpkg -i "$PKG_FILE"; then
                                PKG_INSTALLED=true
                            fi
                        fi
                        ;;
                    pacman)
                        if command -v pacman >/dev/null 2>&1; then
                            if $SUDO pacman -U --noconfirm "$PKG_FILE"; then
                                PKG_INSTALLED=true
                            fi
                        fi
                        ;;
                esac
            fi

            rm -rf "$TMP_DIR"

            if [ "$PKG_INSTALLED" = "true" ]; then
                echo "Native $PKG_TYPE package installation complete!"
                echo "Running initial environment and prerequisite setup..."
                if command -v beru >/dev/null 2>&1; then
                    beru setup --install-prereqs || true
                elif [ -x /usr/bin/beru ]; then
                    /usr/bin/beru setup --install-prereqs || true
                fi
                echo "Beru is installed and ready to use."
                exit 0
            else
                echo "Notice: System $PKG_TYPE package installation could not be completed (e.g. insufficient permissions or package manager error)."
                echo "Falling back to standalone binary installation in \$HOME/.beru/bin..."
            fi
        fi
    fi
fi

# Fallback: standalone tarball installation into $HOME/.beru/bin
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

echo "Fetching standalone binary for $TARGET..."
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

INSTALL_DIR="$HOME/.beru/bin"
mkdir -p "$INSTALL_DIR"

echo "Installing to $INSTALL_DIR..."
mv "$TMP_DIR/$BIN" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$BIN"

rm -rf "$TMP_DIR"

# Attempt to add to PATH automatically in shell rc files
ADDED_TO_RC=false
for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
    if [ -f "$rc" ]; then
        if ! grep -q "$INSTALL_DIR" "$rc"; then
            echo "" >> "$rc"
            echo "# Added by Beru installer" >> "$rc"
            echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$rc"
            ADDED_TO_RC=true
        fi
    fi
done

if [ "$ADDED_TO_RC" = false ] && [ ! -f "$HOME/.bashrc" ] && [ ! -f "$HOME/.zshrc" ] && [ ! -f "$HOME/.profile" ]; then
    echo "# Added by Beru installer" >> "$HOME/.profile"
    echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$HOME/.profile"
fi

# Ensure INSTALL_DIR is in PATH for the current installer execution
export PATH="$INSTALL_DIR:$PATH"

echo "Installation complete!"
echo "Running initial environment and prerequisite setup..."
"$INSTALL_DIR/$BIN" setup --install-prereqs || true

echo "Please restart your terminal or run 'source ~/.bashrc' (or shell equivalent) to use Beru."
