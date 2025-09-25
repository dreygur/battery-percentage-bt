use crate::{GuiError, format_device_display_text, get_device_icon_name};
use battery_monitor_core::{Device, ConnectionStatus};
use battery_monitor_config::{Config, ConfigError};
use gtk4::prelude::*;
use gtk4::{Box, Button, Image, Label, Orientation, Popover, ListBox, ListBoxRow, Window, Dialog, Grid, SpinButton, Switch, Entry};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use tracing::{debug, error, info, warn};

pub struct TrayIcon {
    devices: Arc<Mutex<HashMap<String, Device>>>,
    popover: Option<Popover>,
    device_buttons: Vec<Button>,
}

impl TrayIcon {
    pub fn new(devices: Arc<Mutex<HashMap<String, Device>>>) -> Result<Self, GuiError> {
        Ok(Self {
            devices,
            popover: None,
            device_buttons: Vec::new(),
        })
    }

    pub fn create_tray_widget(&mut self) -> Result<Box, GuiError> {
        let main_box = Box::new(Orientation::Horizontal, 4);
        main_box.set_margin_start(8);
        main_box.set_margin_end(8);
        main_box.set_margin_top(4);
        main_box.set_margin_bottom(4);

        let devices = self.devices.lock().unwrap();
        let connected_devices: Vec<_> = devices
            .values()
            .filter(|device| device.connection_status == ConnectionStatus::Connected)
            .collect();

        if connected_devices.is_empty() {
            let no_devices_label = Label::new(Some("No devices"));
            no_devices_label.set_css_classes(&["dim-label"]);
            main_box.append(&no_devices_label);
            return Ok(main_box);
        }

        for device in connected_devices {
            let device_button = self.create_device_button(device)?;
            main_box.append(&device_button);
            self.device_buttons.push(device_button);
        }

        debug!("Created tray widget with {} devices", self.device_buttons.len());
        Ok(main_box)
    }

    fn create_device_button(&self, device: &Device) -> Result<Button, GuiError> {
        let button = Button::new();
        button.set_has_frame(false);
        button.set_css_classes(&["flat", "device-button"]);

        let button_box = Box::new(Orientation::Horizontal, 4);

        let icon_name = get_device_icon_name(&device.device_type);
        let icon = Image::from_icon_name(icon_name);
        icon.set_icon_size(gtk4::IconSize::Small);
        button_box.append(&icon);

        let battery_text = match device.battery_level {
            Some(level) => format!("{}%", level),
            None => "--".to_string(),
        };

        let battery_label = Label::new(Some(&battery_text));
        battery_label.set_css_classes(&["battery-text"]);

        if let Some(level) = device.battery_level {
            if level <= 20 {
                battery_label.add_css_class("battery-low");
            } else if level <= 50 {
                battery_label.add_css_class("battery-medium");
            } else {
                battery_label.add_css_class("battery-high");
            }
        }

        button_box.append(&battery_label);
        button.set_child(Some(&button_box));

        let device_id = device.id.clone();
        let devices_clone = Arc::clone(&self.devices);

        button.connect_clicked(move |button| {
            Self::on_device_button_clicked(button, &device_id, Arc::clone(&devices_clone));
        });

        Ok(button)
    }

    fn on_device_button_clicked(
        button: &Button,
        device_id: &str,
        devices: Arc<Mutex<HashMap<String, Device>>>
    ) {
        let devices_guard = devices.lock().unwrap();
        if let Some(device) = devices_guard.get(device_id) {
            info!("Device button clicked: {} ({}%)",
                  device.name,
                  device.battery_level.map_or("--".to_string(), |l| l.to_string()));

            Self::show_device_details_popup(button, device);
        }
    }

    fn show_device_details_popup(relative_to: &Button, device: &Device) {
        let popover = Popover::new();
        popover.set_parent(relative_to);
        popover.set_position(gtk4::PositionType::Bottom);

        let content_box = Box::new(Orientation::Vertical, 8);
        content_box.set_margin_start(16);
        content_box.set_margin_end(16);
        content_box.set_margin_top(12);
        content_box.set_margin_bottom(12);

        // Device name as title
        let name_label = Label::new(Some(&device.name));
        name_label.set_css_classes(&["heading"]);
        name_label.set_halign(gtk4::Align::Start);
        content_box.append(&name_label);

        // Device details list
        let details_box = Box::new(Orientation::Vertical, 4);

        // Device type
        let type_row = Box::new(Orientation::Horizontal, 8);
        let type_label = Label::new(Some("Type:"));
        type_label.set_css_classes(&["dim-label"]);
        type_label.set_halign(gtk4::Align::Start);
        type_label.set_size_request(80, -1);
        type_row.append(&type_label);

        let type_value = Label::new(Some(&format!("{:?}", device.device_type)));
        type_value.set_halign(gtk4::Align::Start);
        type_row.append(&type_value);
        details_box.append(&type_row);

        // Connection status
        let status_row = Box::new(Orientation::Horizontal, 8);
        let status_label = Label::new(Some("Status:"));
        status_label.set_css_classes(&["dim-label"]);
        status_label.set_halign(gtk4::Align::Start);
        status_label.set_size_request(80, -1);
        status_row.append(&status_label);

        let status_text = match device.connection_status {
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Disconnected => "Disconnected",
        };
        let status_value = Label::new(Some(status_text));
        status_value.set_halign(gtk4::Align::Start);
        if device.connection_status == ConnectionStatus::Connected {
            status_value.set_css_classes(&["success"]);
        } else {
            status_value.set_css_classes(&["dim-label"]);
        }
        status_row.append(&status_value);
        details_box.append(&status_row);

        // Battery level
        if let Some(level) = device.battery_level {
            let battery_row = Box::new(Orientation::Horizontal, 8);
            let battery_label = Label::new(Some("Battery:"));
            battery_label.set_css_classes(&["dim-label"]);
            battery_label.set_halign(gtk4::Align::Start);
            battery_label.set_size_request(80, -1);
            battery_row.append(&battery_label);

            let battery_value = Label::new(Some(&format!("{}%", level)));
            battery_value.set_halign(gtk4::Align::Start);

            if level <= 20 {
                battery_value.set_css_classes(&["battery-low"]);
            } else if level <= 50 {
                battery_value.set_css_classes(&["battery-medium"]);
            } else {
                battery_value.set_css_classes(&["battery-high"]);
            }

            battery_row.append(&battery_value);
            details_box.append(&battery_row);
        }

        // Device ID (for debugging/advanced info)
        let id_row = Box::new(Orientation::Horizontal, 8);
        let id_label = Label::new(Some("ID:"));
        id_label.set_css_classes(&["dim-label"]);
        id_label.set_halign(gtk4::Align::Start);
        id_label.set_size_request(80, -1);
        id_row.append(&id_label);

        let id_value = Label::new(Some(&device.id));
        id_value.set_halign(gtk4::Align::Start);
        id_value.set_css_classes(&["caption", "dim-label"]);
        id_value.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        id_value.set_max_width_chars(20);
        id_row.append(&id_value);
        details_box.append(&id_row);

        content_box.append(&details_box);
        popover.set_child(Some(&content_box));

        popover.popup();
        debug!("Device details popup shown for: {}", device.name);
    }

    fn show_settings_dialog(relative_to: &impl IsA<gtk4::Widget>) {
        info!("Settings dialog requested");

        // Load current configuration
        let config = match Config::load() {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load configuration: {}", e);
                Self::show_error_dialog(relative_to, "Failed to load settings", &format!("Error: {}", e));
                return;
            }
        };

        // Create settings dialog window
        let dialog = Dialog::builder()
            .title("Battery Monitor Settings")
            .modal(true)
            .default_width(500)
            .default_height(400)
            .build();

        // Get parent window if available
        if let Some(widget) = relative_to.ancestor(Window::static_type()) {
            if let Ok(window) = widget.downcast::<Window>() {
                dialog.set_transient_for(Some(&window));
            }
        }

        // Create main content
        let content_box = Box::new(Orientation::Vertical, 16);
        content_box.set_margin_start(24);
        content_box.set_margin_end(24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);

        // Monitoring Section
        let monitoring_frame = gtk4::Frame::builder()
            .label("Monitoring")
            .build();

        let monitoring_grid = Grid::new();
        monitoring_grid.set_row_spacing(8);
        monitoring_grid.set_column_spacing(16);
        monitoring_grid.set_margin_start(16);
        monitoring_grid.set_margin_end(16);
        monitoring_grid.set_margin_top(12);
        monitoring_grid.set_margin_bottom(16);

        // Polling interval
        let polling_label = Label::new(Some("Polling interval (seconds):"));
        polling_label.set_halign(gtk4::Align::Start);
        monitoring_grid.attach(&polling_label, 0, 0, 1, 1);

        let polling_spin = SpinButton::with_range(5.0, 300.0, 5.0);
        polling_spin.set_value(config.monitoring.polling_interval_seconds as f64);
        polling_spin.set_hexpand(true);
        monitoring_grid.attach(&polling_spin, 1, 0, 1, 1);

        // Auto-start
        let autostart_label = Label::new(Some("Start automatically:"));
        autostart_label.set_halign(gtk4::Align::Start);
        monitoring_grid.attach(&autostart_label, 0, 1, 1, 1);

        let autostart_switch = Switch::new();
        autostart_switch.set_active(config.monitoring.auto_start);
        autostart_switch.set_halign(gtk4::Align::Start);
        monitoring_grid.attach(&autostart_switch, 1, 1, 1, 1);

        monitoring_frame.set_child(Some(&monitoring_grid));
        content_box.append(&monitoring_frame);

        // Notifications Section
        let notifications_frame = gtk4::Frame::builder()
            .label("Notifications")
            .build();

        let notifications_grid = Grid::new();
        notifications_grid.set_row_spacing(8);
        notifications_grid.set_column_spacing(16);
        notifications_grid.set_margin_start(16);
        notifications_grid.set_margin_end(16);
        notifications_grid.set_margin_top(12);
        notifications_grid.set_margin_bottom(16);

        // Enable notifications
        let notifications_enabled_label = Label::new(Some("Enable notifications:"));
        notifications_enabled_label.set_halign(gtk4::Align::Start);
        notifications_grid.attach(&notifications_enabled_label, 0, 0, 1, 1);

        let notifications_enabled_switch = Switch::new();
        notifications_enabled_switch.set_active(config.notifications.enabled);
        notifications_enabled_switch.set_halign(gtk4::Align::Start);
        notifications_grid.attach(&notifications_enabled_switch, 1, 0, 1, 1);

        // Low battery threshold
        let threshold_label = Label::new(Some("Low battery threshold (%):"));
        threshold_label.set_halign(gtk4::Align::Start);
        notifications_grid.attach(&threshold_label, 0, 1, 1, 1);

        let threshold_spin = SpinButton::with_range(1.0, 99.0, 1.0);
        threshold_spin.set_value(config.notifications.low_battery_threshold as f64);
        threshold_spin.set_hexpand(true);
        notifications_grid.attach(&threshold_spin, 1, 1, 1, 1);

        // Show connect/disconnect
        let connect_disconnect_label = Label::new(Some("Show connect/disconnect:"));
        connect_disconnect_label.set_halign(gtk4::Align::Start);
        notifications_grid.attach(&connect_disconnect_label, 0, 2, 1, 1);

        let connect_disconnect_switch = Switch::new();
        connect_disconnect_switch.set_active(config.notifications.show_connect_disconnect);
        connect_disconnect_switch.set_halign(gtk4::Align::Start);
        notifications_grid.attach(&connect_disconnect_switch, 1, 2, 1, 1);

        // Suppression minutes
        let suppression_label = Label::new(Some("Suppression time (minutes):"));
        suppression_label.set_halign(gtk4::Align::Start);
        notifications_grid.attach(&suppression_label, 0, 3, 1, 1);

        let suppression_spin = SpinButton::with_range(1.0, 60.0, 1.0);
        suppression_spin.set_value(config.notifications.suppression_minutes as f64);
        suppression_spin.set_hexpand(true);
        notifications_grid.attach(&suppression_spin, 1, 3, 1, 1);

        notifications_frame.set_child(Some(&notifications_grid));
        content_box.append(&notifications_frame);

        // UI Section
        let ui_frame = gtk4::Frame::builder()
            .label("User Interface")
            .build();

        let ui_grid = Grid::new();
        ui_grid.set_row_spacing(8);
        ui_grid.set_column_spacing(16);
        ui_grid.set_margin_start(16);
        ui_grid.set_margin_end(16);
        ui_grid.set_margin_top(12);
        ui_grid.set_margin_bottom(16);

        // Show disconnected devices
        let show_disconnected_label = Label::new(Some("Show disconnected devices:"));
        show_disconnected_label.set_halign(gtk4::Align::Start);
        ui_grid.attach(&show_disconnected_label, 0, 0, 1, 1);

        let show_disconnected_switch = Switch::new();
        show_disconnected_switch.set_active(config.ui.show_disconnected_devices);
        show_disconnected_switch.set_halign(gtk4::Align::Start);
        ui_grid.attach(&show_disconnected_switch, 1, 0, 1, 1);

        ui_frame.set_child(Some(&ui_grid));
        content_box.append(&ui_frame);

        // Buttons
        let button_box = Box::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);
        button_box.set_margin_top(16);

        let cancel_button = Button::with_label("Cancel");
        let dialog_weak = dialog.downgrade();
        cancel_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.close();
            }
        });
        button_box.append(&cancel_button);

        let save_button = Button::with_label("Save");
        save_button.set_css_classes(&["suggested-action"]);

        // Clone widgets for the save callback
        let polling_spin_clone = polling_spin.clone();
        let autostart_switch_clone = autostart_switch.clone();
        let notifications_enabled_switch_clone = notifications_enabled_switch.clone();
        let threshold_spin_clone = threshold_spin.clone();
        let connect_disconnect_switch_clone = connect_disconnect_switch.clone();
        let suppression_spin_clone = suppression_spin.clone();
        let show_disconnected_switch_clone = show_disconnected_switch.clone();
        let dialog_weak = dialog.downgrade();

        save_button.connect_clicked(move |_| {
            Self::save_settings_and_close(
                &dialog_weak,
                &polling_spin_clone,
                &autostart_switch_clone,
                &notifications_enabled_switch_clone,
                &threshold_spin_clone,
                &connect_disconnect_switch_clone,
                &suppression_spin_clone,
                &show_disconnected_switch_clone,
            );
        });
        button_box.append(&save_button);

        content_box.append(&button_box);

        dialog.set_child(Some(&content_box));
        dialog.present();
        debug!("Settings dialog shown");
    }

    fn save_settings_and_close(
        dialog_weak: &glib::WeakRef<Dialog>,
        polling_spin: &SpinButton,
        autostart_switch: &Switch,
        notifications_enabled_switch: &Switch,
        threshold_spin: &SpinButton,
        connect_disconnect_switch: &Switch,
        suppression_spin: &SpinButton,
        show_disconnected_switch: &Switch,
    ) {
        // Create new configuration from dialog values
        let mut new_config = Config::default();

        new_config.monitoring.polling_interval_seconds = polling_spin.value() as u64;
        new_config.monitoring.auto_start = autostart_switch.is_active();

        new_config.notifications.enabled = notifications_enabled_switch.is_active();
        new_config.notifications.low_battery_threshold = threshold_spin.value() as u8;
        new_config.notifications.show_connect_disconnect = connect_disconnect_switch.is_active();
        new_config.notifications.suppression_minutes = suppression_spin.value() as u64;

        new_config.ui.show_disconnected_devices = show_disconnected_switch.is_active();

        // Save configuration
        match new_config.save() {
            Ok(()) => {
                info!("Settings saved successfully");
                if let Some(dialog) = dialog_weak.upgrade() {
                    dialog.close();
                }
            }
            Err(e) => {
                error!("Failed to save settings: {}", e);
                if let Some(dialog) = dialog_weak.upgrade() {
                    Self::show_error_dialog(&dialog, "Save Failed", &format!("Failed to save settings: {}", e));
                }
            }
        }
    }

    fn show_error_dialog(relative_to: &impl IsA<gtk4::Widget>, title: &str, message: &str) {
        let error_dialog = Dialog::builder()
            .title(title)
            .modal(true)
            .default_width(400)
            .default_height(200)
            .build();

        // Get parent window if available
        if let Some(widget) = relative_to.ancestor(Window::static_type()) {
            if let Ok(window) = widget.downcast::<Window>() {
                error_dialog.set_transient_for(Some(&window));
            }
        }

        let content_box = Box::new(Orientation::Vertical, 16);
        content_box.set_margin_start(24);
        content_box.set_margin_end(24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);

        let message_label = Label::new(Some(message));
        message_label.set_wrap(true);
        message_label.set_halign(gtk4::Align::Start);
        content_box.append(&message_label);

        let button_box = Box::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);

        let ok_button = Button::with_label("OK");
        ok_button.set_css_classes(&["suggested-action"]);
        let error_dialog_weak = error_dialog.downgrade();
        ok_button.connect_clicked(move |_| {
            if let Some(dialog) = error_dialog_weak.upgrade() {
                dialog.close();
            }
        });
        button_box.append(&ok_button);

        content_box.append(&button_box);
        error_dialog.set_child(Some(&content_box));
        error_dialog.present();
    }

    fn show_details_window(relative_to: &impl IsA<gtk4::Widget>, devices: Arc<Mutex<HashMap<String, Device>>>) {
        info!("Details window requested");

        // Create details window
        let window = Window::builder()
            .title("Battery Monitor - Device Details")
            .default_width(600)
            .default_height(500)
            .build();

        // Get parent window if available
        if let Some(widget) = relative_to.ancestor(Window::static_type()) {
            if let Ok(parent_window) = widget.downcast::<Window>() {
                window.set_transient_for(Some(&parent_window));
                window.set_modal(true);
            }
        }

        // Create main content
        let main_box = Box::new(Orientation::Vertical, 0);

        // Header bar
        let header_bar = gtk4::HeaderBar::new();
        header_bar.set_title_widget(Some(&Label::new(Some("Device Details"))));

        // Refresh button in header
        let refresh_button = Button::from_icon_name("view-refresh-symbolic");
        refresh_button.set_tooltip_text(Some("Refresh device list"));
        let devices_clone = Arc::clone(&devices);
        let window_weak = window.downgrade();
        refresh_button.connect_clicked(move |_| {
            info!("Refresh button clicked in details window");
            if let Some(window) = window_weak.upgrade() {
                Self::refresh_details_window(&window, Arc::clone(&devices_clone));
            }
        });
        header_bar.pack_end(&refresh_button);

        window.set_titlebar(Some(&header_bar));

        // Scrollable content area
        let scrolled_window = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .build();

        let content_box = Box::new(Orientation::Vertical, 16);
        content_box.set_margin_start(24);
        content_box.set_margin_end(24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);

        // Create device cards
        let devices_guard = devices.lock().unwrap();
        let mut device_list: Vec<_> = devices_guard.values().collect();
        device_list.sort_by(|a, b| {
            match (a.connection_status, b.connection_status) {
                (ConnectionStatus::Connected, ConnectionStatus::Disconnected) => std::cmp::Ordering::Less,
                (ConnectionStatus::Disconnected, ConnectionStatus::Connected) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        drop(devices_guard);

        if device_list.is_empty() {
            // Empty state
            let empty_box = Box::new(Orientation::Vertical, 16);
            empty_box.set_valign(gtk4::Align::Center);
            empty_box.set_halign(gtk4::Align::Center);
            empty_box.set_vexpand(true);

            let empty_icon = Image::from_icon_name("battery-missing-symbolic");
            empty_icon.set_icon_size(gtk4::IconSize::Large);
            empty_icon.set_css_classes(&["dim-label"]);
            empty_box.append(&empty_icon);

            let empty_label = Label::new(Some("No devices found"));
            empty_label.set_css_classes(&["title-2", "dim-label"]);
            empty_box.append(&empty_label);

            let empty_subtitle = Label::new(Some("Make sure your devices are paired and turned on"));
            empty_subtitle.set_css_classes(&["dim-label"]);
            empty_box.append(&empty_subtitle);

            content_box.append(&empty_box);
        } else {
            // Connected devices section
            let connected_devices: Vec<_> = device_list.iter()
                .filter(|device| device.connection_status == ConnectionStatus::Connected)
                .collect();

            if !connected_devices.is_empty() {
                let connected_label = Label::new(Some("Connected Devices"));
                connected_label.set_css_classes(&["heading"]);
                connected_label.set_halign(gtk4::Align::Start);
                connected_label.set_margin_bottom(8);
                content_box.append(&connected_label);

                for device in connected_devices {
                    let device_card = Self::create_device_detail_card(device, true);
                    content_box.append(&device_card);
                }
            }

            // Disconnected devices section
            let disconnected_devices: Vec<_> = device_list.iter()
                .filter(|device| device.connection_status == ConnectionStatus::Disconnected)
                .collect();

            if !disconnected_devices.is_empty() {
                if !connected_devices.is_empty() {
                    content_box.append(&gtk4::Separator::new(Orientation::Horizontal));
                }

                let disconnected_label = Label::new(Some("Disconnected Devices"));
                disconnected_label.set_css_classes(&["heading", "dim-label"]);
                disconnected_label.set_halign(gtk4::Align::Start);
                disconnected_label.set_margin_top(16);
                disconnected_label.set_margin_bottom(8);
                content_box.append(&disconnected_label);

                for device in disconnected_devices {
                    let device_card = Self::create_device_detail_card(device, false);
                    content_box.append(&device_card);
                }
            }
        }

        scrolled_window.set_child(Some(&content_box));
        main_box.append(&scrolled_window);
        window.set_child(Some(&main_box));

        window.present();
        debug!("Details window shown with {} devices", device_list.len());
    }

    fn create_device_detail_card(device: &Device, is_connected: bool) -> gtk4::Frame {
        let frame = gtk4::Frame::new(None);
        frame.set_margin_bottom(8);

        let card_box = Box::new(Orientation::Horizontal, 16);
        card_box.set_margin_start(16);
        card_box.set_margin_end(16);
        card_box.set_margin_top(12);
        card_box.set_margin_bottom(12);

        // Device icon
        let icon_name = get_device_icon_name(&device.device_type);
        let icon = Image::from_icon_name(icon_name);
        icon.set_icon_size(gtk4::IconSize::Large);

        if !is_connected {
            icon.set_opacity(0.5);
        }

        card_box.append(&icon);

        // Device information
        let info_box = Box::new(Orientation::Vertical, 4);
        info_box.set_hexpand(true);

        let name_label = Label::new(Some(&device.name));
        name_label.set_halign(gtk4::Align::Start);
        name_label.set_css_classes(&["title-4"]);

        if !is_connected {
            name_label.set_opacity(0.7);
        }

        info_box.append(&name_label);

        // Device details
        let details_grid = Grid::new();
        details_grid.set_row_spacing(4);
        details_grid.set_column_spacing(16);

        let mut row = 0;

        // Device type
        let type_label = Label::new(Some("Type:"));
        type_label.set_halign(gtk4::Align::Start);
        type_label.set_css_classes(&["caption", "dim-label"]);
        details_grid.attach(&type_label, 0, row, 1, 1);

        let type_value = Label::new(Some(&format!("{:?}", device.device_type)));
        type_value.set_halign(gtk4::Align::Start);
        type_value.set_css_classes(&["caption"]);
        details_grid.attach(&type_value, 1, row, 1, 1);
        row += 1;

        // Connection status
        let status_label = Label::new(Some("Status:"));
        status_label.set_halign(gtk4::Align::Start);
        status_label.set_css_classes(&["caption", "dim-label"]);
        details_grid.attach(&status_label, 0, row, 1, 1);

        let status_text = match device.connection_status {
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Disconnected => "Disconnected",
        };
        let status_value = Label::new(Some(status_text));
        status_value.set_halign(gtk4::Align::Start);
        status_value.set_css_classes(&["caption"]);
        if is_connected {
            status_value.add_css_class("success");
        } else {
            status_value.add_css_class("dim-label");
        }
        details_grid.attach(&status_value, 1, row, 1, 1);
        row += 1;

        // Device ID
        let id_label = Label::new(Some("Device ID:"));
        id_label.set_halign(gtk4::Align::Start);
        id_label.set_css_classes(&["caption", "dim-label"]);
        details_grid.attach(&id_label, 0, row, 1, 1);

        let id_value = Label::new(Some(&device.id));
        id_value.set_halign(gtk4::Align::Start);
        id_value.set_css_classes(&["caption", "dim-label"]);
        id_value.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        id_value.set_max_width_chars(30);
        details_grid.attach(&id_value, 1, row, 1, 1);

        info_box.append(&details_grid);
        card_box.append(&info_box);

        // Battery level (right side)
        if is_connected {
            if let Some(level) = device.battery_level {
                let battery_box = Box::new(Orientation::Vertical, 4);
                battery_box.set_valign(gtk4::Align::Center);

                let battery_label = Label::new(Some(&format!("{}%", level)));
                battery_label.set_css_classes(&["title-1"]);

                if level <= 20 {
                    battery_label.add_css_class("battery-low");
                } else if level <= 50 {
                    battery_label.add_css_class("battery-medium");
                } else {
                    battery_label.add_css_class("battery-high");
                }

                battery_box.append(&battery_label);

                let battery_caption = Label::new(Some("Battery"));
                battery_caption.set_css_classes(&["caption", "dim-label"]);
                battery_box.append(&battery_caption);

                card_box.append(&battery_box);
            } else {
                let no_battery_label = Label::new(Some("--"));
                no_battery_label.set_css_classes(&["title-1", "dim-label"]);
                no_battery_label.set_valign(gtk4::Align::Center);
                card_box.append(&no_battery_label);
            }
        }

        frame.set_child(Some(&card_box));
        frame
    }

    fn refresh_details_window(window: &Window, devices: Arc<Mutex<HashMap<String, Device>>>) {
        info!("Refreshing details window");
        // In a real implementation, this would trigger the device monitor to refresh
        // and then recreate the window content

        // For now, just close and reopen the window
        window.close();

        // Create a dummy button to use as relative_to (not ideal, but works for demo)
        let dummy_button = Button::new();
        Self::show_details_window(&dummy_button, devices);
    }

    pub fn update_devices(&mut self) {
        debug!("Updating tray devices display");
        self.device_buttons.clear();

        // In a real implementation, this would recreate the tray widget
        // For now, we'll just log the update
        let devices = self.devices.lock().unwrap();
        let connected_count = devices
            .values()
            .filter(|device| device.connection_status == ConnectionStatus::Connected)
            .count();

        info!("Tray updated: {} connected devices", connected_count);
    }

    pub fn create_details_popover(&mut self, relative_to: &impl IsA<gtk4::Widget>) -> Result<Popover, GuiError> {
        let popover = Popover::new();
        popover.set_parent(relative_to);
        popover.set_position(gtk4::PositionType::Bottom);

        let content_box = Box::new(Orientation::Vertical, 8);
        content_box.set_margin_start(12);
        content_box.set_margin_end(12);
        content_box.set_margin_top(8);
        content_box.set_margin_bottom(8);

        let title_label = Label::new(Some("Battery Monitor"));
        title_label.set_css_classes(&["heading"]);
        content_box.append(&title_label);

        let devices_list = self.create_devices_list()?;
        content_box.append(&devices_list);

        let button_box = Box::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);

        let details_button = Button::with_label("Details...");
        let devices_clone = Arc::clone(&self.devices);
        let popover_weak = Rc::downgrade(&Rc::new(popover.clone()));
        details_button.connect_clicked(move |button| {
            if let Some(popover) = popover_weak.upgrade() {
                popover.popdown();
            }
            Self::show_details_window(button, Arc::clone(&devices_clone));
        });
        button_box.append(&details_button);

        let settings_button = Button::with_label("Settings...");
        let popover_weak = Rc::downgrade(&Rc::new(popover.clone()));
        settings_button.connect_clicked(move |button| {
            if let Some(popover) = popover_weak.upgrade() {
                popover.popdown();
            }
            Self::show_settings_dialog(button);
        });
        button_box.append(&settings_button);

        content_box.append(&button_box);
        popover.set_child(Some(&content_box));

        self.popover = Some(popover.clone());
        Ok(popover)
    }

    fn create_devices_list(&self) -> Result<ListBox, GuiError> {
        let list_box = ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::None);
        list_box.set_css_classes(&["boxed-list"]);

        let devices = self.devices.lock().unwrap();
        let mut device_list: Vec<_> = devices.values().collect();
        device_list.sort_by(|a, b| {
            match (a.connection_status, b.connection_status) {
                (ConnectionStatus::Connected, ConnectionStatus::Disconnected) => std::cmp::Ordering::Less,
                (ConnectionStatus::Disconnected, ConnectionStatus::Connected) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        for device in device_list {
            let row = self.create_device_list_row(device)?;
            list_box.append(&row);
        }

        if devices.is_empty() {
            let empty_row = ListBoxRow::new();
            let empty_label = Label::new(Some("No devices found"));
            empty_label.set_css_classes(&["dim-label"]);
            empty_label.set_margin_start(12);
            empty_label.set_margin_end(12);
            empty_label.set_margin_top(8);
            empty_label.set_margin_bottom(8);
            empty_row.set_child(Some(&empty_label));
            list_box.append(&empty_row);
        }

        Ok(list_box)
    }

    fn create_device_list_row(&self, device: &Device) -> Result<ListBoxRow, GuiError> {
        let row = ListBoxRow::new();
        row.set_activatable(false);

        let row_box = Box::new(Orientation::Horizontal, 12);
        row_box.set_margin_start(12);
        row_box.set_margin_end(12);
        row_box.set_margin_top(8);
        row_box.set_margin_bottom(8);

        let icon_name = get_device_icon_name(&device.device_type);
        let icon = Image::from_icon_name(icon_name);
        icon.set_icon_size(gtk4::IconSize::Normal);

        if device.connection_status == ConnectionStatus::Disconnected {
            icon.set_opacity(0.5);
        }

        row_box.append(&icon);

        let info_box = Box::new(Orientation::Vertical, 2);
        info_box.set_hexpand(true);

        let name_label = Label::new(Some(&device.name));
        name_label.set_halign(gtk4::Align::Start);
        name_label.set_css_classes(&["device-name"]);

        if device.connection_status == ConnectionStatus::Disconnected {
            name_label.set_opacity(0.5);
        }

        info_box.append(&name_label);

        let status_text = match device.connection_status {
            ConnectionStatus::Connected => "Connected".to_string(),
            ConnectionStatus::Disconnected => "Disconnected".to_string(),
        };

        let status_label = Label::new(Some(&status_text));
        status_label.set_halign(gtk4::Align::Start);
        status_label.set_css_classes(&["caption", "dim-label"]);
        info_box.append(&status_label);

        row_box.append(&info_box);

        if device.connection_status == ConnectionStatus::Connected {
            let battery_text = match device.battery_level {
                Some(level) => format!("{}%", level),
                None => "--".to_string(),
            };

            let battery_label = Label::new(Some(&battery_text));
            battery_label.set_css_classes(&["battery-percentage"]);

            if let Some(level) = device.battery_level {
                if level <= 20 {
                    battery_label.add_css_class("battery-low");
                } else if level <= 50 {
                    battery_label.add_css_class("battery-medium");
                } else {
                    battery_label.add_css_class("battery-high");
                }
            }

            row_box.append(&battery_label);
        }

        row.set_child(Some(&row_box));
        Ok(row)
    }

    pub fn show_popover(&mut self) {
        if let Some(popover) = &self.popover {
            popover.popup();
            debug!("Tray popover shown");
        }
    }

    pub fn hide_popover(&mut self) {
        if let Some(popover) = &self.popover {
            popover.popdown();
            debug!("Tray popover hidden");
        }
    }
}

impl Default for TrayIcon {
    fn default() -> Self {
        Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
            popover: None,
            device_buttons: Vec::new(),
        }
    }
}
