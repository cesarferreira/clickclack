use anyhow::Result;
use rdev::{listen, Event, EventType};
use std::sync::Arc;
use log::error;

use crate::audio::SoundEngine;

pub struct KeyboardHandler {
    sound_engine: Arc<SoundEngine>,
}

impl KeyboardHandler {
    pub fn new(sound_engine: Arc<SoundEngine>) -> Result<Self> {
        Ok(Self { sound_engine })
    }

    pub fn start(&self) -> Result<()> {
        let sound_engine = self.sound_engine.clone();
        
        std::thread::spawn(move || {
            if let Err(error) = listen(move |event| {
                Self::callback(event, &sound_engine);
            }) {
                error!("Failed to listen for keyboard events: {:?}", error);
            }
        });

        Ok(())
    }

    fn callback(event: Event, sound_engine: &SoundEngine) {
        if let EventType::KeyPress(_) = event.event_type {
            let app_state = crate::APP_STATE.lock();
            if app_state.enabled {
                sound_engine.play_click();
            }
        }
    }
} 