#!/bin/bash

# create-release.sh - Create a release tarball with compiled binaries and scripts

set -e  # Exit on any error

# Configuration
PACKAGE_NAME="battery_percentage"
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
cp scripts/gnome-integration.sh "$RELEASE_DIR/$PACKAGE_NAME/"
cp scripts/install-status-bar.sh "$RELEASE_DIR/$PACKAGE_NAME/"
cp scripts/status-bar-reader.sh "$RELEASE_DIR/$PACKAGE_NAME/"

# Make shell scripts executable
chmod +x "$RELEASE_DIR/$PACKAGE_NAME/gnome-integration.sh"
chmod +x "$RELEASE_DIR/$PACKAGE_NAME/install-status-bar.sh"
chmod +x "$RELEASE_DIR/$PACKAGE_NAME/status-bar-reader.sh"

# Copy README if it exists
if [ -f "README.md" ]; then
    echo -e "${YELLOW}Copying README.md...${NC}"
    cp README.md "$RELEASE_DIR/$PACKAGE_NAME/"
fi

# Create install script
echo -e "${YELLOW}Creating install script...${NC}"

# Create the basic install script first
cat > "$RELEASE_DIR/$PACKAGE_NAME/install.sh" << 'INSTALL_EOF'
#!/bin/bash

# Install script for battery_percentage with GNOME integration

set -e

INSTALL_DIR="$HOME/.local/bin"
SCRIPT_DIR="$HOME/.local/share/battery_percentage"
INDICATOR_FILE="/tmp/bluetooth-battery-status"
DESKTOP_FILE="$HOME/.local/share/applications/bluetooth-battery-monitor.desktop"

echo "Installing battery_percentage..."

# Create directories if they don't exist
mkdir -p "$INSTALL_DIR"
mkdir -p "$SCRIPT_DIR"

# Copy binaries
cp battery_percentage "$INSTALL_DIR/"
cp bluetooth_only "$INSTALL_DIR/"

# Copy scripts
cp gnome-integration.sh "$SCRIPT_DIR/"
cp install-status-bar.sh "$SCRIPT_DIR/"
cp status-bar-reader.sh "$SCRIPT_DIR/"

echo "Installation completed!"
echo "Binaries installed to: $INSTALL_DIR"
echo "Scripts installed to: $SCRIPT_DIR"
echo ""

# GNOME Integration Setup
echo "Setting up GNOME integration..."

# Create desktop entry
mkdir -p "$HOME/.local/share/applications"
INSTALL_EOF

# Add desktop file creation to install script
cat >> "$RELEASE_DIR/$PACKAGE_NAME/install.sh" << 'DESKTOP_SCRIPT_EOF'
cat > "$DESKTOP_FILE" << 'DESKTOP_FILE_EOF'
[Desktop Entry]
Type=Application
Name=Bluetooth Battery Monitor
Exec=$INSTALL_DIR/bluetooth_only
Icon=bluetooth
StartupNotify=false
NoDisplay=true
X-GNOME-Autostart-enabled=true
DESKTOP_FILE_EOF

# Create systemd user service
SERVICE_FILE="$HOME/.config/systemd/user/bluetooth-battery-monitor.service"
mkdir -p "$HOME/.config/systemd/user"

DESKTOP_SCRIPT_EOF

# Add systemd service creation to install script
cat >> "$RELEASE_DIR/$PACKAGE_NAME/install.sh" << 'SERVICE_SCRIPT_EOF'
cat > "$SERVICE_FILE" << 'SERVICE_FILE_EOF'
[Unit]
Description=Bluetooth Battery Monitor
After=graphical-session.target bluetooth.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/bluetooth_only
Restart=always
RestartSec=5
Environment=DISPLAY=:0
Environment=XDG_RUNTIME_DIR=%i

[Install]
WantedBy=default.target
SERVICE_FILE_EOF

SERVICE_SCRIPT_EOF

# Add final instructions to install script
cat >> "$RELEASE_DIR/$PACKAGE_NAME/install.sh" << 'FINAL_EOF'

echo ""
echo "GNOME integration files created!"
echo ""
echo "Installation Summary:"
echo "- Binaries installed to: $INSTALL_DIR"
echo "- Scripts installed to: $SCRIPT_DIR"
echo "- Desktop entry created: $DESKTOP_FILE"
echo "- Systemd service created: $SERVICE_FILE"
echo ""
echo "To complete GNOME integration:"
echo "1. Install a GNOME Shell extension to display custom text in the status bar:"
echo "   - 'Executor' extension: https://extensions.gnome.org/extension/2932/executor/"
echo "   - Or 'Generic Monitor' extension: https://extensions.gnome.org/extension/3968/generic-monitor/"
echo ""
echo "2. Enable and start the background service:"
echo "   systemctl --user daemon-reload"
echo "   systemctl --user enable bluetooth-battery-monitor.service"
echo "   systemctl --user start bluetooth-battery-monitor.service"
echo ""
echo "3. Configure the GNOME extension to read from: $INDICATOR_FILE"
echo "   Command: cat $INDICATOR_FILE"
echo "   Refresh interval: 30 seconds"
echo ""
echo "4. Check service status with:"
echo "   systemctl --user status bluetooth-battery-monitor.service"
FINAL_EOF

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
