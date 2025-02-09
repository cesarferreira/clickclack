use anyhow::Result;
use rdev::{listen, Event, EventType};
use std::sync::Arc;
use log::{error, info};

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
        info!("Starting keyboard listener...");
        
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
        if let EventType::KeyPress(key) = event.event_type {
            info!("Clicked on key {:?}", key);
            // Only hold the lock long enough to check if enabled
            let enabled = {
                let app_state = crate::APP_STATE.lock();
                app_state.enabled
            };
            
            if enabled {
                sound_engine.play_click(Some(key));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn create_test_event(key: rdev::Key) -> Event {
        Event {
            event_type: EventType::KeyPress(key),
            name: None,
            time: std::time::SystemTime::now(),
        }
    }

    #[test]
    fn test_keyboard_handler_creation() {
        let sound_engine = Arc::new(SoundEngine::new().unwrap());
        assert!(KeyboardHandler::new(sound_engine).is_ok());
    }

    #[test]
    fn test_keyboard_handler_start() {
        let sound_engine = Arc::new(SoundEngine::new().unwrap());
        let handler = KeyboardHandler::new(sound_engine).unwrap();
        assert!(handler.start().is_ok());
    }

    #[test]
    fn test_callback_enabled() {
        let sound_engine = Arc::new(SoundEngine::new().unwrap());
        
        // Ensure app is enabled
        {
            let mut app_state = crate::APP_STATE.lock();
            app_state.enabled = true;
        }

        // Test callback with different keys
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyA), &sound_engine);
        KeyboardHandler::callback(create_test_event(rdev::Key::Space), &sound_engine);
        KeyboardHandler::callback(create_test_event(rdev::Key::Return), &sound_engine);
    }

    #[test]
    fn test_callback_disabled() {
        let sound_engine = Arc::new(SoundEngine::new().unwrap());
        
        // Disable app
        {
            let mut app_state = crate::APP_STATE.lock();
            app_state.enabled = false;
        }

        // Test callback while disabled
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyA), &sound_engine);
    }

    #[test]
    fn test_thread_safety() {
        let sound_engine = Arc::new(SoundEngine::new().unwrap());
        let handler = KeyboardHandler::new(sound_engine.clone()).unwrap();
        
        // Start the handler
        handler.start().unwrap();
        
        // Test concurrent access from multiple threads
        let threads: Vec<_> = (0..3).map(|_| {
            let engine = sound_engine.clone();
            thread::spawn(move || {
                KeyboardHandler::callback(create_test_event(rdev::Key::KeyA), &engine);
            })
        }).collect();

        // Wait for all threads to complete
        for thread in threads {
            thread.join().unwrap();
        }
    }
} 