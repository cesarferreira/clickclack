use anyhow::Result;
use rdev::{listen, Event, EventType, Key};
use std::sync::Arc;
use log::{error, info};
use std::collections::HashSet;
use parking_lot::Mutex;

use crate::audio::SoundEngine;

pub struct KeyboardHandler {
    sound_engine: Arc<SoundEngine>,
    pressed_keys: Arc<Mutex<HashSet<Key>>>,
}

impl KeyboardHandler {
    pub fn new(sound_engine: Arc<SoundEngine>) -> Result<Self> {
        Ok(Self {
            sound_engine,
            pressed_keys: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub fn start(&self) -> Result<()> {
        let sound_engine = self.sound_engine.clone();
        let pressed_keys = self.pressed_keys.clone();
        info!("Starting keyboard listener...");
        
        std::thread::spawn(move || {
            if let Err(error) = listen(move |event| {
                Self::callback(event, &sound_engine, &pressed_keys);
            }) {
                error!("Failed to listen for keyboard events: {:?}", error);
            }
        });

        Ok(())
    }

    fn callback(event: Event, sound_engine: &SoundEngine, pressed_keys: &Arc<Mutex<HashSet<Key>>>) {
        match event.event_type {
            EventType::KeyPress(key) => {
                // Only play sound if the key wasn't already pressed
                let should_play = {
                    let mut keys = pressed_keys.lock();
                    if !keys.contains(&key) {
                        keys.insert(key);
                        true
                    } else {
                        false
                    }
                };

                if should_play {
                    info!("Key pressed: {:?}", key);
                    let enabled = {
                        let app_state = crate::APP_STATE.lock();
                        app_state.enabled
                    };
                    if enabled {
                        sound_engine.play_sound(Some(key), true);
                    }
                }
            }
            EventType::KeyRelease(key) => {
                // Only play sound if we had registered this key as pressed
                let should_play = {
                    let mut keys = pressed_keys.lock();
                    keys.remove(&key)
                };

                if should_play {
                    info!("Key released: {:?}", key);
                    let enabled = {
                        let app_state = crate::APP_STATE.lock();
                        app_state.enabled
                    };
                    if enabled {
                        sound_engine.play_sound(Some(key), false);
                    }
                }
            }
            _ => {}
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
        let pressed_keys = Arc::new(Mutex::new(HashSet::new()));
        
        // Ensure app is enabled
        {
            let mut app_state = crate::APP_STATE.lock();
            app_state.enabled = true;
        }

        // Test normal key press and release sequence
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, true), &sound_engine, &pressed_keys);
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, false), &sound_engine, &pressed_keys);

        // Test holding a key (second press should not trigger sound)
        KeyboardHandler::callback(create_test_event(rdev::Key::Space, true), &sound_engine, &pressed_keys);
        KeyboardHandler::callback(create_test_event(rdev::Key::Space, true), &sound_engine, &pressed_keys); // Should not play
        KeyboardHandler::callback(create_test_event(rdev::Key::Space, false), &sound_engine, &pressed_keys);

        // Test multiple keys
        KeyboardHandler::callback(create_test_event(rdev::Key::Return, true), &sound_engine, &pressed_keys);
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyB, true), &sound_engine, &pressed_keys);
        KeyboardHandler::callback(create_test_event(rdev::Key::Return, false), &sound_engine, &pressed_keys);
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyB, false), &sound_engine, &pressed_keys);
    }

    #[test]
    fn test_callback_disabled() {
        let sound_engine = Arc::new(SoundEngine::new().unwrap());
        let pressed_keys = Arc::new(Mutex::new(HashSet::new()));
        
        // Disable app
        {
            let mut app_state = crate::APP_STATE.lock();
            app_state.enabled = false;
        }

        // Test callback while disabled
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, true), &sound_engine, &pressed_keys);
        KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, false), &sound_engine, &pressed_keys);
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
            let pressed_keys = handler.pressed_keys.clone();
            thread::spawn(move || {
                KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, true), &engine, &pressed_keys);
                KeyboardHandler::callback(create_test_event(rdev::Key::KeyA, false), &engine, &pressed_keys);
            })
        }).collect();

        // Wait for all threads to complete
        for thread in threads {
            thread.join().unwrap();
        }
    }
} 