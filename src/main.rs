#[cfg(target_os = "macos")]
use cocoa::base::nil;
#[cfg(target_os = "macos")]
use cocoa::appkit::{NSApplication, NSApplicationActivationPolicyAccessory};
#[cfg(target_os = "macos")]
use cocoa::foundation::NSAutoreleasePool;

mod audio;
mod input;
mod ui;

use anyhow::Result;
use log::info;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;
use std::io::Write;

static APP_STATE: Lazy<Arc<Mutex<AppState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(AppState {
        enabled: true,
        volume: 1.0,
        keyboard_profile: String::from("Kandas-Woods-v1"),
        frequency: 500.0,
        decay: 30.0,
    }))
});

pub struct AppState {
    enabled: bool,
    volume: f32,
    keyboard_profile: String,
    frequency: f32,
    decay: f32,
}

fn main() -> Result<()> {
    // Initialize logging with info level by default
    env_logger::Builder::from_env(env_logger::Env::default()
        .filter_or("RUST_LOG", "info")
        .filter_or("symphonia", "error"))
        .format(|buf, record| {
            // Only show our application logs
            if !record.target().starts_with("symphonia") {
                writeln!(buf, "[{}] {}", record.level(), record.args())
            } else {
                Ok(())
            }
        })
        .init();
    info!("Starting ClickClack...");

    #[cfg(target_os = "macos")]
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
    }

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