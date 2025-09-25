use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use thiserror::Error;
use uuid::Uuid;

pub mod device_monitor;
pub mod bluetooth;
pub mod usb;

pub use device_monitor::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub connection_type: ConnectionType,
    pub battery_level: Option<u8>,
    pub connection_status: ConnectionStatus,
    pub last_seen: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    Mouse,
    Keyboard,
    Mobile,
    Buds,
    Headphones,
    Tablet,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionType {
    Bluetooth,
    USB,
    Wireless2_4G,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone)]
pub enum DeviceEvent {
    DeviceAdded(Device),
    DeviceUpdated(Device),
    DeviceRemoved(String),
    BatteryChanged(String, u8),
}

pub trait DeviceMonitor {
    fn subscribe(&mut self, callback: Box<dyn Fn(DeviceEvent) + Send + Sync>);
    fn start_monitoring(&mut self, interval: Duration) -> Result<(), CoreError>;
    fn stop_monitoring(&mut self);
    fn get_current_devices(&self) -> Vec<Device>;
    async fn refresh_devices(&mut self) -> Result<(), CoreError>;
}

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Device detection failed: {0}")]
    DeviceDetectionFailed(String),
    #[error("Permission denied accessing device: {device_id}")]
    PermissionDenied { device_id: String },
    #[error("System API error: {0}")]
    SystemApiError(#[from] std::io::Error),
    #[error("D-Bus error: {0}")]
    DBusError(#[from] zbus::Error),
    #[error("Monitor already running")]
    MonitorAlreadyRunning,
    #[error("Monitor not running")]
    MonitorNotRunning,
}

impl Device {
    pub fn new(
        name: String,
        device_type: DeviceType,
        connection_type: ConnectionType,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            device_type,
            connection_type,
            battery_level: None,
            connection_status: ConnectionStatus::Disconnected,
            last_seen: SystemTime::now(),
        }
    }

    pub fn with_id(
        id: String,
        name: String,
        device_type: DeviceType,
        connection_type: ConnectionType,
    ) -> Self {
        Self {
            id,
            name,
            device_type,
            connection_type,
            battery_level: None,
            connection_status: ConnectionStatus::Disconnected,
            last_seen: SystemTime::now(),
        }
    }

    pub fn update_battery(&mut self, level: Option<u8>) {
        self.battery_level = level;
        self.last_seen = SystemTime::now();
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connection_status = if connected {
            ConnectionStatus::Connected
        } else {
            ConnectionStatus::Disconnected
        };
        self.last_seen = SystemTime::now();
    }
}

pub fn detect_device_type(device_name: &str, device_class: Option<u32>) -> DeviceType {
    let name_lower = device_name.to_lowercase();

    if let Some(class) = device_class {
        match class {
            0x002580 => return DeviceType::Mouse,
            0x002540 => return DeviceType::Keyboard,
            0x240404 => return DeviceType::Headphones,
            _ => {}
        }
    }

    if name_lower.contains("mouse") {
        DeviceType::Mouse
    } else if name_lower.contains("keyboard") {
        DeviceType::Keyboard
    } else if name_lower.contains("headphone") || name_lower.contains("headset") {
        DeviceType::Headphones
    } else if name_lower.contains("buds") || name_lower.contains("airpods") {
        DeviceType::Buds
    } else if name_lower.contains("phone") || name_lower.contains("mobile") {
        DeviceType::Mobile
    } else if name_lower.contains("tablet") || name_lower.contains("ipad") {
        DeviceType::Tablet
    } else {
        DeviceType::Unknown
    }
}
