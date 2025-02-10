use anyhow::Result;
use log::{error, info, debug};
use rodio::{Decoder, OutputStream, Sink};
use rdev::Key;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc;
use dirs;

fn get_assets_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("clickclack")
}

pub struct SoundEngine {
    _stream: OutputStream,
    stream_handle: rodio::OutputStreamHandle,
    sender: mpsc::Sender<SoundEvent>,
}

pub struct SoundEvent {
    key: Option<Key>,
    is_press: bool,
    volume: f32,
    switch_type: String,
}

// Implement Send and Sync explicitly since we control the thread safety
unsafe impl Send for SoundEngine {}
unsafe impl Sync for SoundEngine {}

impl SoundEngine {
    pub fn new() -> Result<Self> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        let (sender, receiver) = mpsc::channel();

        // Spawn a thread to handle sound events
        let stream_handle_clone = stream_handle.clone();
        std::thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                Self::handle_sound_event(event, &stream_handle_clone);
            }
        });

        Ok(Self {
            _stream: stream,
            stream_handle,
            sender,
        })
    }

    pub fn play_sound(&self, key: Option<Key>, is_press: bool) {
        let app_state = crate::APP_STATE.lock();
        if !app_state.enabled {
            return;
        }

        // Create event with current state
        let event = SoundEvent {
            key,
            is_press,
            volume: app_state.volume,
            switch_type: app_state.switch_type.clone(),
        };

        // Send event to audio thread
        let _ = self.sender.send(event);
    }

    fn get_sound_number_for_key(key: &Key) -> String {
        // Map specific keys to row numbers for GENERIC_R0-R4 sounds
        match key {
            // Row 0 - Number keys
            Key::Num1 | Key::Num2 | Key::Num3 | Key::Num4 | Key::Num5 |
            Key::Num6 | Key::Num7 | Key::Num8 | Key::Num9 | Key::Num0 |
            Key::Minus | Key::Equal => "GENERIC_R0",
            
            // Row 1 - Top letter row
            Key::KeyQ | Key::KeyW | Key::KeyE | Key::KeyR | Key::KeyT |
            Key::KeyY | Key::KeyU | Key::KeyI | Key::KeyO | Key::KeyP |
            Key::LeftBracket | Key::RightBracket => "GENERIC_R1",
            
            // Row 2 - Home row
            Key::KeyA | Key::KeyS | Key::KeyD | Key::KeyF | Key::KeyG |
            Key::KeyH | Key::KeyJ | Key::KeyK | Key::KeyL |
            Key::SemiColon | Key::Quote | Key::BackSlash => "GENERIC_R2",
            
            // Row 3 - Bottom letter row
            Key::KeyZ | Key::KeyX | Key::KeyC | Key::KeyV | Key::KeyB |
            Key::KeyN | Key::KeyM | Key::Comma | Key::Dot | Key::Slash => "GENERIC_R3",
            
            // Row 4 - Space row and modifiers
            Key::Space | Key::Alt | Key::MetaLeft | Key::MetaRight |
            Key::ControlLeft | Key::ControlRight | Key::ShiftLeft |
            Key::ShiftRight => "GENERIC_R4",
            
            // Default to R2 (home row) for any other keys
            _ => "GENERIC_R2"
        }.to_string()
    }

    fn handle_sound_event(event: SoundEvent, stream_handle: &rodio::OutputStreamHandle) {
        // Determine which sound file to play based on the key and event type
        let sound_file = match (event.key, event.is_press) {
            (Some(key), true) => {
                format!("press/{}.mp3", Self::get_sound_number_for_key(&key))
            }
            (Some(_), false) => {
                "release/GENERIC.mp3".to_string()
            }
            (None, _) => "press/GENERIC_R2.mp3".to_string(),
        };

        info!("Key sound: {}", sound_file);

        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("clickclack")
            .join("switchtypes")
            .join(&event.switch_type)
            .join(&sound_file);

        // Create a new sink for this sound
        if let Ok(sink) = Sink::try_new(stream_handle) {
            match File::open(&path) {
                Ok(file) => {
                    let reader = BufReader::new(file);
                    match Decoder::new(reader) {
                        Ok(source) => {
                            sink.set_volume(event.volume);
                            sink.append(source);
                            sink.detach(); // Let the sink clean itself up when done
                        }
                        Err(e) => error!("Failed to decode audio: {:?}", e),
                    }
                }
                Err(e) => error!("Failed to open sound file {:?}: {:?}", path, e),
            }
        }
    }

    #[cfg(test)]
    pub fn play_test_sound(&self) -> bool {
        let app_state = crate::APP_STATE.lock();
        let event = SoundEvent {
            key: None,
            is_press: true,
            volume: app_state.volume,
            switch_type: app_state.switch_type.clone(),
        };
        self.sender.send(event).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_engine_creation() {
        assert!(SoundEngine::new().is_ok());
    }

    #[test]
    fn test_sound_file_selection() {
        let engine = SoundEngine::new().unwrap();
        assert!(engine.play_test_sound());
        // Give some time for the sound to be processed
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_concurrent_sounds() {
        let engine = SoundEngine::new().unwrap();
        
        // Play multiple test sounds
        for _ in 0..3 {
            assert!(engine.play_test_sound());
        }
        // Give some time for the sounds to be processed
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_volume_control() {
        let engine = SoundEngine::new().unwrap();
        {
            let mut app_state = crate::APP_STATE.lock();
            app_state.volume = 0.5;
        }
        assert!(engine.play_test_sound());
    }
} 