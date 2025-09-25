# Battery Monitor for Linux

A comprehensive battery monitoring application for Linux that tracks battery levels of Bluetooth devices, USB keyboards, and wireless 2.4GHz peripherals. Features system tray integration, configurable notifications, and cross-desktop environment compatibility.

## Features

- **Device Detection**: Automatically detects Bluetooth devices and USB/wireless keyboards with battery information
- **System Tray Integration**: Displays battery levels directly in the system tray across all desktop environments
- **Smart Notifications**: Configurable low battery alerts with spam prevention
- **Cross-Platform**: Works on GNOME, KDE, XFCE, and other Linux desktop environments
- **No Root Required**: Operates with standard user permissions
- **Configurable**: TOML-based configuration with validation
- **Daemon Mode**: Can run as a background service without GUI

## Supported Devices

- Bluetooth mice, keyboards, headphones, earbuds, phones, and tablets
- USB HID devices (keyboards, mice) with battery capability
- Wireless 2.4GHz peripherals
- Any device exposing battery information via `/sys/class/power_supply/`

## Installation

### Prerequisites

- Rust 1.70+ (latest stable recommended)
- GTK4 development libraries and dependencies
- D-Bus system access (standard on most Linux distributions)
- pkg-config for library detection

#### Ubuntu/Debian

```bash
# Install build essentials and GTK4 development packages
sudo apt update
sudo apt install -y build-essential pkg-config

# Install GTK4 and related development libraries
sudo apt install -y libgtk-4-dev libglib2.0-dev libcairo2-dev \
                    libpango1.0-dev libgdk-pixbuf2.0-dev \
                    libgraphene-1.0-dev libdbus-1-dev

# Optional: Additional dependencies that may be required
sudo apt install -y libgio-2.0-dev gobject-introspection \
                    libgirepository1.0-dev
```

#### Fedora/RHEL

```bash
# Install build essentials
sudo dnf groupinstall "Development Tools"
sudo dnf install pkg-config

# Install GTK4 and related development libraries
sudo dnf install gtk4-devel glib2-devel cairo-devel \
                  pango-devel gdk-pixbuf2-devel \
                  graphene-devel dbus-devel
```

#### Arch Linux

```bash
# Install base development packages
sudo pacman -S base-devel pkgconf

# Install GTK4 and related libraries
sudo pacman -S gtk4 glib2 cairo pango gdk-pixbuf2 \
               graphene dbus
```

### Verifying Prerequisites

After installing the prerequisites, verify that GTK4 is properly detected:

```bash
pkg-config --modversion gtk4
```

This should output a version number (4.0.0 or higher). If it fails, ensure the development packages are installed correctly.

### Building from Source

1. Clone the repository:

```bash
git clone <repository-url>
cd battery_percentage
```

2. Verify that all prerequisites are installed:

```bash
# Test that GTK4 development libraries are available
pkg-config --exists gtk4 && echo "GTK4 detected" || echo "GTK4 not found - install prerequisites first"
```

3. Build the application:

```bash
# Build in release mode for optimal performance
cargo build --release

# Or build in debug mode for development
cargo build
```

4. Test the build (optional):

```bash
# Run tests to ensure everything works
cargo test

# Check for any linting issues
cargo clippy

# Verify formatting
cargo fmt --check
```

5. Install (optional):

```bash
# Install the main binary
cargo install --path crates/main

# Or run directly from the build directory
./target/release/battery-monitor
```

The binary will be available as `battery-monitor`.

## Usage

### GUI Mode (Default)

```bash
battery-monitor
```

### Daemon Mode (No GUI)

```bash
battery-monitor --daemon
```

### Command Line Options

```bash
# Show help
battery-monitor --help

# Run with verbose logging
battery-monitor --verbose

# Run quietly (errors only)
battery-monitor --quiet

# Show configuration file location
battery-monitor --show-config

# Validate configuration
battery-monitor --check-config

# Print default configuration
battery-monitor --print-default-config

# Reset configuration to defaults
battery-monitor --reset-config
```

## Configuration

Configuration is stored in `~/.config/battery-monitor/config.toml` and follows XDG standards.

### Default Configuration

```toml
[monitoring]
polling_interval_seconds = 30
auto_start = false

[notifications]
enabled = true
low_battery_threshold = 20
show_connect_disconnect = true
suppression_minutes = 5

[ui]
show_disconnected_devices = true
```

### Configuration Options

#### Monitoring Section

- `polling_interval_seconds` (5-300): How often to check device battery levels
- `auto_start` (boolean): Start automatically with desktop session

#### Notifications Section

- `enabled` (boolean): Enable/disable all notifications
- `low_battery_threshold` (1-99): Battery percentage threshold for alerts
- `show_connect_disconnect` (boolean): Show device connection/disconnection notifications
- `suppression_minutes` (1-60): Minutes to suppress repeat notifications

#### UI Section

- `show_disconnected_devices` (boolean): Show disconnected devices in details view

## Architecture

The application is built using a modular architecture with separate crates organized under the `crates/` directory:

```
battery-monitor/
├── crates/
│   ├── core/           # Device detection & battery reading logic
│   ├── gui/            # System tray & settings interface (GTK4-rs)
│   ├── notifications/  # Alert system with suppression & logging
│   ├── config/         # Configuration management (TOML)
│   └── main/           # Binary entry point & orchestration
├── target/             # Build artifacts
└── README.md           # This file
```

### Core Components

- **Device Monitor**: Scans for devices using D-Bus (Bluetooth) and sysfs (USB/wireless)
- **GUI System**: GTK4-based system tray with popover details and settings dialog
- **Notification Manager**: Desktop notifications with intelligent suppression
- **Config Manager**: TOML configuration with validation and XDG compliance

## Development

### Project Structure

```
├── crates/
│   ├── core/src/
│   │   ├── lib.rs              # Core data structures and traits
│   │   ├── device_monitor.rs   # Main monitoring logic
│   │   ├── bluetooth.rs        # Bluetooth device detection
│   │   └── usb.rs             # USB/wireless device detection
│   ├── gui/src/
│   │   ├── lib.rs              # GUI framework and utilities
│   │   ├── tray.rs            # System tray implementation
│   │   ├── details.rs         # Device details window
│   │   └── settings.rs        # Settings dialog
│   ├── notifications/src/
│   │   └── lib.rs              # Notification management
│   ├── config/src/
│   │   └── lib.rs              # Configuration handling
│   └── main/src/
│       ├── main.rs            # Application entry point
│       ├── app.rs             # Main application logic
│       └── cli.rs             # Command line interface
├── target/                     # Build artifacts (generated)
└── Cargo.toml                  # Workspace configuration
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p battery-monitor-core

# Run with output
cargo test -- --nocapture
```

### Building Documentation

```bash
cargo doc --open
```

## System Integration

### Auto-start Setup

Create a desktop entry for auto-start:

```bash
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/battery-monitor.desktop << EOF
[Desktop Entry]
Type=Application
Exec=battery-monitor
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
Name=Battery Monitor
Comment=Monitor battery levels of connected devices
EOF
```

### Systemd User Service

For daemon mode:

```bash
cat > ~/.config/systemd/user/battery-monitor.service << EOF
[Unit]
Description=Battery Monitor Service
After=graphical-session.target

[Service]
Type=exec
ExecStart=/path/to/battery-monitor --daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

systemctl --user enable battery-monitor.service
systemctl --user start battery-monitor.service
```

## Troubleshooting

### Common Issues

1. **No devices detected**
   - Ensure devices are paired and connected
   - Check Bluetooth service: `systemctl status bluetooth`
   - Verify D-Bus access: `dbus-send --system --print-reply --dest=org.bluez / org.freedesktop.DBus.Introspectable.Introspect`

2. **Permission denied errors**
   - Application runs with standard user permissions
   - Ensure user is in `bluetooth` group: `sudo usermod -a -G bluetooth $USER`

3. **GUI not showing**
   - Verify GTK4 installation: `pkg-config --modversion gtk4`
   - Check desktop environment tray support
   - Try running with `--verbose` flag

4. **High CPU usage**
   - Increase polling interval in configuration
   - Check for D-Bus connection issues in logs

### Build Issues

**GTK4 development libraries not found:**

```bash
error: The system library `gtk4` required by crate `gdk4-sys` was not found.
```

**Solution:** Install the GTK4 development packages as described in the Prerequisites section.

**pkg-config not found:**

```bash
error: failed to run custom build command for `glib-sys`
```

**Solution:** Install pkg-config and ensure it's in your PATH:

```bash
# Ubuntu/Debian
sudo apt install pkg-config

# Fedora/RHEL
sudo dnf install pkg-config

# Arch Linux
sudo pacman -S pkgconf
```

**Permissions issues with D-Bus:**

```bash
Permission denied errors when accessing bluetooth devices
```

**Solution:** Add your user to the bluetooth group:

```bash
sudo usermod -a -G bluetooth $USER
# Log out and back in for changes to take effect
```

### Runtime Logging

Enable verbose logging:

```bash
battery-monitor --verbose
```

Log files are written to stdout/stderr. For persistent logging:

```bash
battery-monitor --daemon 2>&1 | logger -t battery-monitor
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Run `cargo test` and `cargo clippy`
6. Submit a pull request

### Code Style

- Follow Rust standard formatting: `cargo fmt`
- Address all clippy warnings: `cargo clippy`
- Include comprehensive error handling
- Write unit tests for new features
- Document public APIs

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with [GTK4-rs](https://gtk-rs.org/) for cross-desktop compatibility
- Uses [notify-rust](https://github.com/hoodie/notify-rust) for desktop notifications
- Bluetooth integration via [zbus](https://github.com/dbus2/zbus)
- Configuration management with [serde](https://serde.rs/) and [toml](https://github.com/toml-rs/toml)

## Changelog

### v0.1.0 (Initial Release)

- Core device detection for Bluetooth and USB devices
- GTK4-based system tray integration
- Configurable notifications with suppression
- TOML-based configuration system
- Cross-desktop environment support
- Daemon mode operation
