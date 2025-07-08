#!/bin/bash

# Voquill Build Script
# Builds binaries for different platforms

set -e

# Create bin directory if it doesn't exist
mkdir -p bin

# Build for current platform
echo "Building for current platform..."
cd src
go build -o ../bin/voquill .
cd ..

# Build for Linux (if not already Linux)
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    echo "Cross-compiling for Linux..."
    cd src
    CGO_ENABLED=1 GOOS=linux GOARCH=amd64 go build -o ../bin/voquill-linux .
    cd ..
fi

# Build for Windows (requires mingw-w64 for cross-compilation)
if command -v x86_64-w64-mingw32-gcc &> /dev/null; then
    echo "Cross-compiling for Windows..."
    cd src
    CGO_ENABLED=1 GOOS=windows GOARCH=amd64 CC=x86_64-w64-mingw32-gcc go build -o ../bin/voquill.exe .
    cd ..
else
    echo "Skipping Windows build (mingw-w64 not found)"
fi

echo "Build complete! Binaries are in the bin/ directory:"
ls -la bin/
