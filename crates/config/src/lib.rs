use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub monitoring: MonitoringConfig,
    pub notifications: NotificationConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonitoringConfig {
    pub polling_interval_seconds: u64,
    pub auto_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub low_battery_threshold: u8,
    pub show_connect_disconnect: bool,
    pub suppression_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiConfig {
    pub show_disconnected_devices: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            monitoring: MonitoringConfig::default(),
            notifications: NotificationConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            polling_interval_seconds: 30,
            auto_start: false,
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            low_battery_threshold: 20,
            show_connect_disconnect: true,
            suppression_minutes: 5,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_disconnected_devices: true,
        }
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("TOML serialization error: {0}")]
    TomlSerError(#[from] toml::ser::Error),
    #[error("TOML deserialization error: {0}")]
    TomlDeError(#[from] toml::de::Error),
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
    #[error("Config directory creation failed")]
    DirectoryCreationFailed,
}

pub trait ConfigManager {
    fn load() -> Result<Config, ConfigError>;
    fn save(&self, config: &Config) -> Result<(), ConfigError>;
    fn get_config_path() -> Result<PathBuf, ConfigError>;
    fn get_config_dir() -> Result<PathBuf, ConfigError>;
}

pub struct FileConfigManager;

impl ConfigManager for FileConfigManager {
    fn load() -> Result<Config, ConfigError> {
        let config_path = Self::get_config_path()?;

        if !config_path.exists() {
            info!("Config file not found, creating default config at {:?}", config_path);
            let default_config = Config::default();
            let manager = FileConfigManager;
            manager.save(&default_config)?;
            return Ok(default_config);
        }

        let config_content = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&config_content)?;

        Self::validate_config(&config)?;

        debug!("Loaded config from {:?}", config_path);
        Ok(config)
    }

    fn save(&self, config: &Config) -> Result<(), ConfigError> {
        Self::validate_config(config)?;

        let config_path = Self::get_config_path()?;
        let config_dir = config_path.parent().unwrap();

        if !config_dir.exists() {
            fs::create_dir_all(config_dir)?;
        }

        let config_content = toml::to_string_pretty(config)?;
        fs::write(&config_path, config_content)?;

        info!("Saved config to {:?}", config_path);
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf, ConfigError> {
        let config_dir = Self::get_config_dir()?;
        Ok(config_dir.join("config.toml"))
    }

    fn get_config_dir() -> Result<PathBuf, ConfigError> {
        dirs::config_dir()
            .map(|dir| dir.join("battery-monitor"))
            .ok_or(ConfigError::DirectoryCreationFailed)
    }
}

impl FileConfigManager {
    fn validate_config(config: &Config) -> Result<(), ConfigError> {
        if config.monitoring.polling_interval_seconds < 5 || config.monitoring.polling_interval_seconds > 300 {
            return Err(ConfigError::ValidationError(
                "Polling interval must be between 5 and 300 seconds".to_string()
            ));
        }

        if config.notifications.low_battery_threshold < 1 || config.notifications.low_battery_threshold > 99 {
            return Err(ConfigError::ValidationError(
                "Low battery threshold must be between 1 and 99 percent".to_string()
            ));
        }

        if config.notifications.suppression_minutes < 1 || config.notifications.suppression_minutes > 60 {
            return Err(ConfigError::ValidationError(
                "Notification suppression must be between 1 and 60 minutes".to_string()
            ));
        }

        Ok(())
    }
}

impl Config {
    pub fn polling_interval(&self) -> Duration {
        Duration::from_secs(self.monitoring.polling_interval_seconds)
    }

    pub fn suppression_duration(&self) -> Duration {
        Duration::from_secs(self.notifications.suppression_minutes * 60)
    }

    pub fn load() -> Result<Self, ConfigError> {
        FileConfigManager::load()
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let manager = FileConfigManager;
        manager.save(self)
    }

    pub fn get_config_path() -> Result<PathBuf, ConfigError> {
        FileConfigManager::get_config_path()
    }

    pub fn set_polling_interval(&mut self, seconds: u64) -> Result<(), ConfigError> {
        if seconds < 5 || seconds > 300 {
            return Err(ConfigError::ValidationError(
                "Polling interval must be between 5 and 300 seconds".to_string()
            ));
        }
        self.monitoring.polling_interval_seconds = seconds;
        Ok(())
    }

    pub fn set_low_battery_threshold(&mut self, threshold: u8) -> Result<(), ConfigError> {
        if threshold < 1 || threshold > 99 {
            return Err(ConfigError::ValidationError(
                "Low battery threshold must be between 1 and 99 percent".to_string()
            ));
        }
        self.notifications.low_battery_threshold = threshold;
        Ok(())
    }

    pub fn set_suppression_minutes(&mut self, minutes: u64) -> Result<(), ConfigError> {
        if minutes < 1 || minutes > 60 {
            return Err(ConfigError::ValidationError(
                "Notification suppression must be between 1 and 60 minutes".to_string()
            ));
        }
        self.notifications.suppression_minutes = minutes;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.monitoring.polling_interval_seconds, 30);
        assert_eq!(config.notifications.low_battery_threshold, 20);
        assert!(config.notifications.enabled);
        assert!(config.ui.show_disconnected_devices);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();

        config.monitoring.polling_interval_seconds = 2;
        assert!(FileConfigManager::validate_config(&config).is_err());

        config.monitoring.polling_interval_seconds = 400;
        assert!(FileConfigManager::validate_config(&config).is_err());

        config = Config::default();
        config.notifications.low_battery_threshold = 0;
        assert!(FileConfigManager::validate_config(&config).is_err());

        config.notifications.low_battery_threshold = 100;
        assert!(FileConfigManager::validate_config(&config).is_err());

        config = Config::default();
        assert!(FileConfigManager::validate_config(&config).is_ok());
    }

    #[test]
    fn test_duration_conversion() {
        let config = Config::default();
        assert_eq!(config.polling_interval(), Duration::from_secs(30));
        assert_eq!(config.suppression_duration(), Duration::from_secs(300));
    }

    #[test]
    fn test_toml_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_config_setters() {
        let mut config = Config::default();

        assert!(config.set_polling_interval(60).is_ok());
        assert_eq!(config.monitoring.polling_interval_seconds, 60);

        assert!(config.set_low_battery_threshold(15).is_ok());
        assert_eq!(config.notifications.low_battery_threshold, 15);

        assert!(config.set_suppression_minutes(10).is_ok());
        assert_eq!(config.notifications.suppression_minutes, 10);

        assert!(config.set_polling_interval(2).is_err());
        assert!(config.set_low_battery_threshold(0).is_err());
        assert!(config.set_suppression_minutes(70).is_err());
    }
}
