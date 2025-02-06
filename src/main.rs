mod audio;
mod input;
mod ui;

use anyhow::Result;
use log::{error, info};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;

static APP_STATE: Lazy<Arc<Mutex<AppState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(AppState {
        enabled: true,
        volume: 1.0,
        frequency: 500.0,
        decay: 30.0,
    }))
});

pub struct AppState {
    enabled: bool,
    volume: f32,
    frequency: f32,
    decay: f32,
}

fn main() -> Result<()> {
    env_logger::init();
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

    Ok(())
}
