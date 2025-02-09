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
    #[cfg(target_os = "macos")]
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
    }

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

    // Initialize the sound system
    let sound_engine = audio::SoundEngine::new()?;
    let sound_engine = Arc::new(sound_engine);

    // Start keyboard listener
    let keyboard_handler = input::KeyboardHandler::new(sound_engine.clone())?;
    keyboard_handler.start()?;

    // Create and run the tray icon - this will start the event loop
    let tray = ui::TrayIcon::new()?;
    tray.run()?;  // This will block and keep the application running

    Ok(())
}