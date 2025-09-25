use crate::GuiError;
use battery_monitor_config::{Config, ConfigError};
use gtk4::prelude::*;
use gtk4::{
    ApplicationWindow, Box, Button, Dialog, Grid, HeaderBar, Label, Orientation, SpinButton,
    Switch,
};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};

pub struct SettingsDialog {
    window: ApplicationWindow,
    config: Arc<Mutex<Config>>,

    polling_interval_spin: SpinButton,
    auto_start_switch: Switch,
    notifications_enabled_switch: Switch,
    low_battery_threshold_spin: SpinButton,
    show_connect_disconnect_switch: Switch,
    suppression_minutes_spin: SpinButton,
    show_disconnected_devices_switch: Switch,

    on_config_changed: Option<std::boxed::Box<dyn Fn(Config) + Send + Sync>>,
}

impl SettingsDialog {
    pub fn new(config: Arc<Mutex<Config>>) -> Result<Self, GuiError> {
        let window = ApplicationWindow::new(&gtk4::Application::default());
        window.set_title(Some("Battery Monitor - Settings"));
        window.set_default_size(450, 500);
        window.set_resizable(false);
        window.set_modal(true);

        let header_bar = HeaderBar::new();
        header_bar.set_title_widget(Some(&Label::new(Some("Settings"))));

        let cancel_button = Button::with_label("Cancel");
        header_bar.pack_start(&cancel_button);

        let save_button = Button::with_label("Save");
        save_button.set_css_classes(&["suggested-action"]);
        header_bar.pack_end(&save_button);

        window.set_titlebar(Some(&header_bar));

        let main_box = Box::new(Orientation::Vertical, 0);
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let content_box = Box::new(Orientation::Vertical, 24);
        content_box.set_margin_start(24);
        content_box.set_margin_end(24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);

        let monitoring_group = Self::create_monitoring_settings_group();
        content_box.append(&monitoring_group);

        let notifications_group = Self::create_notifications_settings_group();
        content_box.append(&notifications_group);

        let ui_group = Self::create_ui_settings_group();
        content_box.append(&ui_group);

        scrolled.set_child(Some(&content_box));
        main_box.append(&scrolled);
        window.set_child(Some(&main_box));

        let polling_interval_spin = SpinButton::with_range(5.0, 300.0, 1.0);
        let auto_start_switch = Switch::new();
        let notifications_enabled_switch = Switch::new();
        let low_battery_threshold_spin = SpinButton::with_range(1.0, 99.0, 1.0);
        let show_connect_disconnect_switch = Switch::new();
        let suppression_minutes_spin = SpinButton::with_range(1.0, 60.0, 1.0);
        let show_disconnected_devices_switch = Switch::new();

        let mut settings_dialog = Self {
            window,
            config,
            polling_interval_spin,
            auto_start_switch,
            notifications_enabled_switch,
            low_battery_threshold_spin,
            show_connect_disconnect_switch,
            suppression_minutes_spin,
            show_disconnected_devices_switch,
            on_config_changed: None,
        };

        settings_dialog.rebuild_ui()?;
        settings_dialog.setup_event_handlers(cancel_button, save_button)?;
        settings_dialog.load_current_config()?;

        Ok(settings_dialog)
    }

    fn rebuild_ui(&mut self) -> Result<(), GuiError> {
        let main_box = self.window.child().unwrap().downcast::<Box>().unwrap();
        let scrolled = main_box
            .first_child()
            .unwrap()
            .downcast::<gtk4::ScrolledWindow>()
            .unwrap();
        let content_box = scrolled.child().unwrap().downcast::<Box>().unwrap();

        while let Some(child) = content_box.first_child() {
            content_box.remove(&child);
        }

        let monitoring_group = self.create_monitoring_settings_group_with_widgets();
        content_box.append(&monitoring_group);

        let notifications_group = self.create_notifications_settings_group_with_widgets();
        content_box.append(&notifications_group);

        let ui_group = self.create_ui_settings_group_with_widgets();
        content_box.append(&ui_group);

        Ok(())
    }

    fn create_monitoring_settings_group() -> Box {
        let group_box = Box::new(Orientation::Vertical, 8);

        let title_label = Label::new(Some("Monitoring"));
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_css_classes(&["heading"]);
        group_box.append(&title_label);

        group_box
    }

    fn create_monitoring_settings_group_with_widgets(&self) -> Box {
        let group_box = Box::new(Orientation::Vertical, 8);

        let title_label = Label::new(Some("Monitoring"));
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_css_classes(&["heading"]);
        group_box.append(&title_label);

        let grid = Grid::new();
        grid.set_row_spacing(12);
        grid.set_column_spacing(12);

        let polling_label = Label::new(Some("Polling interval (seconds):"));
        polling_label.set_halign(gtk4::Align::Start);
        grid.attach(&polling_label, 0, 0, 1, 1);
        grid.attach(&self.polling_interval_spin, 1, 0, 1, 1);

        let auto_start_label = Label::new(Some("Start automatically:"));
        auto_start_label.set_halign(gtk4::Align::Start);
        grid.attach(&auto_start_label, 0, 1, 1, 1);
        grid.attach(&self.auto_start_switch, 1, 1, 1, 1);

        group_box.append(&grid);
        group_box
    }

    fn create_notifications_settings_group() -> Box {
        let group_box = Box::new(Orientation::Vertical, 8);

        let title_label = Label::new(Some("Notifications"));
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_css_classes(&["heading"]);
        group_box.append(&title_label);

        group_box
    }

    fn create_notifications_settings_group_with_widgets(&self) -> Box {
        let group_box = Box::new(Orientation::Vertical, 8);

        let title_label = Label::new(Some("Notifications"));
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_css_classes(&["heading"]);
        group_box.append(&title_label);

        let grid = Grid::new();
        grid.set_row_spacing(12);
        grid.set_column_spacing(12);

        let enabled_label = Label::new(Some("Enable notifications:"));
        enabled_label.set_halign(gtk4::Align::Start);
        grid.attach(&enabled_label, 0, 0, 1, 1);
        grid.attach(&self.notifications_enabled_switch, 1, 0, 1, 1);

        let threshold_label = Label::new(Some("Low battery threshold (%):"));
        threshold_label.set_halign(gtk4::Align::Start);
        grid.attach(&threshold_label, 0, 1, 1, 1);
        grid.attach(&self.low_battery_threshold_spin, 1, 1, 1, 1);

        let connect_disconnect_label = Label::new(Some("Show connect/disconnect:"));
        connect_disconnect_label.set_halign(gtk4::Align::Start);
        grid.attach(&connect_disconnect_label, 0, 2, 1, 1);
        grid.attach(&self.show_connect_disconnect_switch, 1, 2, 1, 1);

        let suppression_label = Label::new(Some("Suppression time (minutes):"));
        suppression_label.set_halign(gtk4::Align::Start);
        grid.attach(&suppression_label, 0, 3, 1, 1);
        grid.attach(&self.suppression_minutes_spin, 1, 3, 1, 1);

        group_box.append(&grid);
        group_box
    }

    fn create_ui_settings_group() -> Box {
        let group_box = Box::new(Orientation::Vertical, 8);

        let title_label = Label::new(Some("User Interface"));
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_css_classes(&["heading"]);
        group_box.append(&title_label);

        group_box
    }

    fn create_ui_settings_group_with_widgets(&self) -> Box {
        let group_box = Box::new(Orientation::Vertical, 8);

        let title_label = Label::new(Some("User Interface"));
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_css_classes(&["heading"]);
        group_box.append(&title_label);

        let grid = Grid::new();
        grid.set_row_spacing(12);
        grid.set_column_spacing(12);

        let show_disconnected_label = Label::new(Some("Show disconnected devices:"));
        show_disconnected_label.set_halign(gtk4::Align::Start);
        grid.attach(&show_disconnected_label, 0, 0, 1, 1);
        grid.attach(&self.show_disconnected_devices_switch, 1, 0, 1, 1);

        group_box.append(&grid);
        group_box
    }

    fn setup_event_handlers(
        &mut self,
        cancel_button: Button,
        save_button: Button,
    ) -> Result<(), GuiError> {
        let window = self.window.clone();
        cancel_button.connect_clicked(move |_| {
            window.close();
        });

        let config = Arc::clone(&self.config);
        let polling_interval_spin = self.polling_interval_spin.clone();
        let auto_start_switch = self.auto_start_switch.clone();
        let notifications_enabled_switch = self.notifications_enabled_switch.clone();
        let low_battery_threshold_spin = self.low_battery_threshold_spin.clone();
        let show_connect_disconnect_switch = self.show_connect_disconnect_switch.clone();
        let suppression_minutes_spin = self.suppression_minutes_spin.clone();
        let show_disconnected_devices_switch = self.show_disconnected_devices_switch.clone();
        let window = self.window.clone();

        let window_clone = window.clone();
        save_button.connect_clicked(move |_| {
            if let Err(e) = Self::save_config(
                Arc::clone(&config),
                &polling_interval_spin,
                &auto_start_switch,
                &notifications_enabled_switch,
                &low_battery_threshold_spin,
                &show_connect_disconnect_switch,
                &suppression_minutes_spin,
                &show_disconnected_devices_switch,
            ) {
                error!("Failed to save config: {}", e);
                Self::show_error_dialog(
                    &window_clone,
                    "Settings Save Failed",
                    &format!("Failed to save settings: {}", e),
                );
            } else {
                info!("Settings saved successfully");
                window.close();
            }
        });

        Ok(())
    }

    fn save_config(
        config: Arc<Mutex<Config>>,
        polling_interval_spin: &SpinButton,
        auto_start_switch: &Switch,
        notifications_enabled_switch: &Switch,
        low_battery_threshold_spin: &SpinButton,
        show_connect_disconnect_switch: &Switch,
        suppression_minutes_spin: &SpinButton,
        show_disconnected_devices_switch: &Switch,
    ) -> Result<(), ConfigError> {
        let mut config_guard = config.lock().unwrap();

        config_guard.monitoring.polling_interval_seconds = polling_interval_spin.value() as u64;
        config_guard.monitoring.auto_start = auto_start_switch.is_active();
        config_guard.notifications.enabled = notifications_enabled_switch.is_active();
        config_guard.notifications.low_battery_threshold = low_battery_threshold_spin.value() as u8;
        config_guard.notifications.show_connect_disconnect =
            show_connect_disconnect_switch.is_active();
        config_guard.notifications.suppression_minutes = suppression_minutes_spin.value() as u64;
        config_guard.ui.show_disconnected_devices = show_disconnected_devices_switch.is_active();

        config_guard.save()?;
        Ok(())
    }

    fn load_current_config(&mut self) -> Result<(), GuiError> {
        let config = self.config.lock().unwrap();

        self.polling_interval_spin
            .set_value(config.monitoring.polling_interval_seconds as f64);
        self.auto_start_switch
            .set_active(config.monitoring.auto_start);
        self.notifications_enabled_switch
            .set_active(config.notifications.enabled);
        self.low_battery_threshold_spin
            .set_value(config.notifications.low_battery_threshold as f64);
        self.show_connect_disconnect_switch
            .set_active(config.notifications.show_connect_disconnect);
        self.suppression_minutes_spin
            .set_value(config.notifications.suppression_minutes as f64);
        self.show_disconnected_devices_switch
            .set_active(config.ui.show_disconnected_devices);

        debug!("Loaded current configuration into settings dialog");
        Ok(())
    }

    pub fn show(&self) -> Result<(), GuiError> {
        self.window.present();
        debug!("Settings dialog shown");
        Ok(())
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
        debug!("Settings dialog hidden");
    }

    pub fn update_config(&mut self) {
        if let Err(e) = self.load_current_config() {
            error!("Failed to update settings dialog config: {}", e);
        }
    }

    pub fn set_config_changed_callback<F>(&mut self, callback: F)
    where
        F: Fn(Config) + Send + Sync + 'static,
    {
        self.on_config_changed = Some(std::boxed::Box::new(callback));
    }

    fn show_error_dialog(parent_window: &ApplicationWindow, title: &str, message: &str) {
        let error_dialog = Dialog::builder()
            .title(title)
            .modal(true)
            .default_width(400)
            .default_height(200)
            .transient_for(parent_window)
            .build();

        let content_box = Box::new(Orientation::Vertical, 16);
        content_box.set_margin_start(24);
        content_box.set_margin_end(24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);

        // Error icon and message
        let error_box = Box::new(Orientation::Horizontal, 12);

        let error_icon = gtk4::Image::from_icon_name("dialog-error-symbolic");
        error_icon.set_icon_size(gtk4::IconSize::Large);
        error_icon.set_valign(gtk4::Align::Start);
        error_box.append(&error_icon);

        let message_label = Label::new(Some(message));
        message_label.set_wrap(true);
        message_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        message_label.set_halign(gtk4::Align::Start);
        message_label.set_valign(gtk4::Align::Start);
        message_label.set_hexpand(true);
        error_box.append(&message_label);

        content_box.append(&error_box);

        // Button box
        let button_box = Box::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);
        button_box.set_margin_top(8);

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

        error!("Error dialog shown: {}", message);
    }
}
