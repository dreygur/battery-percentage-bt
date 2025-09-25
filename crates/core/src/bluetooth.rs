use crate::{detect_device_type, ConnectionType, CoreError, Device};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use zbus::{Connection, Result as ZBusResult};

#[derive(Clone)]
pub struct BluetoothScanner {
    connection: Option<Connection>,
}

impl BluetoothScanner {
    pub fn new() -> Result<Self, CoreError> {
        Ok(Self { connection: None })
    }

    async fn get_connection(&mut self) -> Result<&Connection, CoreError> {
        if self.connection.is_none() {
            match Connection::system().await {
                Ok(conn) => {
                    info!("Established D-Bus system connection for Bluetooth scanning");
                    self.connection = Some(conn);
                }
                Err(e) => {
                    warn!("Failed to establish D-Bus connection: {}", e);
                    return Err(CoreError::DBusError(e));
                }
            }
        }
        Ok(self.connection.as_ref().unwrap())
    }

    pub async fn scan_devices(&mut self) -> Result<Vec<Device>, CoreError> {
        let mut devices = Vec::new();

        match self.get_bluetooth_devices().await {
            Ok(bt_devices) => {
                for (device_id, device_info) in bt_devices {
                    let device = self
                        .create_device_from_bt_info(device_id, device_info)
                        .await?;
                    devices.push(device);
                }
            }
            Err(e) => {
                warn!("Failed to get Bluetooth devices: {}", e);
                // Don't return error - just log and continue with empty list
                debug!("Bluetooth scanning failed, continuing with 0 devices");
            }
        }

        debug!("Found {} Bluetooth devices", devices.len());
        Ok(devices)
    }

    async fn get_bluetooth_devices(
        &mut self,
    ) -> Result<HashMap<String, BluetoothDeviceInfo>, CoreError> {
        let mut devices = HashMap::new();

        let connection = match self.get_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                debug!("No D-Bus connection available: {}", e);
                return Ok(devices); // Return empty list instead of error
            }
        };

        // Try to get Bluetooth adapter and devices
        match Self::scan_bluez_devices_static(connection).await {
            Ok(found_devices) => {
                devices.extend(found_devices);
                if !devices.is_empty() {
                    info!("Successfully scanned {} Bluetooth devices", devices.len());
                }
            }
            Err(e) => {
                debug!("Bluetooth scanning via BlueZ failed: {}", e);
                // This is normal if BlueZ is not running or no Bluetooth adapter
            }
        }

        Ok(devices)
    }

    async fn scan_bluez_devices_static(
        connection: &Connection,
    ) -> ZBusResult<HashMap<String, BluetoothDeviceInfo>> {
        let mut devices = HashMap::new();

        // Get the BlueZ object manager to enumerate all objects
        let object_manager = zbus::fdo::ObjectManagerProxy::builder(connection)
            .destination("org.bluez")?
            .path("/org/bluez")?
            .build()
            .await?;

        let managed_objects = object_manager.get_managed_objects().await?;

        for (object_path, interfaces) in managed_objects {
            // Look for Device1 interfaces (Bluetooth devices)
            if let Some(device_props) = interfaces.get("org.bluez.Device1") {
                let device_id = object_path.to_string();
                let mut device_info = BluetoothDeviceInfo::default();

                // Extract device name
                if let Some(name_variant) = device_props.get("Name") {
                    if let Some(name) = name_variant.downcast_ref::<str>() {
                        device_info.name = name.to_string();
                    }
                }

                // Skip devices without names (usually not useful)
                if device_info.name.is_empty() {
                    continue;
                }

                // Extract connection status
                if let Some(connected_variant) = device_props.get("Connected") {
                    if let Some(connected) = connected_variant.downcast_ref::<bool>() {
                        device_info.connected = *connected;
                    }
                }

                // Extract device class for type detection
                if let Some(class_variant) = device_props.get("Class") {
                    if let Some(class_val) = class_variant.downcast_ref::<u32>() {
                        device_info.device_class = Some(*class_val);
                    }
                }

                // Extract UUIDs for service identification
                if let Some(uuids_variant) = device_props.get("UUIDs") {
                    if let Some(uuids_array) = uuids_variant.downcast_ref::<zbus::zvariant::Array>()
                    {
                        device_info.uuids = uuids_array
                            .iter()
                            .filter_map(|v| v.downcast_ref::<str>().map(|s| s.to_string()))
                            .collect();
                    }
                }

                // Try to get battery level if device is connected
                if device_info.connected {
                    if let Ok(battery_level) =
                        Self::get_battery_level_static(connection, &object_path).await
                    {
                        device_info.battery_level = Some(battery_level);
                        debug!(
                            "Device {} has battery level: {}%",
                            device_info.name, battery_level
                        );
                    }
                }

                devices.insert(device_id, device_info);
            }
        }

        Ok(devices)
    }

    async fn get_battery_level_static(
        connection: &Connection,
        device_path: &zbus::zvariant::ObjectPath<'_>,
    ) -> ZBusResult<u8> {
        // Try to get battery information via Battery1 interface
        let properties_proxy = zbus::fdo::PropertiesProxy::builder(connection)
            .destination("org.bluez")?
            .path(device_path)?
            .build()
            .await?;

        // Get battery percentage from Battery1 interface
        match properties_proxy
            .get("org.bluez.Battery1".try_into().unwrap(), "Percentage")
            .await
        {
            Ok(percentage_variant) => {
                if let Some(percentage) = percentage_variant.downcast_ref::<u8>() {
                    return Ok(*percentage);
                }
            }
            Err(_) => {
                // Battery1 interface not available, which is normal for many devices
            }
        }

        Err(zbus::Error::InterfaceNotFound)
    }

    async fn create_device_from_bt_info(
        &self,
        device_id: String,
        info: BluetoothDeviceInfo,
    ) -> Result<Device, CoreError> {
        let device_type = detect_device_type(&info.name, info.device_class);

        let mut device =
            Device::with_id(device_id, info.name, device_type, ConnectionType::Bluetooth);

        device.set_connected(info.connected);
        device.update_battery(info.battery_level);

        Ok(device)
    }
}

#[derive(Debug, Default)]
struct BluetoothDeviceInfo {
    name: String,
    connected: bool,
    device_class: Option<u32>,
    uuids: Vec<String>,
    battery_level: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeviceType;

    #[test]
    fn test_device_type_detection() {
        assert_eq!(
            detect_device_type("Logitech Mouse", Some(0x002580)),
            DeviceType::Mouse
        );
        assert_eq!(
            detect_device_type("Apple Magic Keyboard", Some(0x002540)),
            DeviceType::Keyboard
        );
        assert_eq!(
            detect_device_type("Sony WH-1000XM4", Some(0x240404)),
            DeviceType::Headphones
        );
        assert_eq!(detect_device_type("AirPods Pro", None), DeviceType::Buds);
        assert_eq!(detect_device_type("iPhone 13", None), DeviceType::Mobile);
        assert_eq!(
            detect_device_type("Unknown Device", None),
            DeviceType::Unknown
        );
    }

    #[tokio::test]
    async fn test_bluetooth_scanner_creation() {
        let scanner = BluetoothScanner::new();
        assert!(scanner.is_ok());
    }
}
