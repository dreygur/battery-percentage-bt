#!/bin/bash

# GNOME Shell Extension Integration Script for Battery Status
# This script creates a simple indicator in the GNOME top bar

INDICATOR_FILE="/tmp/bluetooth-battery-status"
DESKTOP_FILE="$HOME/.local/share/applications/bluetooth-battery-monitor.desktop"
BINARY_PATH="/home/rakib/Code/battery_percentage/target/debug/bluetooth_only"

echo "Setting up GNOME integration for Bluetooth Battery Monitor..."

# Build the bluetooth_only binary first
echo "Building bluetooth_only binary..."
cd "/home/rakib/Code/battery_percentage"
cargo build --bin bluetooth_only

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Failed to build bluetooth_only binary"
    exit 1
fi

# Create desktop entry
mkdir -p "$HOME/.local/share/applications"
cat > "$DESKTOP_FILE" << EOF
[Desktop Entry]
Type=Application
Name=Bluetooth Battery Monitor
Exec=$BINARY_PATH
Icon=bluetooth
StartupNotify=false
NoDisplay=true
X-GNOME-Autostart-enabled=true
EOF

# Create systemd user service
SERVICE_FILE="$HOME/.config/systemd/user/bluetooth-battery-monitor.service"
mkdir -p "$HOME/.config/systemd/user"

cat > "$SERVICE_FILE" << EOF
[Unit]
Description=Bluetooth Battery Monitor
After=graphical-session.target bluetooth.target

[Service]
Type=simple
ExecStart=$BINARY_PATH
Restart=always
RestartSec=5
Environment=DISPLAY=:0
Environment=XDG_RUNTIME_DIR=%i

[Install]
WantedBy=default.target
EOF

echo "GNOME integration files created!"
echo ""
echo "Next steps:"
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
