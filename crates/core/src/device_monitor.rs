use crate::{Device, DeviceEvent, DeviceMonitor, CoreError};
use crate::bluetooth::BluetoothScanner;
use crate::usb::UsbScanner;
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

pub struct LinuxDeviceMonitor {
    devices: Arc<StdMutex<HashMap<String, Device>>>,
    callbacks: Vec<Box<dyn Fn(DeviceEvent) + Send + Sync>>,
    bluetooth_scanner: Arc<Mutex<BluetoothScanner>>,
    usb_scanner: UsbScanner,
    monitoring_task: Option<tokio::task::JoinHandle<()>>,
    shutdown_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl LinuxDeviceMonitor {
    pub fn new() -> Result<Self, CoreError> {
        Ok(Self {
            devices: Arc::new(StdMutex::new(HashMap::new())),
            callbacks: Vec::new(),
            bluetooth_scanner: Arc::new(Mutex::new(BluetoothScanner::new().map_err(|e| {
                CoreError::DeviceDetectionFailed(format!("Failed to initialize Bluetooth scanner: {}", e))
            })?)),
            usb_scanner: UsbScanner::new(),
            monitoring_task: None,
            shutdown_sender: None,
        })
    }

    fn notify_callbacks(&self, event: DeviceEvent) {
        for callback in &self.callbacks {
            callback(event.clone());
        }
    }

    fn update_device_list(&self, mut new_devices: Vec<Device>) -> Vec<DeviceEvent> {
        let mut events = Vec::new();
        let mut devices = self.devices.lock().unwrap();
        let mut current_device_ids = std::collections::HashSet::new();

        for device in new_devices.drain(..) {
            current_device_ids.insert(device.id.clone());

            match devices.get(&device.id) {
                Some(existing_device) => {
                    if existing_device != &device {
                        if existing_device.battery_level != device.battery_level {
                            if let Some(level) = device.battery_level {
                                events.push(DeviceEvent::BatteryChanged(device.id.clone(), level));
                            }
                        }
                        events.push(DeviceEvent::DeviceUpdated(device.clone()));
                        devices.insert(device.id.clone(), device);
                    }
                }
                None => {
                    events.push(DeviceEvent::DeviceAdded(device.clone()));
                    devices.insert(device.id.clone(), device);
                }
            }
        }

        let removed_devices: Vec<String> = devices
            .keys()
            .filter(|id| !current_device_ids.contains(*id))
            .cloned()
            .collect();

        for device_id in removed_devices {
            devices.remove(&device_id);
            events.push(DeviceEvent::DeviceRemoved(device_id));
        }

        events
    }

    async fn monitoring_loop(&self, interval_duration: Duration, mut shutdown_receiver: tokio::sync::oneshot::Receiver<()>) {
        let mut interval_timer = interval(interval_duration);
        interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval_timer.tick() => {
                    match self.scan_all_devices().await {
                        Ok(devices) => {
                            let events = self.update_device_list(devices);
                            for event in events {
                                self.notify_callbacks(event);
                            }
                        }
                        Err(e) => {
                            error!("Error scanning devices: {}", e);
                        }
                    }
                }
                _ = &mut shutdown_receiver => {
                    info!("Device monitoring stopped");
                    break;
                }
            }
        }
    }

    async fn scan_all_devices(&self) -> Result<Vec<Device>, CoreError> {
        let mut all_devices = Vec::new();

        // Scan Bluetooth devices with mutex lock
        let bluetooth_result = {
            let mut bluetooth_scanner = self.bluetooth_scanner.lock().await;
            bluetooth_scanner.scan_devices().await
        };
        match bluetooth_result {
            Ok(mut bt_devices) => {
                all_devices.append(&mut bt_devices);
            }
            Err(e) => {
                warn!("Bluetooth scan failed: {}", e);
            }
        }

        match self.usb_scanner.scan_devices().await {
            Ok(mut usb_devices) => {
                all_devices.append(&mut usb_devices);
            }
            Err(e) => {
                warn!("USB scan failed: {}", e);
            }
        }

        debug!("Found {} devices total", all_devices.len());
        Ok(all_devices)
    }
}

impl DeviceMonitor for LinuxDeviceMonitor {
    fn subscribe(&mut self, callback: Box<dyn Fn(DeviceEvent) + Send + Sync>) {
        self.callbacks.push(callback);
    }

    fn start_monitoring(&mut self, interval_duration: Duration) -> Result<(), CoreError> {
        if self.monitoring_task.is_some() {
            return Err(CoreError::MonitorAlreadyRunning);
        }

        let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel();
        self.shutdown_sender = Some(shutdown_sender);

        let monitor = self.clone_for_monitoring();
        let task = tokio::spawn(async move {
            monitor.monitoring_loop(interval_duration, shutdown_receiver).await;
        });

        self.monitoring_task = Some(task);
        info!("Device monitoring started with interval {:?}", interval_duration);
        Ok(())
    }

    fn stop_monitoring(&mut self) {
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(());
        }

        if let Some(task) = self.monitoring_task.take() {
            task.abort();
        }

        info!("Device monitoring stopped");
    }

    fn get_current_devices(&self) -> Vec<Device> {
        let devices = self.devices.lock().unwrap();
        devices.values().cloned().collect()
    }

    async fn refresh_devices(&mut self) -> Result<(), CoreError> {
        let devices = self.scan_all_devices().await?;
        let events = self.update_device_list(devices);

        for event in events {
            self.notify_callbacks(event);
        }

        Ok(())
    }
}

impl LinuxDeviceMonitor {
    fn clone_for_monitoring(&self) -> MonitoringClone {
        MonitoringClone {
            devices: Arc::clone(&self.devices),
            callbacks: self.callbacks.iter().map(|cb| {
                let cb_ptr = cb.as_ref() as *const (dyn Fn(DeviceEvent) + Send + Sync);
                unsafe { &*cb_ptr }
            }).collect(),
            bluetooth_scanner: Arc::clone(&self.bluetooth_scanner),
            usb_scanner: self.usb_scanner.clone(),
        }
    }
}

struct MonitoringClone {
    devices: Arc<StdMutex<HashMap<String, Device>>>,
    callbacks: Vec<&'static (dyn Fn(DeviceEvent) + Send + Sync)>,
    bluetooth_scanner: Arc<Mutex<BluetoothScanner>>,
    usb_scanner: UsbScanner,
}

impl MonitoringClone {
    fn notify_callbacks(&self, event: DeviceEvent) {
        for callback in &self.callbacks {
            callback(event.clone());
        }
    }

    fn update_device_list(&self, mut new_devices: Vec<Device>) -> Vec<DeviceEvent> {
        let mut events = Vec::new();
        let mut devices = self.devices.lock().unwrap();
        let mut current_device_ids = std::collections::HashSet::new();

        for device in new_devices.drain(..) {
            current_device_ids.insert(device.id.clone());

            match devices.get(&device.id) {
                Some(existing_device) => {
                    if existing_device != &device {
                        if existing_device.battery_level != device.battery_level {
                            if let Some(level) = device.battery_level {
                                events.push(DeviceEvent::BatteryChanged(device.id.clone(), level));
                            }
                        }
                        events.push(DeviceEvent::DeviceUpdated(device.clone()));
                        devices.insert(device.id.clone(), device);
                    }
                }
                None => {
                    events.push(DeviceEvent::DeviceAdded(device.clone()));
                    devices.insert(device.id.clone(), device);
                }
            }
        }

        let removed_devices: Vec<String> = devices
            .keys()
            .filter(|id| !current_device_ids.contains(*id))
            .cloned()
            .collect();

        for device_id in removed_devices {
            devices.remove(&device_id);
            events.push(DeviceEvent::DeviceRemoved(device_id));
        }

        events
    }

    async fn scan_all_devices(&self) -> Result<Vec<Device>, CoreError> {
        let mut all_devices = Vec::new();

        // Scan Bluetooth devices with mutex lock
        let bluetooth_result = {
            let mut bluetooth_scanner = self.bluetooth_scanner.lock().await;
            bluetooth_scanner.scan_devices().await
        };
        match bluetooth_result {
            Ok(mut bt_devices) => {
                all_devices.append(&mut bt_devices);
            }
            Err(e) => {
                warn!("Bluetooth scan failed: {}", e);
            }
        }

        match self.usb_scanner.scan_devices().await {
            Ok(mut usb_devices) => {
                all_devices.append(&mut usb_devices);
            }
            Err(e) => {
                warn!("USB scan failed: {}", e);
            }
        }

        debug!("Found {} devices total", all_devices.len());
        Ok(all_devices)
    }

    async fn monitoring_loop(&self, interval_duration: Duration, mut shutdown_receiver: tokio::sync::oneshot::Receiver<()>) {
        let mut interval_timer = interval(interval_duration);
        interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval_timer.tick() => {
                    match self.scan_all_devices().await {
                        Ok(devices) => {
                            let events = self.update_device_list(devices);
                            for event in events {
                                self.notify_callbacks(event);
                            }
                        }
                        Err(e) => {
                            error!("Error scanning devices: {}", e);
                        }
                    }
                }
                _ = &mut shutdown_receiver => {
                    info!("Device monitoring stopped");
                    break;
                }
            }
        }
    }
}
