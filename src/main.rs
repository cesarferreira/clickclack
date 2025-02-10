#[cfg(target_os = "macos")]
use cocoa::base::nil;
#[cfg(target_os = "macos")]
use cocoa::appkit::{NSApplication, NSApplicationActivationPolicyAccessory};
#[cfg(target_os = "macos")]
use cocoa::foundation::NSAutoreleasePool;

mod audio;
mod input;
mod ui;
mod config;
mod service;

use anyhow::Result;
use log::{info, error};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;
use std::io::Write;
use clap::{Parser, Subcommand};

static APP_STATE: Lazy<Arc<Mutex<config::Config>>> = Lazy::new(|| {
    Arc::new(Mutex::new(config::Config::load().unwrap_or_default()))
});

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the ClickClack service
    Start,
    /// Stop the ClickClack service
    Stop,
    /// Restart the ClickClack service
    Restart,
}

fn main() -> Result<()> {
    // Initialize logging with debug level
    env_logger::Builder::from_env(env_logger::Env::default()
        .filter_or("RUST_LOG", "debug"))
        .format(|buf, record| {
            writeln!(buf, "[{}] {}", record.level(), record.args())
        })
        .init();
    info!("Starting ClickClack...");

    // Parse command line arguments
    let cli = Cli::parse();

    // Handle service commands if present
    if let Some(command) = cli.command {
        let service_manager = service::ServiceManager::new()?;
        match command {
            Commands::Start => {
                service_manager.start_service()?;
                println!("ClickClack service started successfully");
                return Ok(());
            }
            Commands::Stop => {
                service_manager.stop_service()?;
                println!("ClickClack service stopped successfully");
                return Ok(());
            }
            Commands::Restart => {
                service_manager.restart_service()?;
                println!("ClickClack service restarted successfully");
                return Ok(());
            }
        }
    }

    // Regular application startup
    #[cfg(target_os = "macos")]
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
    }

    // Initialize assets
    if let Err(e) = ui::tray::ensure_assets_exist() {
        error!("Failed to initialize assets: {}", e);
        return Err(anyhow::anyhow!("Failed to initialize assets: {}", e));
    }
    info!("Assets initialized successfully");

    // Initialize the sound system
    let sound_engine = Arc::new(audio::SoundEngine::new()?);
    info!("Sound engine initialized");

    // Start keyboard listener in a separate thread
    let keyboard_handler = input::KeyboardHandler::new(sound_engine.clone())?;
    keyboard_handler.start()?;
    info!("Keyboard handler started");

    // Create the tray icon
    let _tray = ui::TrayIcon::new()?;
    info!("Tray icon created");
    
    // Run the main event loop
    #[cfg(target_os = "macos")]
    unsafe {
        let app = NSApplication::sharedApplication(nil);
        app.run();
    }

    #[cfg(not(target_os = "macos"))]
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(())
}