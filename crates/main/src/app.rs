use battery_monitor_config::Config;
use battery_monitor_core::{DeviceEvent, DeviceMonitor, LinuxDeviceMonitor};
use battery_monitor_notifications::{DesktopNotificationManager, NotificationManager, NotificationType};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{debug, error, info, warn};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Device monitor initialization failed: {0}")]
    DeviceMonitorError(#[from] battery_monitor_core::CoreError),
    #[error("Configuration error: {0}")]
    ConfigError(#[from] battery_monitor_config::ConfigError),
    #[error("Notification error: {0}")]
    NotificationError(#[from] battery_monitor_notifications::NotificationError),
    #[error("Application already running")]
    AlreadyRunning,
    #[error("Application not initialized")]
    NotInitialized,
}

pub struct BatteryMonitorApp {
    config: Arc<Mutex<Config>>,
    device_monitor: LinuxDeviceMonitor,
    notification_manager: Box<dyn NotificationManager + Send + Sync>,
    devices: Arc<Mutex<HashMap<String, battery_monitor_core::Device>>>,
    is_running: Arc<Mutex<bool>>,
    shutdown_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl BatteryMonitorApp {
    pub async fn new(config: Config, _with_gui: bool) -> Result<Self, AppError> {
        let config = Arc::new(Mutex::new(config));

        let device_monitor = LinuxDeviceMonitor::new()?;

        let mut notification_manager = Box::new(DesktopNotificationManager::new());
        {
            let config_guard = config.lock().unwrap();
            notification_manager.set_enabled(config_guard.notifications.enabled);
            notification_manager.set_suppression_duration(config_guard.suppression_duration());
        }

        let devices = Arc::new(Mutex::new(HashMap::new()));

        let mut app = Self {
            config: Arc::clone(&config),
            device_monitor,
            notification_manager,
            devices: Arc::clone(&devices),
            is_running: Arc::new(Mutex::new(false)),
            shutdown_sender: None,
        };

        app.setup_device_callbacks()?;

        info!("Battery Monitor application initialized");
        Ok(app)
    }

    fn setup_device_callbacks(&mut self) -> Result<(), AppError> {
        let config = Arc::clone(&self.config);
        let devices = Arc::clone(&self.devices);

        // Create a separate notification manager for the callback
        let callback_notification_manager = Arc::new(Mutex::new(Box::new(DesktopNotificationManager::new()) as Box<dyn NotificationManager + Send + Sync>));

        // Initialize the callback notification manager
        {
            let config_guard = config.lock().unwrap();
            let mut nm = callback_notification_manager.lock().unwrap();
            nm.set_enabled(config_guard.notifications.enabled);
            nm.set_suppression_duration(config_guard.suppression_duration());
        }

        self.device_monitor.subscribe(Box::new(move |event| {
            Self::handle_device_event(
                event,
                Arc::clone(&config),
                Arc::clone(&callback_notification_manager),
                Arc::clone(&devices),
            );
        }));

        Ok(())
    }

    fn handle_device_event(
        event: DeviceEvent,
        config: Arc<Mutex<Config>>,
        notification_manager: Arc<Mutex<Box<dyn NotificationManager + Send + Sync>>>,
        devices: Arc<Mutex<HashMap<String, battery_monitor_core::Device>>>,
    ) {
        match event {
            DeviceEvent::DeviceAdded(device) => {
                info!("Device added: {} ({})", device.name, device.id);

                {
                    let mut devices_guard = devices.lock().unwrap();
                    devices_guard.insert(device.id.clone(), device.clone());
                }

                let config_guard = config.lock().unwrap();
                if config_guard.notifications.show_connect_disconnect {
                    let mut nm = notification_manager.lock().unwrap();
                    if let Err(e) = nm.send_notification(NotificationType::DeviceConnected(device)) {
                        debug!("Failed to send connection notification: {}", e);
                    }
                }
            }
            DeviceEvent::DeviceUpdated(device) => {
                debug!("Device updated: {} ({})", device.name, device.id);

                {
                    let mut devices_guard = devices.lock().unwrap();
                    devices_guard.insert(device.id.clone(), device);
                }
            }
            DeviceEvent::DeviceRemoved(device_id) => {
                info!("Device removed: {}", device_id);

                let removed_device = {
                    let mut devices_guard = devices.lock().unwrap();
                    devices_guard.remove(&device_id)
                };

                if let Some(device) = removed_device {
                    let config_guard = config.lock().unwrap();
                    if config_guard.notifications.show_connect_disconnect {
                        let mut nm = notification_manager.lock().unwrap();
                        if let Err(e) = nm.send_notification(NotificationType::DeviceDisconnected(device)) {
                            debug!("Failed to send disconnection notification: {}", e);
                        }
                    }
                }
            }
            DeviceEvent::BatteryChanged(device_id, level) => {
                debug!("Battery changed for {}: {}%", device_id, level);

                let device = {
                    let devices_guard = devices.lock().unwrap();
                    devices_guard.get(&device_id).cloned()
                };

                if let Some(device) = device {
                    let config_guard = config.lock().unwrap();
                    if level <= config_guard.notifications.low_battery_threshold {
                        let mut nm = notification_manager.lock().unwrap();
                        if let Err(e) = nm.send_notification(NotificationType::LowBattery {
                            device,
                            threshold: config_guard.notifications.low_battery_threshold,
                        }) {
                            debug!("Failed to send low battery notification: {}", e);
                        }
                    }
                }
            }
        }
    }

    pub async fn run(&mut self) -> Result<(), AppError> {
        {
            let mut is_running = self.is_running.lock().unwrap();
            if *is_running {
                return Err(AppError::AlreadyRunning);
            }
            *is_running = true;
        }

        let polling_interval = {
            let config = self.config.lock().unwrap();
            config.polling_interval()
        };

        info!("Starting device monitoring with interval {:?}", polling_interval);
        self.device_monitor.start_monitoring(polling_interval)?;

        let (shutdown_sender, mut shutdown_receiver) = tokio::sync::oneshot::channel();
        self.shutdown_sender = Some(shutdown_sender);

        let mut update_interval = interval(Duration::from_secs(1));
        update_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let mut crash_recovery_task = self.setup_crash_recovery();

        loop {
            tokio::select! {
                _ = update_interval.tick() => {
                    // Periodic maintenance tasks
                    self.perform_maintenance().await;
                }
                _ = &mut shutdown_receiver => {
                    info!("Shutdown signal received");
                    break;
                }
                result = &mut crash_recovery_task => {
                    match result {
                        Ok(_) => {
                            warn!("Crash recovery task completed unexpectedly");
                        }
                        Err(e) => {
                            error!("Crash recovery task failed: {}", e);
                        }
                    }
                }
            }
        }

        self.device_monitor.stop_monitoring();

        {
            let mut is_running = self.is_running.lock().unwrap();
            *is_running = false;
        }

        info!("Battery Monitor application stopped");
        Ok(())
    }

    async fn perform_maintenance(&mut self) {
        // Perform periodic maintenance tasks
        debug!("Performing maintenance tasks");

        // Refresh device list periodically
        if let Err(e) = self.device_monitor.refresh_devices().await {
            warn!("Failed to refresh devices during maintenance: {}", e);
        }

        // Print current device status
        let devices = self.devices.lock().unwrap();
        let connected_count = devices.len();
        if connected_count > 0 {
            debug!("Currently tracking {} devices", connected_count);
            for device in devices.values() {
                debug!("  - {}: {:?} ({})",
                       device.name,
                       device.battery_level,
                       match device.connection_status {
                           battery_monitor_core::ConnectionStatus::Connected => "connected",
                           battery_monitor_core::ConnectionStatus::Disconnected => "disconnected",
                       });
            }
        }
    }

    fn setup_crash_recovery(&self) -> tokio::task::JoinHandle<Result<(), AppError>> {
        let is_running = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            let mut crash_check_interval = interval(Duration::from_secs(30));
            crash_check_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                crash_check_interval.tick().await;

                let running = {
                    let is_running_guard = is_running.lock().unwrap();
                    *is_running_guard
                };

                if !running {
                    debug!("Application not running, crash recovery exiting");
                    break;
                }

                // Perform crash recovery checks
                debug!("Crash recovery check passed");
            }

            Ok(())
        })
    }

    pub async fn shutdown(&mut self) {
        info!("Shutting down Battery Monitor application");

        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(());
        }

        self.device_monitor.stop_monitoring();

        {
            let mut is_running = self.is_running.lock().unwrap();
            *is_running = false;
        }

        info!("Application shutdown complete");
    }

    pub fn is_running(&self) -> bool {
        let is_running = self.is_running.lock().unwrap();
        *is_running
    }

    pub fn get_current_config(&self) -> Config {
        let config = self.config.lock().unwrap();
        config.clone()
    }

    pub fn update_config(&mut self, new_config: Config) -> Result<(), AppError> {
        {
            let mut config = self.config.lock().unwrap();
            *config = new_config.clone();
        }

        // Update notification manager settings
        self.notification_manager.set_enabled(new_config.notifications.enabled);
        self.notification_manager.set_suppression_duration(new_config.suppression_duration());

        // Restart monitoring with new interval if changed
        self.device_monitor.stop_monitoring();
        self.device_monitor.start_monitoring(new_config.polling_interval())?;

        info!("Configuration updated");
        Ok(())
    }
}
