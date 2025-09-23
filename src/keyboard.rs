use std::collections::HashMap;
use hidapi::{HidApi, HidDevice, DeviceInfo};

#[derive(Clone, Debug)]
pub struct Keyboard {
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub battery_percentage: Option<u8>,
    pub keyboard_type: KeyboardType,
    pub path: String,
    pub serial_number: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeyboardType {
    AjazzAK870,
    Mechanical,
    Membrane,
    Unknown,
}

impl Keyboard {
    pub fn get_icon(&self) -> &'static str {
        match self.keyboard_type {
            KeyboardType::AjazzAK870 => "⌨️",
            KeyboardType::Mechanical => "🔧",
            KeyboardType::Membrane => "⌨️",
            KeyboardType::Unknown => "⌨️",
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

    pub fn device_id(&self) -> String {
        format!("{:04x}:{:04x}", self.vendor_id, self.product_id)
    }
}

pub struct KeyboardManager {
    pub connected_keyboards: HashMap<String, Keyboard>,
    hid_api: HidApi,
}

impl KeyboardManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let hid_api = HidApi::new()?;
        Ok(Self {
            connected_keyboards: HashMap::new(),
            hid_api,
        })
    }

    pub fn scan_for_keyboards(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connected_keyboards.clear();

        // Refresh the device list
        self.hid_api.refresh_devices()?;

        // Enumerate all HID devices
        for device_info in self.hid_api.device_list() {
            if let Some(keyboard) = self.analyze_hid_device(device_info)? {
                let device_key = format!("{}:{}", keyboard.path, keyboard.device_id());
                println!("Found keyboard: {} ({})", keyboard.name, keyboard.device_id());
                println!("  Type: {:?}", keyboard.keyboard_type);
                if let Some(battery) = keyboard.battery_percentage {
                    println!("  Battery: {}%", battery);
                }
                self.connected_keyboards.insert(device_key, keyboard);
            }
        }

        Ok(())
    }

    fn analyze_hid_device(&self, device_info: &DeviceInfo) -> Result<Option<Keyboard>, Box<dyn std::error::Error>> {
        // Check if this might be a keyboard
        let is_keyboard = self.is_likely_keyboard(device_info);

        if !is_keyboard {
            return Ok(None);
        }

        let name = device_info.product_string()
            .unwrap_or("Unknown Keyboard")
            .to_string();

        let vendor_id = device_info.vendor_id();
        let product_id = device_info.product_id();
        let path = device_info.path().to_string_lossy().to_string();
        let serial_number = device_info.serial_number().map(|s| s.to_string());

        let keyboard_type = self.detect_keyboard_type(&name, vendor_id, product_id);

        // Try to get battery percentage
        let battery_percentage = self.get_hid_battery(device_info, &keyboard_type)?;

        Ok(Some(Keyboard {
            name,
            vendor_id,
            product_id,
            battery_percentage,
            keyboard_type,
            path,
            serial_number,
        }))
    }

    fn is_likely_keyboard(&self, device_info: &DeviceInfo) -> bool {
        // Check usage page and usage for keyboard indicators
        let usage_page = device_info.usage_page();
        let usage = device_info.usage();

        // HID Usage Page 1 (Generic Desktop) with Usage 6 (Keyboard)
        if usage_page == 1 && usage == 6 {
            return true;
        }

        // Check product string for keyboard indicators
        if let Some(product) = device_info.product_string() {
            let product_lower = product.to_lowercase();
            if product_lower.contains("keyboard") ||
               product_lower.contains("ak870") ||
               product_lower.contains("ajazz") {
                return true;
            }
        }

        // Check specific vendor/product ID combinations
        let vendor_id = device_info.vendor_id();
        let product_id = device_info.product_id();

        // Your specific AK870
        if vendor_id == 0x05ac && product_id == 0x024f {
            return true;
        }

        // Other known keyboard vendor IDs
        match vendor_id {
            0x0483 | 0x1ea7 => true, // Known Ajazz vendor IDs
            _ => false,
        }
    }

    fn detect_keyboard_type(&self, name: &str, vendor_id: u16, product_id: u16) -> KeyboardType {
        let name_lower = name.to_lowercase();

        // Check for Ajazz AK870 specifically by name
        if name_lower.contains("ak870") || name_lower.contains("ajazz") {
            return KeyboardType::AjazzAK870;
        }

        // Check vendor/product ID for known keyboards
        match (vendor_id, product_id) {
            // Specific Ajazz AK870 device ID: 05ac:024f
            (0x05ac, 0x024f) => KeyboardType::AjazzAK870,
            // Other known Ajazz vendor IDs
            (0x0483, _) => KeyboardType::AjazzAK870,
            (0x1ea7, _) => KeyboardType::AjazzAK870,
            // Apple vendor ID but could be used by other manufacturers for AK870
            (0x05ac, _) => {
                if name_lower.contains("keyboard") || name_lower.contains("ak") {
                    KeyboardType::AjazzAK870
                } else {
                    KeyboardType::Unknown
                }
            },
            _ => {
                if name_lower.contains("mechanical") {
                    KeyboardType::Mechanical
                } else if name_lower.contains("membrane") {
                    KeyboardType::Membrane
                } else {
                    KeyboardType::Unknown
                }
            }
        }
    }

    fn get_hid_battery(&self, device_info: &DeviceInfo, keyboard_type: &KeyboardType) -> Result<Option<u8>, Box<dyn std::error::Error>> {
        match keyboard_type {
            KeyboardType::AjazzAK870 => self.get_ajazz_ak870_hid_battery(device_info),
            _ => Ok(None),
        }
    }

    fn get_ajazz_ak870_hid_battery(&self, device_info: &DeviceInfo) -> Result<Option<u8>, Box<dyn std::error::Error>> {
        // Check if this is a wireless receiver
        let is_wireless_receiver = device_info.product_string()
            .map(|p| p.to_lowercase().contains("wireless") || p.to_lowercase().contains("receiver"))
            .unwrap_or(false);

        // Try to open the HID device
        match self.hid_api.open_path(device_info.path()) {
            Ok(device) => {
                if is_wireless_receiver {
                    println!("Detected wireless receiver, using specialized detection...");
                    // For wireless receivers, use different approach
                    if let Some(battery) = self.try_wireless_battery_detection(&device)? {
                        return Ok(Some(battery));
                    }
                } else {
                    // For direct USB keyboards, try standard methods
                    // Method 1: Standard HID battery report (Report ID 0x01)
                    if let Some(battery) = self.try_standard_battery_report(&device)? {
                        return Ok(Some(battery));
                    }

                    // Method 2: Custom Ajazz battery report (Report ID 0x02)
                    if let Some(battery) = self.try_ajazz_battery_report(&device)? {
                        return Ok(Some(battery));
                    }

                    // Method 3: Feature report for battery (Report ID 0x03)
                    if let Some(battery) = self.try_feature_battery_report(&device)? {
                        return Ok(Some(battery));
                    }
                }

                // Method 4: Try reading input reports that might contain battery info (works for both)
                if let Some(battery) = self.try_input_battery_report(&device)? {
                    return Ok(Some(battery));
                }

                Ok(None)
            }
            Err(e) => {
                // If we can't open the device, try alternative methods
                println!("Failed to open HID device: {}", e);

                // Fall back to system battery interfaces
                self.get_system_battery_for_device(device_info.vendor_id(), device_info.product_id())
            }
        }
    }

    fn try_standard_battery_report(&self, device: &HidDevice) -> Result<Option<u8>, Box<dyn std::error::Error>> {
        let mut buf = [0u8; 65];
        buf[0] = 0x01; // Report ID for battery

        match device.get_feature_report(&mut buf) {
            Ok(size) if size > 1 => {
                // Battery level is often in the second byte as a percentage
                let battery_level = buf[1];
                if battery_level <= 100 {
                    return Ok(Some(battery_level));
                }
            }
            _ => {}
        }

        Ok(None)
    }

    fn try_ajazz_battery_report(&self, device: &HidDevice) -> Result<Option<u8>, Box<dyn std::error::Error>> {
        let mut buf = [0u8; 65];
        buf[0] = 0x02; // Ajazz-specific report ID

        match device.get_feature_report(&mut buf) {
            Ok(size) if size > 2 => {
                // Check different possible battery positions
                for &byte in &buf[1..size.min(10)] {
                    if byte <= 100 && byte > 0 {
                        return Ok(Some(byte));
                    }
                }
            }
            _ => {}
        }

        Ok(None)
    }

    fn try_feature_battery_report(&self, device: &HidDevice) -> Result<Option<u8>, Box<dyn std::error::Error>> {
        // Try different report IDs that might contain battery info
        for report_id in [0x03, 0x04, 0x05, 0x10, 0x20] {
            let mut buf = [0u8; 65];
            buf[0] = report_id;

            if let Ok(size) = device.get_feature_report(&mut buf) {
                if size > 1 {
                    // Look for battery percentage in various positions
                    for i in 1..size.min(8) {
                        let value = buf[i];
                        if value <= 100 && value > 0 {
                            // Additional validation: check if this looks like a battery percentage
                            if self.validate_battery_value(value, &buf[1..size]) {
                                return Ok(Some(value));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn try_wireless_battery_detection(&self, device: &HidDevice) -> Result<Option<u8>, Box<dyn std::error::Error>> {
        println!("Trying wireless receiver battery detection methods...");

        // Method 1: Try to send battery query command to wireless receiver
        if let Some(battery) = self.try_wireless_battery_query(&device)? {
            return Ok(Some(battery));
        }

        // Method 2: Monitor input reports for battery notifications
        if let Some(battery) = self.try_wireless_input_monitoring(&device)? {
            return Ok(Some(battery));
        }

        // Method 3: Try specific wireless receiver feature reports (avoid broken pipe)
        if let Some(battery) = self.try_safe_feature_reports(&device)? {
            return Ok(Some(battery));
        }

        Ok(None)
    }

    fn try_wireless_battery_query(&self, device: &HidDevice) -> Result<Option<u8>, Box<dyn std::error::Error>> {
        // Send battery query command to receiver
        // Common commands for wireless keyboards
        let battery_query_commands = [
            [0x10, 0xFF, 0x8F, 0x20, 0x00, 0x00, 0x00], // Generic battery query
            [0x10, 0xFF, 0x83, 0xB5, 0x40, 0x00, 0x00], // Logitech-style query
            [0x11, 0xFF, 0x8F, 0x20, 0x00, 0x00, 0x00], // Alternative report ID
        ];

        for command in &battery_query_commands {
            if let Ok(_) = device.write(command) {
                std::thread::sleep(std::time::Duration::from_millis(10));

                // Try to read response
                let mut buf = [0u8; 65];
                device.set_blocking_mode(false)?;

                for _ in 0..3 {
                    match device.read(&mut buf) {
                        Ok(size) if size > 4 => {
                            // Look for battery response pattern
                            if buf[0] == 0x10 && buf[2] == 0x8F {
                                if buf[4] <= 100 && buf[4] > 0 {
                                    println!("Found battery level via wireless query: {}%", buf[4]);
                                    return Ok(Some(buf[4]));
                                }
                            }
                        }
                        _ => {}
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        }

        Ok(None)
    }

    fn try_wireless_input_monitoring(&self, device: &HidDevice) -> Result<Option<u8>, Box<dyn std::error::Error>> {
        device.set_blocking_mode(false)?;

        // Monitor input reports for longer period to catch battery notifications
        for attempt in 0..20 {
            let mut buf = [0u8; 65];
            match device.read(&mut buf) {
                Ok(size) if size > 0 => {
                    println!("Input report {}: {:02x?}", attempt, &buf[0..size.min(8)]);

                    // Look for battery information patterns in wireless reports
                    // Many wireless keyboards send battery info in specific patterns
                    if size >= 4 {
                        // Pattern 1: Battery in byte 3 or 4
                        for pos in [3, 4, 5] {
                            if pos < size {
                                let value = buf[pos];
                                if value <= 100 && value > 0 && value % 5 == 0 {
                                    // Wireless keyboards often report in 5% increments
                                    println!("Found potential battery value at pos {}: {}%", pos, value);
                                    return Ok(Some(value));
                                }
                            }
                        }

                        // Pattern 2: Check for battery notification header
                        if buf[0] == 0x10 || buf[0] == 0x11 {
                            for i in 1..size.min(8) {
                                let value = buf[i];
                                if value <= 100 && value >= 5 && value % 5 == 0 {
                                    println!("Found battery in notification: {}%", value);
                                    return Ok(Some(value));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        Ok(None)
    }

    fn try_safe_feature_reports(&self, device: &HidDevice) -> Result<Option<u8>, Box<dyn std::error::Error>> {
        // Try feature reports that are less likely to cause broken pipe
        // These are read-only and safer for wireless receivers
        let safe_report_ids = [0x00, 0x06, 0x07, 0x08]; // Avoid 0x01-0x05 which caused broken pipe

        for &report_id in &safe_report_ids {
            let mut buf = [0u8; 65];
            buf[0] = report_id;

            match device.get_feature_report(&mut buf) {
                Ok(size) if size > 1 => {
                    println!("Safe feature report ID 0x{:02x}: {} bytes", report_id, size);

                    for i in 1..size.min(16) {
                        let value = buf[i];
                        if value <= 100 && value > 0 {
                            if self.validate_battery_value(value, &buf[1..size]) {
                                println!("Found battery in safe feature report: {}%", value);
                                return Ok(Some(value));
                            }
                        }
                    }
                }
                Ok(_) => {
                    // Size <= 1, no useful data
                }
                Err(_) => {
                    // Ignore errors for safe feature reports
                }
            }
        }

        Ok(None)
    }

    fn try_input_battery_report(&self, device: &HidDevice) -> Result<Option<u8>, Box<dyn std::error::Error>> {
        // Set non-blocking mode
        device.set_blocking_mode(false)?;

        // Try to read a few input reports
        for _ in 0..5 {
            let mut buf = [0u8; 65];
            match device.read(&mut buf) {
                Ok(size) if size > 0 => {
                    // Look for battery info in input reports
                    for i in 0..size.min(8) {
                        let value = buf[i];
                        if value <= 100 && value > 0 {
                            if self.validate_battery_value(value, &buf[0..size]) {
                                return Ok(Some(value));
                            }
                        }
                    }
                }
                _ => break,
            }
        }

        Ok(None)
    }

    fn validate_battery_value(&self, value: u8, buffer: &[u8]) -> bool {
        // Simple validation to check if this looks like a real battery value
        if value == 0 || value > 100 {
            return false;
        }

        // Check if the value appears in a reasonable context
        // (e.g., not all bytes are the same, which might indicate an error)
        let unique_bytes = buffer.iter().collect::<std::collections::HashSet<_>>().len();
        unique_bytes > 1 && value >= 10 // Assume battery is at least 10% if reporting
    }

    fn get_system_battery_for_device(&self, vendor_id: u16, product_id: u16) -> Result<Option<u8>, Box<dyn std::error::Error>> {
        // Fall back to system battery interfaces when HID access fails
        use std::fs;

        let power_supply_path = "/sys/class/power_supply";

        if let Ok(entries) = fs::read_dir(power_supply_path) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Check if this is a battery device
                let type_file = path.join("type");
                if let Ok(device_type) = fs::read_to_string(&type_file) {
                    if device_type.trim() == "Battery" {
                        // Check if this might be our keyboard
                        if self.is_keyboard_power_supply(&path, vendor_id, product_id)? {
                            let capacity_file = path.join("capacity");
                            if let Ok(capacity_str) = fs::read_to_string(&capacity_file) {
                                if let Ok(capacity) = capacity_str.trim().parse::<u8>() {
                                    return Ok(Some(capacity));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn is_keyboard_power_supply(&self, power_supply_path: &std::path::Path, _vendor_id: u16, _product_id: u16) -> Result<bool, Box<dyn std::error::Error>> {
        use std::fs;

        // Check various identification methods
        let device_name = power_supply_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check if the power supply name suggests it's a HID/keyboard device
        if device_name.contains("hid") || device_name.contains("keyboard") {
            return Ok(true);
        }

        // Check model and manufacturer files
        let model_file = power_supply_path.join("model_name");
        let manufacturer_file = power_supply_path.join("manufacturer");

        let model = fs::read_to_string(&model_file).unwrap_or_default().to_lowercase();
        let manufacturer = fs::read_to_string(&manufacturer_file).unwrap_or_default().to_lowercase();

        // Check for Ajazz or AK870 indicators
        if model.contains("ak870") || model.contains("ajazz") ||
           manufacturer.contains("ajazz") {
            return Ok(true);
        }

        // Check if vendor/product IDs match (if available in sysfs)
        // This is more complex and may require traversing device hierarchy
        // For now, use heuristics based on device naming

        Ok(false)
    }

    pub fn get_status_text(&self) -> String {
        if self.connected_keyboards.is_empty() {
            return "No keyboards".to_string();
        }

        let mut status_parts = Vec::new();
        for keyboard in self.connected_keyboards.values() {
            status_parts.push(keyboard.format_for_status());
        }

        status_parts.join(" | ")
    }

    pub fn update_battery_levels(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Refresh device list to get current state
        self.hid_api.refresh_devices()?;

        // Update battery levels for known keyboards
        let keyboard_paths: Vec<_> = self.connected_keyboards.keys().cloned().collect();

        for keyboard_key in keyboard_paths {
            if let Some(keyboard) = self.connected_keyboards.get(&keyboard_key) {
                if keyboard.keyboard_type == KeyboardType::AjazzAK870 {
                    // Find the device in the current device list
                    if let Some(device_info) = self.hid_api.device_list()
                        .find(|d| d.vendor_id() == keyboard.vendor_id &&
                                  d.product_id() == keyboard.product_id &&
                                  d.path().to_string_lossy() == keyboard.path) {

                        if let Ok(Some(new_battery)) = self.get_hid_battery(device_info, &keyboard.keyboard_type) {
                            if let Some(kb) = self.connected_keyboards.get_mut(&keyboard_key) {
                                if kb.battery_percentage != Some(new_battery) {
                                    println!("Keyboard battery updated for {}: {}%", kb.name, new_battery);
                                    kb.battery_percentage = Some(new_battery);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
