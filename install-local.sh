#!/bin/bash

set -e

# Determine the local bin directory
LOCAL_BIN="$HOME/.local/bin"

echo "Building rcat in release mode..."
cargo build --release

# Create local bin directory if it doesn't exist
if [ ! -d "$LOCAL_BIN" ]; then
    echo "Creating $LOCAL_BIN directory..."
    mkdir -p "$LOCAL_BIN"
fi

echo "Installing to $LOCAL_BIN..."
cp target/release/rcat "$LOCAL_BIN/"

echo "Setting executable permissions..."
chmod +x "$LOCAL_BIN/rcat"

# Check if LOCAL_BIN is in PATH
if [[ ":$PATH:" != *":$LOCAL_BIN:"* ]]; then
    echo ""
    echo "WARNING: $LOCAL_BIN is not in your PATH."
    echo "Add the following line to your ~/.bashrc or ~/.zshrc:"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
fi

echo "Installation complete!"
echo "Usage: rcat <path>"

# Verify installation
if command -v rcat &> /dev/null; then
    echo "✓ rcat is available in your PATH"
else
    echo "⚠ rcat installed but not yet in PATH (restart your shell or source your rc file)"
fi