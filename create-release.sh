#!/bin/bash

# create-release.sh - Create a release tarball with compiled binaries and scripts

set -e  # Exit on any error

# Configuration
PACKAGE_NAME="bettery_percentage"
VERSION=$(grep '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
RELEASE_DIR="release"
TARBALL_NAME="${PACKAGE_NAME}-${VERSION}-linux-x86_64.tar.gz"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Creating release tarball for ${PACKAGE_NAME} v${VERSION}${NC}"

# Clean up any existing release directory
if [ -d "$RELEASE_DIR" ]; then
    echo -e "${YELLOW}Cleaning up existing release directory...${NC}"
    rm -rf "$RELEASE_DIR"
fi

# Create release directory structure
echo -e "${YELLOW}Creating release directory structure...${NC}"
mkdir -p "$RELEASE_DIR/$PACKAGE_NAME"
cd "$RELEASE_DIR/$PACKAGE_NAME"

# Build the project in release mode
echo -e "${YELLOW}Building project in release mode...${NC}"
cd ../..
cargo build --release

# Check if binaries were built successfully
if [ ! -f "target/release/${PACKAGE_NAME}" ]; then
    echo -e "${RED}Error: Main binary ${PACKAGE_NAME} not found in target/release/${NC}"
    exit 1
fi

if [ ! -f "target/release/bluetooth_only" ]; then
    echo -e "${RED}Error: bluetooth_only binary not found in target/release/${NC}"
    exit 1
fi

# Copy binaries to release directory
echo -e "${YELLOW}Copying compiled binaries...${NC}"
cp "target/release/${PACKAGE_NAME}" "$RELEASE_DIR/$PACKAGE_NAME/"
cp "target/release/bluetooth_only" "$RELEASE_DIR/$PACKAGE_NAME/"

# Make binaries executable
chmod +x "$RELEASE_DIR/$PACKAGE_NAME/${PACKAGE_NAME}"
chmod +x "$RELEASE_DIR/$PACKAGE_NAME/bluetooth_only"

# Copy shell scripts
echo -e "${YELLOW}Copying shell scripts...${NC}"
cp gnome-integration.sh "$RELEASE_DIR/$PACKAGE_NAME/"
cp install-status-bar.sh "$RELEASE_DIR/$PACKAGE_NAME/"
cp status-bar-reader.sh "$RELEASE_DIR/$PACKAGE_NAME/"

# Make shell scripts executable
chmod +x "$RELEASE_DIR/$PACKAGE_NAME/gnome-integration.sh"
chmod +x "$RELEASE_DIR/$PACKAGE_NAME/install-status-bar.sh"
chmod +x "$RELEASE_DIR/$PACKAGE_NAME/status-bar-reader.sh"

# Copy README if it exists
if [ -f "README.md" ]; then
    echo -e "${YELLOW}Copying README.md...${NC}"
    cp README.md "$RELEASE_DIR/$PACKAGE_NAME/"
fi

# Create a simple install script
echo -e "${YELLOW}Creating install script...${NC}"
cat > "$RELEASE_DIR/$PACKAGE_NAME/install.sh" << 'EOF'
#!/bin/bash

# Simple install script for bettery_percentage

set -e

INSTALL_DIR="$HOME/.local/bin"
SCRIPT_DIR="$HOME/.local/share/bettery_percentage"

echo "Installing bettery_percentage..."

# Create directories if they don't exist
mkdir -p "$INSTALL_DIR"
mkdir -p "$SCRIPT_DIR"

# Copy binaries
cp bettery_percentage "$INSTALL_DIR/"
cp bluetooth_only "$INSTALL_DIR/"

# Copy scripts
cp gnome-integration.sh "$SCRIPT_DIR/"
cp install-status-bar.sh "$SCRIPT_DIR/"
cp status-bar-reader.sh "$SCRIPT_DIR/"

echo "Installation completed!"
echo "Binaries installed to: $INSTALL_DIR"
echo "Scripts installed to: $SCRIPT_DIR"
echo ""
echo "Make sure $INSTALL_DIR is in your PATH to use the binaries."
echo "You can add it by running:"
echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
echo "  source ~/.bashrc"
EOF

chmod +x "$RELEASE_DIR/$PACKAGE_NAME/install.sh"

# Create the tarball
echo -e "${YELLOW}Creating tarball...${NC}"
cd "$RELEASE_DIR"
tar -czf "../$TARBALL_NAME" "$PACKAGE_NAME"
cd ..

# Clean up release directory
rm -rf "$RELEASE_DIR"

# Display results
echo -e "${GREEN}Release tarball created successfully!${NC}"
echo -e "${GREEN}File: ${TARBALL_NAME}${NC}"
echo -e "${GREEN}Size: $(du -h "$TARBALL_NAME" | cut -f1)${NC}"

# Show contents
echo -e "${YELLOW}Tarball contents:${NC}"
tar -tzf "$TARBALL_NAME"

echo -e "${GREEN}Release creation completed!${NC}"
