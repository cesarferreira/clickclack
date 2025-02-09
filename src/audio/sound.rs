use anyhow::Result;
use log::error;
use rodio::{Decoder, OutputStream, Sink};
use rdev::Key;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use std::sync::mpsc;

pub struct SoundEngine {
    _stream: OutputStream,  // Keep stream alive but don't share it
    stream_handle: rodio::OutputStreamHandle,
    sinks: Arc<Mutex<Vec<Sink>>>,
    sender: mpsc::Sender<SoundEvent>,
}

pub struct SoundEvent {
    key: Option<Key>,
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
            sinks: Arc::new(Mutex::new(Vec::new())),
            sender,
        })
    }

    pub fn play_click(&self, key: Option<Key>) {
        let app_state = crate::APP_STATE.lock();
        if !app_state.enabled {
            return;
        }

        // Create event with current state
        let event = SoundEvent {
            key,
            volume: app_state.volume,
            profile: app_state.keyboard_profile.clone(),
        };

        // Send event to audio thread
        let _ = self.sender.send(event);
    }

    fn handle_sound_event(event: SoundEvent, stream_handle: &rodio::OutputStreamHandle) {
        // Determine which sound file to play based on the key
        let sound_file = match event.key {
            Some(Key::Return) => "down_enter.mp3".to_string(),
            Some(Key::Space) => "down_space.mp3".to_string(),
            Some(_) => {
                // Use different down sounds for variety
                let num = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() % 7 + 1) as u8;
                format!("down{}.mp3", num)
            }
            None => "down1.mp3".to_string(),
        };

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
            volume: app_state.volume,
            profile: app_state.keyboard_profile.clone(),
        };
        self.sender.send(event).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_sound_engine_creation() {
        assert!(SoundEngine::new().is_ok());
    }

    #[test]
    fn test_sound_file_selection() {
        let engine = SoundEngine::new().unwrap();
        
        // Test with test sound instead of real files
        assert!(engine.play_test_sound());
        
        // Check that sink was created
        {
            let sinks = engine.sinks.lock();
            assert!(!sinks.is_empty());
        }
    }

    #[test]
    fn test_concurrent_sounds() {
        let engine = SoundEngine::new().unwrap();
        
        // Play multiple test sounds
        for _ in 0..3 {
            assert!(engine.play_test_sound());
        }

        // Check that multiple sinks are created
        {
            let sinks = engine.sinks.lock();
            assert!(sinks.len() > 1);
        }
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