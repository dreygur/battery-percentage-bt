use battery_monitor_config::Config;
use battery_monitor_core::{ConnectionStatus, Device, DeviceType};
use gtk4::prelude::*;
use gtk4::Application;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tracing::{debug, error, info};

pub mod details;
pub mod settings;
pub mod tray;

pub use details::*;
pub use settings::*;
pub use tray::*;

#[derive(Error, Debug)]
pub enum GuiError {
    #[error("GTK initialization failed: {0}")]
    GtkInitError(String),
    #[error("Application creation failed: {0}")]
    ApplicationError(String),
    #[error("Window creation failed: {0}")]
    WindowError(String),
    #[error("Widget error: {0}")]
    WidgetError(String),
}

pub struct BatteryMonitorGui {
    application: Application,
    tray_icon: Option<TrayIcon>,
    details_window: Option<DetailsWindow>,
    settings_dialog: Option<SettingsDialog>,
    devices: Arc<Mutex<HashMap<String, Device>>>,
    config: Arc<Mutex<Config>>,
}

impl BatteryMonitorGui {
    pub fn new(app_id: &str) -> Result<Self, GuiError> {
        let application = Application::builder().application_id(app_id).build();

        Ok(Self {
            application,
            tray_icon: None,
            details_window: None,
            settings_dialog: None,
            devices: Arc::new(Mutex::new(HashMap::new())),
            config: Arc::new(Mutex::new(Config::default())),
        })
    }

    pub fn initialize(&mut self) -> Result<(), GuiError> {
        let devices = Arc::clone(&self.devices);
        let config = Arc::clone(&self.config);

        self.application.connect_activate(move |app| {
            info!("GTK application activated");

            // Create main window
            let window = gtk4::ApplicationWindow::builder()
                .application(app)
                .title("Battery Monitor")
                .default_width(400)
                .default_height(300)
                .build();

            // Create main content box
            let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
            main_box.set_margin_start(20);
            main_box.set_margin_end(20);
            main_box.set_margin_top(20);
            main_box.set_margin_bottom(20);

            // Header
            let header_label = gtk4::Label::new(Some("Battery Monitor"));
            header_label.set_markup("<b><big>Battery Monitor</big></b>");
            header_label.set_halign(gtk4::Align::Center);
            main_box.append(&header_label);

            // Device list
            let device_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
            let devices_guard = devices.lock().unwrap();

            if devices_guard.is_empty() {
                let no_devices_label = gtk4::Label::new(Some("No devices detected"));
                no_devices_label.set_halign(gtk4::Align::Center);
                device_box.append(&no_devices_label);
            } else {
                for device in devices_guard.values() {
                    let device_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

                    let icon = gtk4::Image::from_icon_name(get_device_icon_name(&device.device_type));
                    device_row.append(&icon);

                    let name_label = gtk4::Label::new(Some(&device.name));
                    name_label.set_hexpand(true);
                    name_label.set_halign(gtk4::Align::Start);
                    device_row.append(&name_label);

                    let battery_text = match device.battery_level {
                        Some(level) => format!("{}%", level),
                        None => "--".to_string(),
                    };
                    let battery_label = gtk4::Label::new(Some(&battery_text));
                    device_row.append(&battery_label);

                    device_box.append(&device_row);
                }
            }

            main_box.append(&device_box);

            // Buttons
            let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
            button_box.set_halign(gtk4::Align::Center);

            let refresh_button = gtk4::Button::with_label("Refresh");
            let quit_button = gtk4::Button::with_label("Quit");

            button_box.append(&refresh_button);
            button_box.append(&quit_button);
            main_box.append(&button_box);

            // Connect quit button
            let window_clone = window.clone();
            quit_button.connect_clicked(move |_| {
                window_clone.close();
            });

            window.set_child(Some(&main_box));
            window.present();

            info!("Main window created and presented");
        });

        let tray_icon = TrayIcon::new(Arc::clone(&self.devices))?;
        self.tray_icon = Some(tray_icon);

        info!("GUI initialized successfully");
        Ok(())
    }

    pub fn run(&self) -> Result<(), GuiError> {
        let args: Vec<String> = std::env::args().collect();
        let exit_code = self.application.run_with_args(&args);

        if exit_code != 0.into() {
            return Err(GuiError::ApplicationError(format!(
                "Application exited with code {:?}",
                exit_code
            )));
        }

        Ok(())
    }

    pub fn update_devices(&mut self, new_devices: HashMap<String, Device>) {
        {
            let mut devices = self.devices.lock().unwrap();
            *devices = new_devices;
        }

        if let Some(tray) = &mut self.tray_icon {
            tray.update_devices();
        }

        if let Some(details) = &mut self.details_window {
            details.update_devices();
        }

        debug!("GUI devices updated");
    }

    pub fn update_config(&mut self, new_config: Config) {
        {
            let mut config = self.config.lock().unwrap();
            *config = new_config;
        }

        if let Some(settings) = &mut self.settings_dialog {
            settings.update_config();
        }

        debug!("GUI config updated");
    }

    pub fn show_details(&mut self) -> Result<(), GuiError> {
        if self.details_window.is_none() {
            let details = DetailsWindow::new(Arc::clone(&self.devices), Arc::clone(&self.config))?;
            self.details_window = Some(details);
        }

        if let Some(details) = &mut self.details_window {
            details.show();
        }

        Ok(())
    }

    pub fn show_settings(&mut self) -> Result<(), GuiError> {
        if self.settings_dialog.is_none() {
            let settings = SettingsDialog::new(Arc::clone(&self.config))?;
            self.settings_dialog = Some(settings);
        }

        if let Some(settings) = &mut self.settings_dialog {
            settings.show()?;
        }

        Ok(())
    }

    pub fn hide_windows(&mut self) {
        if let Some(details) = &mut self.details_window {
            details.hide();
        }

        if let Some(settings) = &mut self.settings_dialog {
            settings.hide();
        }
    }

    pub fn quit(&mut self) {
        self.hide_windows();
        self.application.quit();
        info!("Application quit requested");
    }
}

pub fn format_device_display_text(device: &Device) -> String {
    match device.battery_level {
        Some(level) => format!("{}: {}%", device.name, level),
        None => format!("{}: --", device.name),
    }
}

pub fn get_device_icon_name(device_type: &DeviceType) -> &'static str {
    match device_type {
        DeviceType::Mouse => "input-mouse-symbolic",
        DeviceType::Keyboard => "input-keyboard-symbolic",
        DeviceType::Mobile => "phone-symbolic",
        DeviceType::Buds | DeviceType::Headphones => "audio-headphones-symbolic",
        DeviceType::Tablet => "computer-tablet-symbolic",
        DeviceType::Unknown => "battery-symbolic",
    }
}

pub fn get_connection_status_text(device: &Device) -> String {
    match device.connection_status {
        ConnectionStatus::Connected => "Connected".to_string(),
        ConnectionStatus::Disconnected => "Disconnected".to_string(),
    }
}

pub fn get_connection_type_text(device: &Device) -> &'static str {
    match device.connection_type {
        battery_monitor_core::ConnectionType::Bluetooth => "Bluetooth",
        battery_monitor_core::ConnectionType::USB => "USB",
        battery_monitor_core::ConnectionType::Wireless2_4G => "Wireless 2.4G",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use battery_monitor_core::{ConnectionStatus, ConnectionType};
    use std::time::SystemTime;

    fn create_test_device() -> Device {
        Device {
            id: "test-device".to_string(),
            name: "Test Mouse".to_string(),
            device_type: DeviceType::Mouse,
            connection_type: ConnectionType::Bluetooth,
            battery_level: Some(75),
            connection_status: ConnectionStatus::Connected,
            last_seen: SystemTime::now(),
        }
    }

    #[test]
    fn test_device_display_formatting() {
        let device = create_test_device();
        let display_text = format_device_display_text(&device);
        assert_eq!(display_text, "Test Mouse: 75%");

        let mut device_no_battery = device;
        device_no_battery.battery_level = None;
        let display_text_no_battery = format_device_display_text(&device_no_battery);
        assert_eq!(display_text_no_battery, "Test Mouse: --");
    }

    #[test]
    fn test_icon_name_mapping() {
        assert_eq!(
            get_device_icon_name(&DeviceType::Mouse),
            "input-mouse-symbolic"
        );
        assert_eq!(
            get_device_icon_name(&DeviceType::Keyboard),
            "input-keyboard-symbolic"
        );
        assert_eq!(get_device_icon_name(&DeviceType::Mobile), "phone-symbolic");
        assert_eq!(
            get_device_icon_name(&DeviceType::Headphones),
            "audio-headphones-symbolic"
        );
        assert_eq!(
            get_device_icon_name(&DeviceType::Unknown),
            "battery-symbolic"
        );
    }

    #[test]
    fn test_connection_status_text() {
        let mut device = create_test_device();

        device.connection_status = ConnectionStatus::Connected;
        assert_eq!(get_connection_status_text(&device), "Connected");

        device.connection_status = ConnectionStatus::Disconnected;
        assert_eq!(get_connection_status_text(&device), "Disconnected");
    }

    #[test]
    fn test_connection_type_text() {
        let mut device = create_test_device();

        device.connection_type = ConnectionType::Bluetooth;
        assert_eq!(get_connection_type_text(&device), "Bluetooth");

        device.connection_type = ConnectionType::USB;
        assert_eq!(get_connection_type_text(&device), "USB");

        device.connection_type = ConnectionType::Wireless2_4G;
        assert_eq!(get_connection_type_text(&device), "Wireless 2.4G");
    }
}
