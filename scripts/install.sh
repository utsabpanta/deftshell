#!/bin/sh
# DeftShell Install Script
# Usage: curl -fsSL https://deftshell.dev/install.sh | sh
set -e

REPO="deftshell/deftshell"
BINARY_NAME="ds"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin) OS="apple-darwin" ;;
    Linux)  OS="unknown-linux-gnu" ;;
    *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
    x86_64)  ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *)       echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

TARGET="${ARCH}-${OS}"

echo "Installing DeftShell for ${TARGET}..."

# Get latest release
LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"v\(.*\)".*/\1/')

if [ -z "$LATEST" ]; then
    echo "Failed to determine latest version"
    exit 1
fi

echo "Latest version: v${LATEST}"

# Download binary
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${LATEST}/ds-${TARGET}.tar.gz"
echo "Downloading from ${DOWNLOAD_URL}..."

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

curl -fsSL "$DOWNLOAD_URL" -o "${TMP_DIR}/ds.tar.gz"
tar xzf "${TMP_DIR}/ds.tar.gz" -C "$TMP_DIR"

# Install
if [ -w "$INSTALL_DIR" ]; then
    mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
else
    echo "Need sudo to install to ${INSTALL_DIR}"
    sudo mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
fi

chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo ""
echo "DeftShell installed successfully!"
echo ""
echo "Add this to your shell config:"
echo ""

SHELL_NAME=$(basename "$SHELL")
case "$SHELL_NAME" in
    zsh)  echo "  echo 'eval \"\$(ds init zsh)\"' >> ~/.zshrc" ;;
    bash) echo "  echo 'eval \"\$(ds init bash)\"' >> ~/.bashrc" ;;
    fish) echo "  echo 'ds init fish | source' >> ~/.config/fish/config.fish" ;;
    *)    echo "  eval \"\$(ds init $SHELL_NAME)\"" ;;
esac

echo ""
echo "Then restart your shell or run: source ~/.${SHELL_NAME}rc"
echo "Run 'ds help' to get started."
