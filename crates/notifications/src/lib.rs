use battery_monitor_core::{Device, DeviceType};
use notify_rust::{Notification, NotificationHandle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    LowBattery { device: Device, threshold: u8 },
    DeviceConnected(Device),
    DeviceDisconnected(Device),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRecord {
    pub timestamp: SystemTime,
    pub notification_type: NotificationType,
    pub sent: bool,
}

pub trait NotificationManager {
    fn send_notification(
        &mut self,
        notification: NotificationType,
    ) -> Result<(), NotificationError>;
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
    fn get_notification_log(&self) -> Vec<NotificationRecord>;
    fn clear_log(&mut self);
    fn set_suppression_duration(&mut self, duration: Duration);
}

#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("Notification system error: {0}")]
    SystemError(String),
    #[error("Notification disabled")]
    Disabled,
    #[error("Notification suppressed")]
    Suppressed,
    #[error("Invalid notification data: {0}")]
    InvalidData(String),
}

pub struct DesktopNotificationManager {
    enabled: bool,
    suppression_duration: Duration,
    notification_log: Vec<NotificationRecord>,
    last_low_battery_alert: HashMap<String, SystemTime>,
    last_connection_alert: HashMap<String, SystemTime>,
    max_log_size: usize,
}

impl DesktopNotificationManager {
    pub fn new() -> Self {
        Self {
            enabled: true,
            suppression_duration: Duration::from_secs(300), // 5 minutes
            notification_log: Vec::new(),
            last_low_battery_alert: HashMap::new(),
            last_connection_alert: HashMap::new(),
            max_log_size: 1000,
        }
    }

    fn is_suppressed(&self, device_id: &str, notification_type: &NotificationType) -> bool {
        let now = SystemTime::now();

        match notification_type {
            NotificationType::LowBattery { .. } => {
                if let Some(last_alert) = self.last_low_battery_alert.get(device_id) {
                    if let Ok(duration) = now.duration_since(*last_alert) {
                        return duration < self.suppression_duration;
                    }
                }
            }
            NotificationType::DeviceConnected(_) | NotificationType::DeviceDisconnected(_) => {
                if let Some(last_alert) = self.last_connection_alert.get(device_id) {
                    if let Ok(duration) = now.duration_since(*last_alert) {
                        return duration < self.suppression_duration;
                    }
                }
            }
        }

        false
    }

    fn update_suppression_state(&mut self, device_id: &str, notification_type: &NotificationType) {
        let now = SystemTime::now();

        match notification_type {
            NotificationType::LowBattery { .. } => {
                self.last_low_battery_alert
                    .insert(device_id.to_string(), now);
            }
            NotificationType::DeviceConnected(_) | NotificationType::DeviceDisconnected(_) => {
                self.last_connection_alert
                    .insert(device_id.to_string(), now);
            }
        }
    }

    fn send_desktop_notification(
        &self,
        notification_type: &NotificationType,
    ) -> Result<NotificationHandle, NotificationError> {
        let (summary, body, icon) = match notification_type {
            NotificationType::LowBattery { device, threshold } => {
                let battery_text = if let Some(level) = device.battery_level {
                    format!("{}%", level)
                } else {
                    "Unknown".to_string()
                };

                let summary = format!("Low Battery: {}", device.name);
                let body = format!(
                    "Battery level is {} (below {}% threshold)",
                    battery_text, threshold
                );
                let icon = self.get_device_icon(&device.device_type);
                (summary, body, icon)
            }
            NotificationType::DeviceConnected(device) => {
                let summary = format!("Device Connected");
                let body = format!("{} is now connected", device.name);
                let icon = self.get_device_icon(&device.device_type);
                (summary, body, icon)
            }
            NotificationType::DeviceDisconnected(device) => {
                let summary = format!("Device Disconnected");
                let body = format!("{} has been disconnected", device.name);
                let icon = self.get_device_icon(&device.device_type);
                (summary, body, icon)
            }
        };

        let mut notification = Notification::new();
        notification
            .summary(&summary)
            .body(&body)
            .icon(&icon)
            .timeout(notify_rust::Timeout::Milliseconds(5000));

        // Set urgency based on notification type
        match notification_type {
            NotificationType::LowBattery { .. } => {
                notification.urgency(notify_rust::Urgency::Normal);
            }
            _ => {
                notification.urgency(notify_rust::Urgency::Low);
            }
        }

        notification.show().map_err(|e| {
            NotificationError::SystemError(format!("Failed to show notification: {}", e))
        })
    }

    fn get_device_icon(&self, device_type: &DeviceType) -> &'static str {
        match device_type {
            DeviceType::Mouse => "input-mouse",
            DeviceType::Keyboard => "input-keyboard",
            DeviceType::Mobile => "phone",
            DeviceType::Buds | DeviceType::Headphones => "audio-headphones",
            DeviceType::Tablet => "computer-tablet",
            DeviceType::Unknown => "battery",
        }
    }

    fn add_to_log(&mut self, notification_type: NotificationType, sent: bool) {
        let record = NotificationRecord {
            timestamp: SystemTime::now(),
            notification_type,
            sent,
        };

        self.notification_log.push(record);

        // Trim log if it exceeds max size
        if self.notification_log.len() > self.max_log_size {
            let excess = self.notification_log.len() - self.max_log_size;
            self.notification_log.drain(0..excess);
        }

        debug!(
            "Added notification to log, total entries: {}",
            self.notification_log.len()
        );
    }

    fn get_device_id_from_notification(notification_type: &NotificationType) -> &str {
        match notification_type {
            NotificationType::LowBattery { device, .. } => &device.id,
            NotificationType::DeviceConnected(device) => &device.id,
            NotificationType::DeviceDisconnected(device) => &device.id,
        }
    }
}

impl NotificationManager for DesktopNotificationManager {
    fn send_notification(
        &mut self,
        notification: NotificationType,
    ) -> Result<(), NotificationError> {
        if !self.enabled {
            self.add_to_log(notification, false);
            return Err(NotificationError::Disabled);
        }

        let device_id = Self::get_device_id_from_notification(&notification);

        if self.is_suppressed(device_id, &notification) {
            self.add_to_log(notification, false);
            return Err(NotificationError::Suppressed);
        }

        match self.send_desktop_notification(&notification) {
            Ok(_handle) => {
                self.update_suppression_state(device_id, &notification);
                self.add_to_log(notification, true);
                info!("Notification sent successfully");
                Ok(())
            }
            Err(e) => {
                self.add_to_log(notification, false);
                error!("Failed to send notification: {}", e);
                Err(e)
            }
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        info!(
            "Notifications {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }

    fn get_notification_log(&self) -> Vec<NotificationRecord> {
        self.notification_log.clone()
    }

    fn clear_log(&mut self) {
        self.notification_log.clear();
        self.last_low_battery_alert.clear();
        self.last_connection_alert.clear();
        info!("Notification log cleared");
    }

    fn set_suppression_duration(&mut self, duration: Duration) {
        self.suppression_duration = duration;
        info!("Notification suppression duration set to {:?}", duration);
    }
}

impl Default for DesktopNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use battery_monitor_core::{ConnectionStatus, ConnectionType};

    fn create_test_device() -> Device {
        Device {
            id: "test-device".to_string(),
            name: "Test Mouse".to_string(),
            device_type: DeviceType::Mouse,
            connection_type: ConnectionType::Bluetooth,
            battery_level: Some(15),
            connection_status: ConnectionStatus::Connected,
            last_seen: SystemTime::now(),
        }
    }

    #[test]
    fn test_notification_manager_creation() {
        let manager = DesktopNotificationManager::new();
        assert!(manager.is_enabled());
        assert_eq!(manager.get_notification_log().len(), 0);
    }

    #[test]
    fn test_enable_disable_notifications() {
        let mut manager = DesktopNotificationManager::new();
        assert!(manager.is_enabled());

        manager.set_enabled(false);
        assert!(!manager.is_enabled());

        manager.set_enabled(true);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_suppression_duration() {
        let mut manager = DesktopNotificationManager::new();
        let new_duration = Duration::from_secs(600);

        manager.set_suppression_duration(new_duration);
        assert_eq!(manager.suppression_duration, new_duration);
    }

    #[test]
    fn test_notification_log() {
        let mut manager = DesktopNotificationManager::new();
        let device = create_test_device();

        let notification = NotificationType::LowBattery {
            device: device.clone(),
            threshold: 20,
        };

        manager.add_to_log(notification, true);

        let log = manager.get_notification_log();
        assert_eq!(log.len(), 1);
        assert!(log[0].sent);

        manager.clear_log();
        assert_eq!(manager.get_notification_log().len(), 0);
    }

    #[test]
    fn test_device_icon_mapping() {
        let manager = DesktopNotificationManager::new();

        assert_eq!(manager.get_device_icon(&DeviceType::Mouse), "input-mouse");
        assert_eq!(
            manager.get_device_icon(&DeviceType::Keyboard),
            "input-keyboard"
        );
        assert_eq!(manager.get_device_icon(&DeviceType::Mobile), "phone");
        assert_eq!(
            manager.get_device_icon(&DeviceType::Headphones),
            "audio-headphones"
        );
        assert_eq!(manager.get_device_icon(&DeviceType::Unknown), "battery");
    }
}
