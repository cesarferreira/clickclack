#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};
#[cfg(target_os = "macos")]
use cocoa::base::{id, nil, NO, YES};
#[cfg(target_os = "macos")]
use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy, NSApplicationActivationPolicyAccessory};
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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    info!("Starting ClickClack...");

    // Initialize the sound system
    let sound_engine = audio::SoundEngine::new()?;
    let sound_engine = Arc::new(sound_engine);

    // Start keyboard listener
    let keyboard_handler = input::KeyboardHandler::new(sound_engine.clone())?;
    keyboard_handler.start()?;

    // Create and run the tray icon
    let tray = ui::TrayIcon::new()?;
    tray.run()?;

    // Keep the main thread running
    #[cfg(target_os = "macos")]
    unsafe {
        let app = NSApplication::sharedApplication(nil);
        app.run();
    }

    Ok(())
}