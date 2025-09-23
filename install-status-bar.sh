#!/bin/bash

# Complete installation script for Ubuntu 24.04 status bar integration

set -e

PROJECT_DIR="/home/rakib/Code/battery_percentage"
BINARY_NAME="bluetooth_only"
BINARY_PATH="$PROJECT_DIR/target/debug/$BINARY_NAME"
SERVICE_NAME="bluetooth-battery-monitor"

echo "🔋 Installing Bluetooth Battery Monitor for Ubuntu 24.04 Status Bar"
echo "=================================================================="

# Step 1: Build the project
echo "📦 Building the project..."
cd "$PROJECT_DIR"
cargo build --bin "$BINARY_NAME"

if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Error: Failed to build $BINARY_NAME binary"
    exit 1
fi
echo "✅ Binary built successfully"

# Step 2: Run the GNOME integration script
echo "🔧 Setting up GNOME integration..."
bash "$PROJECT_DIR/gnome-integration.sh"

# Step 3: Enable and start the service
echo "🚀 Enabling and starting the background service..."
systemctl --user daemon-reload
systemctl --user enable "$SERVICE_NAME.service"
systemctl --user start "$SERVICE_NAME.service"

# Step 4: Check if service is running
sleep 2
if systemctl --user is-active --quiet "$SERVICE_NAME.service"; then
    echo "✅ Service is running"
else
    echo "⚠️  Service may not be running properly. Check with:"
    echo "   systemctl --user status $SERVICE_NAME.service"
fi

# Step 5: Test the status file
echo "🧪 Testing status file generation..."
sleep 5
if [ -f "/tmp/bluetooth-battery-status" ]; then
    echo "✅ Status file created: /tmp/bluetooth-battery-status"
    echo "📄 Current content:"
    cat "/tmp/bluetooth-battery-status"
else
    echo "⚠️  Status file not found yet. It may take a moment to appear."
fi

echo ""
echo "🎉 Installation Complete!"
echo "========================"
echo ""
echo "📋 Next Steps:"
echo "1. Install a GNOME Shell extension to display the battery info:"
echo "   • Open https://extensions.gnome.org/extension/2932/executor/ in Firefox"
echo "   • Click 'Install' to add the Executor extension"
echo "   • Or alternatively use Generic Monitor: https://extensions.gnome.org/extension/3968/generic-monitor/"
echo ""
echo "2. Configure the extension:"
echo "   • Command: cat /tmp/bluetooth-battery-status"
echo "   • Interval: 30 seconds"
echo "   • Enable the extension in GNOME Extensions app"
echo ""
echo "3. Monitor the service:"
echo "   • Check status: systemctl --user status $SERVICE_NAME.service"
echo "   • View logs: journalctl --user -u $SERVICE_NAME.service -f"
echo "   • Stop service: systemctl --user stop $SERVICE_NAME.service"
echo ""
echo "📱 The battery percentages will appear in your top status bar!"
