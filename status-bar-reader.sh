#!/bin/bash

# Simple status bar reader for GNOME
# This script can be used with extensions like "Generic Monitor" to display battery info

STATUS_FILE="/tmp/bluetooth-battery-status"

if [ -f "$STATUS_FILE" ]; then
    cat "$STATUS_FILE"
else
    echo "No devices"
fi
