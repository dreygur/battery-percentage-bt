# Battery Monitor

Keep track of your Bluetooth devices and keyboard battery levels right from your GNOME status bar. I built this because I got tired of my AirPods dying unexpectedly and my Ajazz AK870 keyboard running out of juice at the worst possible moments.

## What it does

This app monitors battery levels for your connected devices and shows them in your status bar with desktop notifications. It's particularly good at tracking the Ajazz AK870 keyboard, which can be tricky to monitor otherwise.

**Devices it tracks:**

- Bluetooth headphones, earbuds (like AirPods)
- Bluetooth mice and other peripherals
- Your phone when connected via Bluetooth
- The Ajazz AK870 keyboard (and other USB keyboards)
- Pretty much any Bluetooth device that reports battery info

## How it works

The app is split into two main parts:

- `bluetooth.rs` handles all the Bluetooth device discovery and battery monitoring
- `keyboard.rs` deals with USB keyboards, especially the AK870

It runs in the background and updates your status bar every 30 seconds, plus sends notifications when things change.

## Getting started

First, build the project:

```bash
cargo build --release
```

Then set up the GNOME integration:

```bash
./gnome-integration.sh
systemctl --user daemon-reload
systemctl --user enable bluetooth-battery-monitor.service
systemctl --user start bluetooth-battery-monitor.service
```

## Running it

You can run it manually to see what's happening:

```bash
./target/debug/battery_percentage
```

Or if you want just the status bar output:

```bash
./status-bar-reader.sh
```

## What you'll see

The app displays your devices like this:

- 🎧 Headphones: 85%
- ⌨️ AK870 Keyboard: 92%
- 📱 Phone: 78%

## About the AK870 keyboard support

Getting battery info from the Ajazz AK870 was surprisingly tricky. The keyboard uses device ID `05ac:024f` and doesn't always play nice with standard battery reporting.

I ended up using direct HID access through hidapi, which tries several different methods:

1. Scanning all HID devices to find keyboards
2. Looking for specific vendor/product IDs
3. Trying multiple battery report formats
4. Falling back to system interfaces when direct access fails

If your AK870 isn't being detected, try running with sudo first to rule out permission issues:

```bash
sudo ./target/debug/battery_percentage
```

You might also need to add yourself to the input group:

```bash
sudo usermod -a -G input $USER
```

## Troubleshooting

**AK870 not showing up?**

- Make sure it's plugged in via USB and powered on
- Check if `lsusb` shows device `05ac:024f`
- Try running `./target/debug/hid_test` to see if the device is detected
- Install `libudev-dev` if you haven't: `sudo apt install libudev-dev pkg-config`

**Bluetooth devices missing?**

- Check that BlueZ is running: `systemctl status bluetooth`
- Make sure your devices are actually connected (not just paired)
- Some devices only report battery when actively being used

**GNOME integration not working?**

- You'll need a GNOME extension like "Generic Monitor" to display the status
- Check that the app can write to `/tmp/bluetooth-battery-status`
- Make sure notification permissions are enabled

## Technical details

The app checks Bluetooth devices in real-time when they connect/disconnect, plus does a full scan every 30 seconds. Keyboards get rescanned every 2 minutes since they're more stable connections.

Battery readings for the AK870 use multiple fallback methods because the keyboard's HID implementation is a bit quirky. It tries feature reports, input reports, and system power supply interfaces until something works.

## Dependencies

You'll need:

- Rust and Cargo (for building)
- BlueZ (for Bluetooth)
- A GNOME desktop with extensions support
- `libudev-dev` and `pkg-config` (for HID access)

## Want to add support for your device?

The code is pretty modular. To add a new keyboard:

1. Add detection logic in `keyboard.rs`
2. Figure out how to read its battery (good luck!)
3. Add it to the `KeyboardType` enum
4. Update the detection function

Feel free to submit PRs if you get other devices working!

## License

MIT - do whatever you want with it.
