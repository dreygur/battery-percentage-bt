use crate::{Device, ConnectionType, CoreError, detect_device_type};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, warn};

#[derive(Clone)]
pub struct UsbScanner;

impl UsbScanner {
    pub fn new() -> Self {
        Self
    }

    pub async fn scan_devices(&self) -> Result<Vec<Device>, CoreError> {
        let mut devices = Vec::new();

        match self.scan_power_supply_devices().await {
            Ok(mut power_devices) => {
                devices.append(&mut power_devices);
            }
            Err(e) => {
                warn!("Failed to scan power supply devices: {}", e);
            }
        }

        match self.scan_usb_hid_devices().await {
            Ok(mut usb_devices) => {
                devices.append(&mut usb_devices);
            }
            Err(e) => {
                warn!("Failed to scan USB HID devices: {}", e);
            }
        }

        debug!("Found {} USB/power devices", devices.len());
        Ok(devices)
    }

    async fn scan_power_supply_devices(&self) -> Result<Vec<Device>, CoreError> {
        let mut devices = Vec::new();
        let power_supply_path = PathBuf::from("/sys/class/power_supply");

        if !power_supply_path.exists() {
            return Ok(devices);
        }

        let entries = fs::read_dir(&power_supply_path)
            .map_err(|e| CoreError::SystemApiError(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| CoreError::SystemApiError(e))?;
            let device_path = entry.path();
            let device_name = entry.file_name().to_string_lossy().to_string();

            if let Ok(device) = self.create_power_supply_device(&device_path, &device_name).await {
                if device.battery_level.is_some() {
                    devices.push(device);
                }
            }
        }

        Ok(devices)
    }

    async fn create_power_supply_device(&self, device_path: &PathBuf, device_name: &str) -> Result<Device, CoreError> {
        let type_path = device_path.join("type");
        let capacity_path = device_path.join("capacity");
        let status_path = device_path.join("status");
        let model_name_path = device_path.join("model_name");

        let device_type_str = fs::read_to_string(&type_path)
            .map_err(|e| CoreError::SystemApiError(e))?
            .trim()
            .to_string();

        if device_type_str == "Battery" {
            let capacity = if capacity_path.exists() {
                let capacity_str = fs::read_to_string(&capacity_path)
                    .map_err(|e| CoreError::SystemApiError(e))?;
                capacity_str.trim().parse::<u8>().ok()
            } else {
                None
            };

            let status = if status_path.exists() {
                fs::read_to_string(&status_path)
                    .map_err(|e| CoreError::SystemApiError(e))?
                    .trim()
                    .to_string()
            } else {
                "Unknown".to_string()
            };

            let model_name = if model_name_path.exists() {
                fs::read_to_string(&model_name_path)
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            } else {
                device_name.to_string()
            };

            let display_name = if model_name.is_empty() {
                device_name.to_string()
            } else {
                model_name
            };

            let device_type = detect_device_type(&display_name, None);
            let connection_type = if device_name.contains("hid") || device_name.contains("usb") {
                ConnectionType::USB
            } else if device_name.contains("wireless") || device_name.contains("2.4g") {
                ConnectionType::Wireless2_4G
            } else {
                ConnectionType::USB
            };

            let mut device = Device::with_id(
                format!("power_supply_{}", device_name),
                display_name,
                device_type,
                connection_type,
            );

            device.update_battery(capacity);
            device.set_connected(status == "Discharging" || status == "Charging" || status == "Full");

            return Ok(device);
        }

        Err(CoreError::DeviceDetectionFailed("Not a battery device".to_string()))
    }

    async fn scan_usb_hid_devices(&self) -> Result<Vec<Device>, CoreError> {
        let mut devices = Vec::new();
        let usb_devices_path = PathBuf::from("/sys/bus/usb/devices");

        if !usb_devices_path.exists() {
            return Ok(devices);
        }

        let entries = fs::read_dir(&usb_devices_path)
            .map_err(|e| CoreError::SystemApiError(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| CoreError::SystemApiError(e))?;
            let device_path = entry.path();

            if let Ok(device) = self.create_usb_hid_device(&device_path).await {
                devices.push(device);
            }
        }

        Ok(devices)
    }

    async fn create_usb_hid_device(&self, device_path: &PathBuf) -> Result<Device, CoreError> {
        let product_path = device_path.join("product");
        let manufacturer_path = device_path.join("manufacturer");
        let bdev_class_path = device_path.join("bDeviceClass");

        if !product_path.exists() {
            return Err(CoreError::DeviceDetectionFailed("No product info".to_string()));
        }

        let product = fs::read_to_string(&product_path)
            .map_err(|e| CoreError::SystemApiError(e))?
            .trim()
            .to_string();

        let manufacturer = if manufacturer_path.exists() {
            fs::read_to_string(&manufacturer_path)
                .unwrap_or_default()
                .trim()
                .to_string()
        } else {
            String::new()
        };

        let device_class = if bdev_class_path.exists() {
            fs::read_to_string(&bdev_class_path)
                .ok()
                .and_then(|s| u32::from_str_radix(s.trim(), 16).ok())
        } else {
            None
        };

        if device_class == Some(0x03) {
            let display_name = if manufacturer.is_empty() {
                product
            } else {
                format!("{} {}", manufacturer, product)
            };

            let device_type = detect_device_type(&display_name, device_class);
            let device_id = format!("usb_{}", device_path.file_name().unwrap().to_string_lossy());

            let mut device = Device::with_id(
                device_id,
                display_name,
                device_type,
                ConnectionType::USB,
            );

            device.set_connected(true);

            if let Ok(battery_level) = self.get_usb_battery_level(device_path).await {
                device.update_battery(Some(battery_level));
            }

            return Ok(device);
        }

        Err(CoreError::DeviceDetectionFailed("Not an HID device".to_string()))
    }

    async fn get_usb_battery_level(&self, device_path: &PathBuf) -> Result<u8, CoreError> {
        let power_path = device_path.join("power");

        if !power_path.exists() {
            return Err(CoreError::DeviceDetectionFailed("No power info".to_string()));
        }

        let entries = fs::read_dir(&power_path)
            .map_err(|e| CoreError::SystemApiError(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| CoreError::SystemApiError(e))?;
            let entry_path = entry.path();

            if entry.file_name().to_string_lossy().starts_with("supply") {
                let capacity_path = entry_path.join("capacity");
                if capacity_path.exists() {
                    let capacity_str = fs::read_to_string(&capacity_path)
                        .map_err(|e| CoreError::SystemApiError(e))?;
                    if let Ok(capacity) = capacity_str.trim().parse::<u8>() {
                        return Ok(capacity);
                    }
                }
            }
        }

        Err(CoreError::DeviceDetectionFailed("No battery capacity found".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_usb_scanner_creation() {
        let scanner = UsbScanner::new();
        let devices = scanner.scan_devices().await;
        assert!(devices.is_ok());
    }
}
