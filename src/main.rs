mod bluetooth;
mod keyboard;

use bluer::{AdapterEvent, DeviceEvent, DiscoveryFilter, DiscoveryTransport};
use bluetooth::{BluetoothDevice, BluetoothManager};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use keyboard::KeyboardManager;
use std::process::Command;
use tokio::time::{sleep, Duration};

fn update_status_display(bt_manager: &BluetoothManager, kb_manager: &KeyboardManager) {
    let bt_status = bt_manager.get_status_text();
    let kb_status = kb_manager.get_status_text();

    let combined_status = if bt_status.contains("No Bluetooth") && kb_status.contains("No keyboards") {
        "No devices connected".to_string()
    } else if bt_status.contains("No Bluetooth") {
        kb_status
    } else if kb_status.contains("No keyboards") {
        bt_status
    } else {
        format!("{} | {}", kb_status, bt_status)
    };

    // Write to status file for GNOME integration
    let indicator_file = "/tmp/bluetooth-battery-status";
    let _ = std::fs::write(indicator_file, &combined_status);

    // Send desktop notification
    let has_battery_info = bt_manager.connected_devices.values().any(|d| d.battery_percentage.is_some()) ||
                          kb_manager.connected_keyboards.values().any(|k| k.battery_percentage.is_some());

    let notification_text = if has_battery_info {
        format!("🔋 {}", combined_status)
    } else {
        format!("📱 {}", combined_status)
    };

    let _ = Command::new("notify-send")
        .arg("Device Battery Status")
        .arg(&notification_text)
        .arg("-t")
        .arg("3000")
        .arg("-u")
        .arg("low")
        .output();

    println!("Status: {}", combined_status);
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting device battery monitor...");
    println!("Monitoring Bluetooth devices and keyboards for battery status");

    // Initialize managers
    let mut bt_manager = BluetoothManager::new();
    let mut kb_manager = match KeyboardManager::new() {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("Failed to initialize keyboard manager: {}", e);
            eprintln!("Continuing with Bluetooth-only monitoring...");
            // Create a fallback that will have no keyboards
            KeyboardManager::new().unwrap_or_else(|_| panic!("Failed to create fallback keyboard manager"))
        }
    };

    // Initial keyboard scan
    println!("Scanning for keyboards...");
    if let Err(e) = kb_manager.scan_for_keyboards() {
        eprintln!("Warning: Failed to scan keyboards: {}", e);
    }

    // Setup Bluetooth monitoring
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let filter = DiscoveryFilter {
        transport: DiscoveryTransport::Auto,
        ..Default::default()
    };

    adapter.set_discovery_filter(filter).await?;

    let device_events = adapter.discover_devices().await?;
    pin_mut!(device_events);

    let mut all_change_events = SelectAll::new();

    // Initial status update
    update_status_display(&bt_manager, &kb_manager);

    loop {
        tokio::select! {
            Some(device_event) = device_events.next() => {
                match device_event {
                    AdapterEvent::DeviceAdded(addr) => {
                        let device = adapter.device(addr)?;

                        if let Ok(Some(bt_device)) = BluetoothDevice::from_device(device.clone(), addr).await {
                            bt_manager.add_device(bt_device);
                            update_status_display(&bt_manager, &kb_manager);

                            let change_events = device.events().await?.map(move |evt| (addr, evt));
                            all_change_events.push(change_events);
                        }
                    }
                    AdapterEvent::DeviceRemoved(addr) => {
                        if bt_manager.remove_device(addr) {
                            update_status_display(&bt_manager, &kb_manager);
                        }
                    }
                    _ => (),
                }
            }
            Some((addr, DeviceEvent::PropertyChanged(_))) = all_change_events.next() => {
                if bt_manager.connected_devices.contains_key(&addr) {
                    let device = adapter.device(addr)?;

                    if device.is_connected().await.unwrap_or(false) {
                        if let Ok(Some(updated_device)) = BluetoothDevice::from_device(device, addr).await {
                            if bt_manager.update_device(addr, updated_device) {
                                update_status_display(&bt_manager, &kb_manager);
                            }
                        }
                    } else {
                        if bt_manager.remove_device(addr) {
                            update_status_display(&bt_manager, &kb_manager);
                        }
                    }
                }
            }
            _ = sleep(Duration::from_secs(30)) => {
                println!("Periodic update check...");

                // Update Bluetooth devices
                let mut bt_updated = false;
                let addresses: Vec<_> = bt_manager.connected_devices.keys().cloned().collect();
                for addr in addresses {
                    let device = adapter.device(addr)?;
                    if let Ok(Some(updated_device)) = BluetoothDevice::from_device(device, addr).await {
                        if bt_manager.update_device(addr, updated_device) {
                            bt_updated = true;
                        }
                    }
                }

                // Update keyboard batteries
                let kb_count_before = kb_manager.connected_keyboards.len();
                if let Err(e) = kb_manager.update_battery_levels() {
                    eprintln!("Warning: Failed to update keyboard batteries: {}", e);
                }

                // Rescan for new keyboards occasionally
                if kb_count_before == 0 {
                    if let Err(e) = kb_manager.scan_for_keyboards() {
                        eprintln!("Warning: Failed to rescan keyboards: {}", e);
                    }
                }

                if bt_updated || kb_count_before != kb_manager.connected_keyboards.len() {
                    update_status_display(&bt_manager, &kb_manager);
                }
            }
            _ = sleep(Duration::from_secs(120)) => {
                // Rescan for keyboards every 2 minutes
                println!("Rescanning for keyboards...");
                if let Err(e) = kb_manager.scan_for_keyboards() {
                    eprintln!("Warning: Failed to rescan keyboards: {}", e);
                }
                update_status_display(&bt_manager, &kb_manager);
            }
        }
    }
}
