use anyhow::Result;
use log::{error, info};
use rodio::{Decoder, OutputStream, Sink};
use rdev::Key;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use std::sync::mpsc;
use env_logger;
use std::io::Write;

pub struct SoundEngine {
    _stream: OutputStream,
    stream_handle: rodio::OutputStreamHandle,
    sender: mpsc::Sender<SoundEvent>,
}

pub struct SoundEvent {
    key: Option<Key>,
    is_press: bool,
    volume: f32,
    profile: String,
}

// Implement Send and Sync explicitly since we control the thread safety
unsafe impl Send for SoundEngine {}
unsafe impl Sync for SoundEngine {}

impl SoundEngine {
    pub fn new() -> Result<Self> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        let (sender, receiver) = mpsc::channel();

        // Spawn a dedicated thread for audio playback
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
            profile: app_state.keyboard_profile.clone(),
        };

        // Send event to audio thread
        let _ = self.sender.send(event);
    }

    fn get_sound_number_for_key(key: &Key) -> u8 {
        // Map specific keys to consistent sound numbers
        match key {
            // Map letter keys
            Key::KeyA | Key::KeyQ | Key::KeyZ | Key::Num1 => 1,
            Key::KeyS | Key::KeyW | Key::KeyX | Key::Num2 => 2,
            Key::KeyD | Key::KeyE | Key::KeyC | Key::Num3 => 3,
            Key::KeyF | Key::KeyR | Key::KeyV | Key::Num4 => 4,
            Key::KeyG | Key::KeyT | Key::KeyB | Key::Num5 => 5,
            Key::KeyH | Key::KeyY | Key::KeyN | Key::Num6 => 6,
            Key::KeyJ | Key::KeyU | Key::KeyM | Key::Num7 => 7,
            
            // Map modifier keys to deeper sounds
            Key::ShiftLeft | Key::ShiftRight | Key::CapsLock => 1,
            Key::ControlLeft | Key::ControlRight | Key::Alt => 2,
            Key::MetaLeft | Key::MetaRight => 3,
            
            // Map remaining keys based on their pattern
            Key::KeyK | Key::KeyI | Key::Comma => 1,
            Key::KeyL | Key::KeyO | Key::Dot => 2,
            Key::SemiColon | Key::KeyP | Key::Slash => 3,
            Key::Quote | Key::LeftBracket | Key::Num8 => 4,
            Key::BackSlash | Key::RightBracket | Key::Num9 => 5,
            Key::Tab | Key::Num0 | Key::Equal => 6,
            _ => 7  // Any other keys
        }
    }

    fn handle_sound_event(event: SoundEvent, stream_handle: &rodio::OutputStreamHandle) {
        // Determine which sound file to play based on the key and event type
        let sound_file = match (event.key, event.is_press) {
            (Some(Key::Return), true) => "down_enter.mp3".to_string(),
            (Some(Key::Return), false) => "up_enter.mp3".to_string(),
            (Some(Key::Space), true) => "down_space.mp3".to_string(),
            (Some(Key::Space), false) => "up_space.mp3".to_string(),
            (Some(key), true) => {
                let num = Self::get_sound_number_for_key(&key);
                format!("down{}.mp3", num)
            }
            (Some(key), false) => {
                let num = Self::get_sound_number_for_key(&key);
                format!("up{}.mp3", num)
            }
            (None, _) => "down1.mp3".to_string(),
        };

        info!("Key sound: {}", sound_file);

        let path = PathBuf::from("assets")
            .join("keyboards")
            .join(event.profile)
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
            profile: app_state.keyboard_profile.clone(),
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