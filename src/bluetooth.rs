use bluer::{Address, Device};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct BluetoothDevice {
    pub name: String,
    pub address: Address,
    pub battery_percentage: Option<u8>,
    pub device_type: BluetoothDeviceType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BluetoothDeviceType {
    Headphones,
    Mouse,
    Phone,
    Tablet,
    Speaker,
    Unknown,
}

impl BluetoothDevice {
    pub async fn from_device(device: Device, addr: Address) -> bluer::Result<Option<Self>> {
        if !device.is_connected().await? {
            return Ok(None);
        }

        let name = device.name().await?.unwrap_or_else(|| "Unknown Device".to_string());
        let battery_percentage = device.battery_percentage().await?;
        let device_type = Self::detect_device_type(&name, &device).await;

        Ok(Some(BluetoothDevice {
            name,
            address: addr,
            battery_percentage,
            device_type,
        }))
    }

    // fn classify_device(uuids: &[uuid::Uuid]) -> &'static str {
    //     for uuid in uuids {
    //         let short = uuid.as_u128() >> 96; // extract 16-bit portion if in Bluetooth base UUID

    //         match short {
    //             0x1812 => return "Keyboard/Mouse (HID)",  // Human Interface Device
    //             0x1108 | 0x111e => return "Headset/Handsfree",
    //             0x1800 => return "Generic Access (Phone/Peripheral)",
    //             0x180F => return "Battery-powered Device",
    //             _ => {}
    //         }
    //     }

    //     "Unknown"
    // }

    async fn detect_device_type(name: &str, _device: &Device) -> BluetoothDeviceType {
        let name_lower = name.to_lowercase();
        // let class = device.all_properties().await;
        // println!("Device: {:?}", class);

        if name_lower.contains("headphone") || name_lower.contains("earbuds") ||
           name_lower.contains("airpods") || name_lower.contains("buds") {
            BluetoothDeviceType::Headphones
        } else if name_lower.contains("mouse") {
            BluetoothDeviceType::Mouse
        } else if name_lower.contains("phone") || name_lower.contains("iphone") ||
                  name_lower.contains("samsung") || name_lower.contains("pixel") {
            BluetoothDeviceType::Phone
        } else if name_lower.contains("ipad") || name_lower.contains("tablet") {
            BluetoothDeviceType::Tablet
        } else if name_lower.contains("speaker") || name_lower.contains("soundbar") {
            BluetoothDeviceType::Speaker
        } else {
            BluetoothDeviceType::Unknown
        }
    }

    pub fn get_icon(&self) -> &'static str {
        match self.device_type {
            BluetoothDeviceType::Headphones => "🎧",
            BluetoothDeviceType::Mouse => "🖱️",
            BluetoothDeviceType::Phone => "📱",
            BluetoothDeviceType::Tablet => "📟",
            BluetoothDeviceType::Speaker => "🔊",
            BluetoothDeviceType::Unknown => "📻",
        }
    }

    pub fn format_for_status(&self) -> String {
        let short_name = if self.name.len() > 12 {
            format!("{}...", &self.name[..9])
        } else {
            self.name.clone()
        };

        match self.battery_percentage {
            Some(battery) => format!("{} {}: {}%", self.get_icon(), short_name, battery),
            None => format!("{} {}", self.get_icon(), short_name),
        }
    }
}

pub struct BluetoothManager {
    pub connected_devices: HashMap<Address, BluetoothDevice>,
}

impl BluetoothManager {
    pub fn new() -> Self {
        Self {
            connected_devices: HashMap::new(),
        }
    }

    pub fn add_device(&mut self, device: BluetoothDevice) {
        println!("Connected Bluetooth device: {} ({})", device.name, device.address);
        if let Some(battery) = device.battery_percentage {
            println!("  Battery: {}%", battery);
        }
        self.connected_devices.insert(device.address, device);
    }

    pub fn remove_device(&mut self, addr: Address) -> bool {
        if let Some(device) = self.connected_devices.remove(&addr) {
            println!("Bluetooth device disconnected: {} ({})", device.name, addr);
            true
        } else {
            false
        }
    }

    pub fn update_device(&mut self, addr: Address, updated_device: BluetoothDevice) -> bool {
        if let Some(existing_device) = self.connected_devices.get_mut(&addr) {
            if existing_device.battery_percentage != updated_device.battery_percentage {
                println!("Bluetooth battery updated for {}: {:?}%",
                    updated_device.name, updated_device.battery_percentage);
                *existing_device = updated_device;
                return true;
            }
        }
        false
    }

    pub fn get_status_text(&self) -> String {
        if self.connected_devices.is_empty() {
            return "No Bluetooth devices".to_string();
        }

        let mut status_parts = Vec::new();
        for device in self.connected_devices.values() {
            if device.battery_percentage.is_some() {
                status_parts.push(device.format_for_status());
            }
        }

        if status_parts.is_empty() {
            format!("{} connected Bluetooth device(s)", self.connected_devices.len())
        } else {
            status_parts.join(" | ")
        }
    }
}
