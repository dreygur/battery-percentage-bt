use battery_percentage::bluetooth::{BluetoothDevice, BluetoothManager};
use bluer::{AdapterEvent, Session};
use futures::StreamExt;
use std::time::Duration;
use tokio::time::interval;

#[tokio::main]
async fn main() -> bluer::Result<()> {
    println!("Starting Bluetooth-only battery monitor...");

    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let mut bluetooth_manager = BluetoothManager::new();
    let mut discover_events = adapter.discover_devices().await?;
    let mut status_interval = interval(Duration::from_secs(30));

    // Initial scan for connected devices
    scan_connected_devices(&adapter, &mut bluetooth_manager).await?;

    loop {
        tokio::select! {
            Some(evt) = discover_events.next() => {
                handle_adapter_event(evt, &adapter, &mut bluetooth_manager).await;
            }
            _ = status_interval.tick() => {
                update_device_status(&adapter, &mut bluetooth_manager).await;
                print_status(&bluetooth_manager);
            }
        }
    }
}

async fn scan_connected_devices(
    adapter: &bluer::Adapter,
    bluetooth_manager: &mut BluetoothManager,
) -> bluer::Result<()> {
    let device_list = adapter.device_addresses().await?;

    for addr in device_list {
        let device = adapter.device(addr)?;
        if device.is_connected().await? {
            if let Ok(Some(bt_device)) = BluetoothDevice::from_device(device, addr).await {
                bluetooth_manager.add_device(bt_device);
            }
        }
    }

    Ok(())
}

async fn handle_adapter_event(
    evt: AdapterEvent,
    adapter: &bluer::Adapter,
    bluetooth_manager: &mut BluetoothManager,
) {
    match evt {
        AdapterEvent::DeviceAdded(addr) => {
            let device = match adapter.device(addr) {
                Ok(device) => device,
                Err(e) => {
                    eprintln!("Failed to get device {}: {}", addr, e);
                    return;
                }
            };

            match BluetoothDevice::from_device(device, addr).await {
                Ok(Some(bt_device)) => {
                    bluetooth_manager.add_device(bt_device);
                }
                Ok(None) => {
                    // Device not connected, ignore
                }
                Err(e) => {
                    eprintln!("Error processing device {}: {}", addr, e);
                }
            }
        }
        AdapterEvent::DeviceRemoved(addr) => {
            bluetooth_manager.remove_device(addr);
        }
        _ => {}
    }
}

async fn update_device_status(
    adapter: &bluer::Adapter,
    bluetooth_manager: &mut BluetoothManager,
) {
    let addresses: Vec<_> = bluetooth_manager.connected_devices.keys().copied().collect();

    for addr in addresses {
        let device = match adapter.device(addr) {
            Ok(device) => device,
            Err(_) => continue,
        };

        match BluetoothDevice::from_device(device, addr).await {
            Ok(Some(updated_device)) => {
                bluetooth_manager.update_device(addr, updated_device);
            }
            Ok(None) => {
                // Device disconnected
                bluetooth_manager.remove_device(addr);
            }
            Err(e) => {
                eprintln!("Error updating device {}: {}", addr, e);
            }
        }
    }
}

fn print_status(bluetooth_manager: &BluetoothManager) {
    let status = bluetooth_manager.get_status_text();
    println!("Status: {}", status);

    // Write status to file for status bar integration
    if let Err(e) = std::fs::write("/tmp/bluetooth-battery-status", &status) {
        eprintln!("Failed to write status file: {}", e);
    }
}
