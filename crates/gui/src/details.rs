use crate::{
    get_connection_status_text, get_connection_type_text, get_device_icon_name, GuiError,
};
use battery_monitor_config::Config;
use battery_monitor_core::{ConnectionStatus, Device};
use gtk4::prelude::*;
use gtk4::{
    ApplicationWindow, Box, Button, HeaderBar, Image, Label, ListBox, ListBoxRow, Orientation,
    ScrolledWindow,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info};

pub struct DetailsWindow {
    window: ApplicationWindow,
    devices: Arc<Mutex<HashMap<String, Device>>>,
    config: Arc<Mutex<Config>>,
    device_list: ListBox,
}

impl DetailsWindow {
    pub fn new(
        devices: Arc<Mutex<HashMap<String, Device>>>,
        config: Arc<Mutex<Config>>,
    ) -> Result<Self, GuiError> {
        let window = ApplicationWindow::new(&gtk4::Application::default());
        window.set_title(Some("Battery Monitor - Device Details"));
        window.set_default_size(500, 400);
        window.set_resizable(true);

        let header_bar = HeaderBar::new();
        header_bar.set_title_widget(Some(&Label::new(Some("Device Details"))));
        window.set_titlebar(Some(&header_bar));

        let main_box = Box::new(Orientation::Vertical, 0);

        let device_list = ListBox::new();
        device_list.set_selection_mode(gtk4::SelectionMode::None);
        device_list.set_css_classes(&["boxed-list"]);

        let scrolled = ScrolledWindow::new();
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scrolled.set_child(Some(&device_list));
        scrolled.set_vexpand(true);

        main_box.append(&scrolled);

        let button_box = Box::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);
        button_box.set_margin_start(12);
        button_box.set_margin_end(12);
        button_box.set_margin_top(8);
        button_box.set_margin_bottom(8);

        let refresh_button = Button::with_label("Refresh");
        let devices_clone = Arc::clone(&devices);
        let device_list_clone = device_list.clone();
        let config_clone = Arc::clone(&config);

        refresh_button.connect_clicked(move |_| {
            info!("Manual device refresh requested from details window");

            // Simulate refresh by clearing and repopulating the list
            // In a real implementation, this would trigger the device monitor
            Self::refresh_device_list(
                &device_list_clone,
                Arc::clone(&devices_clone),
                Arc::clone(&config_clone),
            );
        });
        button_box.append(&refresh_button);

        let close_button = Button::with_label("Close");
        close_button.connect_clicked({
            let window = window.clone();
            move |_| {
                window.close();
            }
        });
        button_box.append(&close_button);

        main_box.append(&button_box);
        window.set_child(Some(&main_box));

        let mut details_window = Self {
            window,
            devices,
            config,
            device_list,
        };

        details_window.populate_device_list()?;

        Ok(details_window)
    }

    pub fn show(&self) {
        self.window.present();
        debug!("Details window shown");
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
        debug!("Details window hidden");
    }

    pub fn update_devices(&mut self) {
        if let Err(e) = self.populate_device_list() {
            error!("Failed to update device list: {}", e);
        }
    }

    fn populate_device_list(&mut self) -> Result<(), GuiError> {
        while let Some(child) = self.device_list.first_child() {
            self.device_list.remove(&child);
        }

        let devices = self.devices.lock().unwrap();
        let config = self.config.lock().unwrap();

        if devices.is_empty() {
            let empty_row = self.create_empty_state_row()?;
            self.device_list.append(&empty_row);
            return Ok(());
        }

        let mut device_list: Vec<_> = devices.values().collect();
        device_list.sort_by(|a, b| match (a.connection_status, b.connection_status) {
            (ConnectionStatus::Connected, ConnectionStatus::Disconnected) => {
                std::cmp::Ordering::Less
            }
            (ConnectionStatus::Disconnected, ConnectionStatus::Connected) => {
                std::cmp::Ordering::Greater
            }
            _ => a.name.cmp(&b.name),
        });

        for device in device_list {
            if device.connection_status == ConnectionStatus::Connected
                || config.ui.show_disconnected_devices
            {
                let row = self.create_device_detail_row(device)?;
                self.device_list.append(&row);
            }
        }

        debug!("Device list populated with {} devices", devices.len());
        Ok(())
    }

    fn create_empty_state_row(&self) -> Result<ListBoxRow, GuiError> {
        let row = ListBoxRow::new();
        row.set_activatable(false);
        row.set_selectable(false);

        let content_box = Box::new(Orientation::Vertical, 12);
        content_box.set_margin_start(24);
        content_box.set_margin_end(24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);
        content_box.set_halign(gtk4::Align::Center);

        let icon = Image::from_icon_name("battery-missing-symbolic");
        icon.set_icon_size(gtk4::IconSize::Large);
        icon.set_opacity(0.5);
        content_box.append(&icon);

        let title_label = Label::new(Some("No devices found"));
        title_label.set_css_classes(&["title-4"]);
        title_label.set_opacity(0.7);
        content_box.append(&title_label);

        let subtitle_label = Label::new(Some(
            "Make sure your Bluetooth and USB devices are connected",
        ));
        subtitle_label.set_css_classes(&["body", "dim-label"]);
        subtitle_label.set_justify(gtk4::Justification::Center);
        subtitle_label.set_wrap(true);
        content_box.append(&subtitle_label);

        row.set_child(Some(&content_box));
        Ok(row)
    }

    fn create_device_detail_row(&self, device: &Device) -> Result<ListBoxRow, GuiError> {
        let row = ListBoxRow::new();
        row.set_activatable(false);
        row.set_selectable(false);

        let main_box = Box::new(Orientation::Vertical, 8);
        main_box.set_margin_start(16);
        main_box.set_margin_end(16);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);

        let header_box = Box::new(Orientation::Horizontal, 12);

        let icon_name = get_device_icon_name(&device.device_type);
        let icon = Image::from_icon_name(icon_name);
        icon.set_icon_size(gtk4::IconSize::Large);

        if device.connection_status == ConnectionStatus::Disconnected {
            icon.set_opacity(0.5);
        }

        header_box.append(&icon);

        let info_box = Box::new(Orientation::Vertical, 4);
        info_box.set_hexpand(true);

        let name_label = Label::new(Some(&device.name));
        name_label.set_halign(gtk4::Align::Start);
        name_label.set_css_classes(&["heading"]);

        if device.connection_status == ConnectionStatus::Disconnected {
            name_label.set_opacity(0.6);
        }

        info_box.append(&name_label);

        let type_text = format!(
            "{:?} • {}",
            device.device_type,
            get_connection_type_text(device)
        );
        let type_label = Label::new(Some(&type_text));
        type_label.set_halign(gtk4::Align::Start);
        type_label.set_css_classes(&["caption", "dim-label"]);
        info_box.append(&type_label);

        header_box.append(&info_box);

        let status_box = Box::new(Orientation::Vertical, 2);
        status_box.set_halign(gtk4::Align::End);

        let status_text = get_connection_status_text(device);
        let status_label = Label::new(Some(&status_text));
        status_label.set_halign(gtk4::Align::End);

        if device.connection_status == ConnectionStatus::Connected {
            status_label.set_css_classes(&["success"]);
        } else {
            status_label.set_css_classes(&["warning"]);
        }

        status_box.append(&status_label);

        if device.connection_status == ConnectionStatus::Connected {
            let battery_text = match device.battery_level {
                Some(level) => format!("{}%", level),
                None => "Battery info unavailable".to_string(),
            };

            let battery_label = Label::new(Some(&battery_text));
            battery_label.set_halign(gtk4::Align::End);
            battery_label.set_css_classes(&["title-3"]);

            if let Some(level) = device.battery_level {
                if level <= 20 {
                    battery_label.add_css_class("error");
                } else if level <= 50 {
                    battery_label.add_css_class("warning");
                } else {
                    battery_label.add_css_class("success");
                }
            }

            status_box.append(&battery_label);
        }

        header_box.append(&status_box);
        main_box.append(&header_box);

        let details_box = Box::new(Orientation::Horizontal, 16);
        details_box.set_margin_start(52);
        details_box.set_css_classes(&["dim-label"]);

        let device_id_label = Label::new(Some(&format!("ID: {}", device.id)));
        device_id_label.set_css_classes(&["caption", "monospace"]);
        device_id_label.set_selectable(true);
        details_box.append(&device_id_label);

        let last_seen_text = self.format_last_seen(&device.last_seen);
        let last_seen_label = Label::new(Some(&format!("Last seen: {}", last_seen_text)));
        last_seen_label.set_css_classes(&["caption"]);
        details_box.append(&last_seen_label);

        main_box.append(&details_box);

        if device.connection_status == ConnectionStatus::Disconnected {
            main_box.set_opacity(0.7);
        }

        row.set_child(Some(&main_box));
        Ok(row)
    }

    fn format_last_seen(&self, last_seen: &SystemTime) -> String {
        match last_seen.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default();
                let seconds_ago = now.as_secs().saturating_sub(duration.as_secs());

                if seconds_ago < 60 {
                    "Just now".to_string()
                } else if seconds_ago < 3600 {
                    format!("{} minutes ago", seconds_ago / 60)
                } else if seconds_ago < 86400 {
                    format!("{} hours ago", seconds_ago / 3600)
                } else {
                    format!("{} days ago", seconds_ago / 86400)
                }
            }
            Err(_) => "Unknown".to_string(),
        }
    }

    fn refresh_device_list(
        device_list: &ListBox,
        devices: Arc<Mutex<HashMap<String, Device>>>,
        config: Arc<Mutex<Config>>,
    ) {
        info!("Refreshing device list in details window");

        // Clear existing list items
        while let Some(child) = device_list.first_child() {
            device_list.remove(&child);
        }

        // Add loading indicator
        let loading_row = ListBoxRow::new();
        loading_row.set_activatable(false);
        loading_row.set_selectable(false);

        let loading_box = Box::new(Orientation::Horizontal, 12);
        loading_box.set_margin_start(24);
        loading_box.set_margin_end(24);
        loading_box.set_margin_top(24);
        loading_box.set_margin_bottom(24);
        loading_box.set_halign(gtk4::Align::Center);

        let spinner = gtk4::Spinner::new();
        spinner.start();
        loading_box.append(&spinner);

        let loading_label = Label::new(Some("Refreshing device list..."));
        loading_label.set_css_classes(&["dim-label"]);
        loading_box.append(&loading_label);

        loading_row.set_child(Some(&loading_box));
        device_list.append(&loading_row);

        // Simulate async refresh with a timeout
        let device_list_weak = device_list.downgrade();
        let devices_clone = Arc::clone(&devices);
        let config_clone = Arc::clone(&config);

        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(1000), move || {
            if let Some(device_list) = device_list_weak.upgrade() {
                // Clear loading indicator
                while let Some(child) = device_list.first_child() {
                    device_list.remove(&child);
                }

                // Repopulate with actual devices
                Self::populate_device_list_static(&device_list, &devices_clone, &config_clone);
            }

            gtk4::glib::ControlFlow::Break
        });
    }

    fn populate_device_list_static(
        device_list: &ListBox,
        devices: &Arc<Mutex<HashMap<String, Device>>>,
        config: &Arc<Mutex<Config>>,
    ) {
        let devices_guard = devices.lock().unwrap();
        let config_guard = config.lock().unwrap();

        if devices_guard.is_empty() {
            let empty_row = Self::create_empty_state_row_static();
            device_list.append(&empty_row);
            return;
        }

        let mut device_list_vec: Vec<_> = devices_guard.values().collect();
        device_list_vec.sort_by(|a, b| match (a.connection_status, b.connection_status) {
            (ConnectionStatus::Connected, ConnectionStatus::Disconnected) => {
                std::cmp::Ordering::Less
            }
            (ConnectionStatus::Disconnected, ConnectionStatus::Connected) => {
                std::cmp::Ordering::Greater
            }
            _ => a.name.cmp(&b.name),
        });

        for device in device_list_vec {
            if device.connection_status == ConnectionStatus::Connected
                || config_guard.ui.show_disconnected_devices
            {
                let row = Self::create_device_detail_row_static(device);
                device_list.append(&row);
            }
        }

        debug!("Device list refreshed with {} devices", devices_guard.len());
    }

    fn create_empty_state_row_static() -> ListBoxRow {
        let row = ListBoxRow::new();
        row.set_activatable(false);
        row.set_selectable(false);

        let content_box = Box::new(Orientation::Vertical, 12);
        content_box.set_margin_start(24);
        content_box.set_margin_end(24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);
        content_box.set_halign(gtk4::Align::Center);

        let icon = Image::from_icon_name("battery-missing-symbolic");
        icon.set_icon_size(gtk4::IconSize::Large);
        icon.set_opacity(0.5);
        content_box.append(&icon);

        let title_label = Label::new(Some("No devices found"));
        title_label.set_css_classes(&["title-4"]);
        title_label.set_opacity(0.7);
        content_box.append(&title_label);

        let subtitle_label = Label::new(Some(
            "Make sure your Bluetooth and USB devices are connected",
        ));
        subtitle_label.set_css_classes(&["body", "dim-label"]);
        subtitle_label.set_justify(gtk4::Justification::Center);
        subtitle_label.set_wrap(true);
        content_box.append(&subtitle_label);

        row.set_child(Some(&content_box));
        row
    }

    fn create_device_detail_row_static(device: &Device) -> ListBoxRow {
        let row = ListBoxRow::new();
        row.set_activatable(false);
        row.set_selectable(false);

        let main_box = Box::new(Orientation::Vertical, 8);
        main_box.set_margin_start(16);
        main_box.set_margin_end(16);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);

        let header_box = Box::new(Orientation::Horizontal, 12);

        let icon_name = get_device_icon_name(&device.device_type);
        let icon = Image::from_icon_name(icon_name);
        icon.set_icon_size(gtk4::IconSize::Large);

        if device.connection_status == ConnectionStatus::Disconnected {
            icon.set_opacity(0.5);
        }

        header_box.append(&icon);

        let info_box = Box::new(Orientation::Vertical, 4);
        info_box.set_hexpand(true);

        let name_label = Label::new(Some(&device.name));
        name_label.set_halign(gtk4::Align::Start);
        name_label.set_css_classes(&["heading"]);

        if device.connection_status == ConnectionStatus::Disconnected {
            name_label.set_opacity(0.6);
        }

        info_box.append(&name_label);

        let type_text = format!(
            "{:?} • {}",
            device.device_type,
            get_connection_type_text(device)
        );
        let type_label = Label::new(Some(&type_text));
        type_label.set_halign(gtk4::Align::Start);
        type_label.set_css_classes(&["caption", "dim-label"]);
        info_box.append(&type_label);

        header_box.append(&info_box);

        let status_box = Box::new(Orientation::Vertical, 2);
        status_box.set_halign(gtk4::Align::End);

        let status_text = get_connection_status_text(device);
        let status_label = Label::new(Some(&status_text));
        status_label.set_halign(gtk4::Align::End);

        if device.connection_status == ConnectionStatus::Connected {
            status_label.set_css_classes(&["success"]);
        } else {
            status_label.set_css_classes(&["warning"]);
        }

        status_box.append(&status_label);

        if device.connection_status == ConnectionStatus::Connected {
            let battery_text = match device.battery_level {
                Some(level) => format!("{}%", level),
                None => "Battery info unavailable".to_string(),
            };

            let battery_label = Label::new(Some(&battery_text));
            battery_label.set_halign(gtk4::Align::End);
            battery_label.set_css_classes(&["title-3"]);

            if let Some(level) = device.battery_level {
                if level <= 20 {
                    battery_label.add_css_class("error");
                } else if level <= 50 {
                    battery_label.add_css_class("warning");
                } else {
                    battery_label.add_css_class("success");
                }
            }

            status_box.append(&battery_label);
        }

        header_box.append(&status_box);
        main_box.append(&header_box);

        let details_box = Box::new(Orientation::Horizontal, 16);
        details_box.set_margin_start(52);
        details_box.set_css_classes(&["dim-label"]);

        let device_id_label = Label::new(Some(&format!("ID: {}", device.id)));
        device_id_label.set_css_classes(&["caption", "monospace"]);
        device_id_label.set_selectable(true);
        details_box.append(&device_id_label);

        let last_seen_text = Self::format_last_seen_static(&device.last_seen);
        let last_seen_label = Label::new(Some(&format!("Last seen: {}", last_seen_text)));
        last_seen_label.set_css_classes(&["caption"]);
        details_box.append(&last_seen_label);

        main_box.append(&details_box);

        if device.connection_status == ConnectionStatus::Disconnected {
            main_box.set_opacity(0.7);
        }

        row.set_child(Some(&main_box));
        row
    }

    fn format_last_seen_static(last_seen: &SystemTime) -> String {
        match last_seen.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default();
                let seconds_ago = now.as_secs().saturating_sub(duration.as_secs());

                if seconds_ago < 60 {
                    "Just now".to_string()
                } else if seconds_ago < 3600 {
                    format!("{} minutes ago", seconds_ago / 60)
                } else if seconds_ago < 86400 {
                    format!("{} hours ago", seconds_ago / 3600)
                } else {
                    format!("{} days ago", seconds_ago / 86400)
                }
            }
            Err(_) => "Unknown".to_string(),
        }
    }
}
