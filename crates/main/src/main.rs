use battery_monitor_config::Config;
use clap::Parser;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook_tokio::Signals;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod cli;

use app::BatteryMonitorApp;
use cli::Args;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Handle special command line options
    if args.show_config {
        match Config::get_config_path() {
            Ok(path) => {
                println!("{}", path.display());
                return Ok(());
            }
            Err(e) => {
                eprintln!("Error getting config path: {}", e);
                return Err(e.into());
            }
        }
    }

    if args.check_config {
        match Config::load() {
            Ok(_) => {
                println!("Configuration is valid");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Configuration error: {}", e);
                return Err(e.into());
            }
        }
    }

    if args.print_default_config {
        let default_config = Config::default();
        let toml_str = toml::to_string_pretty(&default_config)?;
        println!("{}", toml_str);
        return Ok(());
    }

    if args.reset_config {
        let default_config = Config::default();
        match default_config.save() {
            Ok(_) => {
                println!("Configuration reset to defaults");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Error resetting config: {}", e);
                return Err(e.into());
            }
        }
    }

    setup_logging(&args)?;

    info!("Starting Battery Monitor v{}", env!("CARGO_PKG_VERSION"));

    let config = load_or_create_config().await?;
    info!("Configuration loaded from {:?}", Config::get_config_path()?);

    info!("Running in CLI/daemon mode (no GUI)");
    run_daemon(config).await
}

fn setup_logging(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let log_level = if args.verbose {
        tracing::Level::DEBUG
    } else if args.quiet {
        tracing::Level::WARN
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_level(true)
                .with_ansi(!args.no_color),
        )
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            log_level,
        ))
        .init();

    Ok(())
}

async fn load_or_create_config() -> Result<Config, Box<dyn std::error::Error>> {
    match Config::load() {
        Ok(config) => {
            debug!("Configuration loaded successfully");
            Ok(config)
        }
        Err(e) => {
            warn!("Failed to load config, using defaults: {}", e);
            let default_config = Config::default();
            default_config.save()?;
            info!("Created default configuration file");
            Ok(default_config)
        }
    }
}

async fn run_daemon(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = BatteryMonitorApp::new(config, false).await?;

    let signals = Signals::new(&[SIGTERM, SIGINT])?;
    let handle = signals.handle();

    let signals_task = tokio::spawn(async move {
        let mut signals = signals.fuse();
        while let Some(signal) = signals.next().await {
            match signal {
                SIGTERM | SIGINT => {
                    info!("Received shutdown signal");
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        result = app.run() => {
            if let Err(e) = result {
                error!("Application error: {}", e);
            }
        }
        _ = signals_task => {
            info!("Shutting down due to signal");
            app.shutdown().await;
        }
    }

    handle.close();
    info!("Battery Monitor daemon stopped");
    Ok(())
}
