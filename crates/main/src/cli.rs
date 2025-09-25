use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "battery-monitor",
    version = env!("CARGO_PKG_VERSION"),
    about = "Linux battery monitor for Bluetooth devices and USB keyboards",
    long_about = "Battery Monitor tracks battery levels of Bluetooth devices and USB/wireless keyboards, displaying them in the system tray with configurable notifications."
)]
pub struct Args {
    /// Run in daemon mode (no GUI)
    #[arg(short, long)]
    pub daemon: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppress output (only show warnings and errors)
    #[arg(short, long)]
    pub quiet: bool,

    /// Disable colored output
    #[arg(long)]
    pub no_color: bool,

    /// Show configuration file path and exit
    #[arg(long)]
    pub show_config: bool,

    /// Validate configuration file and exit
    #[arg(long)]
    pub check_config: bool,

    /// Print default configuration and exit
    #[arg(long)]
    pub print_default_config: bool,

    /// Reset configuration to defaults
    #[arg(long)]
    pub reset_config: bool,
}
