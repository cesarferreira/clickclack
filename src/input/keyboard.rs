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
        let (is_press, key) = match event.event_type {
            EventType::KeyPress(key) => {
                info!("Key pressed: {:?}", key);
                (true, Some(key))
            }
            EventType::KeyRelease(key) => {
                info!("Key released: {:?}", key);
                (false, Some(key))
            }
            _ => return,
        };

        // Only hold the lock long enough to check if enabled
        let enabled = {
            let app_state = crate::APP_STATE.lock();
            app_state.enabled
        };
        
        if enabled {
            sound_engine.play_sound(key, is_press);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn create_test_event(key: rdev::Key, is_press: bool) -> Event {
        Event {
            event_type: if is_press {
                EventType::KeyPress(key)
            } else {
                EventType::KeyRelease(key)
            },
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

        // Test callback with different keys (press and release)
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, true), &sound_engine);
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, false), &sound_engine);
        KeyboardHandler::callback(create_test_event(rdev::Key::Space, true), &sound_engine);
        KeyboardHandler::callback(create_test_event(rdev::Key::Space, false), &sound_engine);
        KeyboardHandler::callback(create_test_event(rdev::Key::Return, true), &sound_engine);
        KeyboardHandler::callback(create_test_event(rdev::Key::Return, false), &sound_engine);
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
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, true), &sound_engine);
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, false), &sound_engine);
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
                KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, true), &engine);
                KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, false), &engine);
            })
        }).collect();

        // Wait for all threads to complete
        for thread in threads {
            thread.join().unwrap();
        }
    }
} 