use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, Stream, SizedSample};
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Instant;
use parking_lot::Mutex;
pub struct SoundEngine {
    _stream: Stream,
    _sample_rate: f32,
    last_click: Arc<Mutex<Option<Instant>>>,
}
// Implement Send and Sync explicitly since we know our implementation is thread-safe
unsafe impl Send for SoundEngine {}
unsafe impl Sync for SoundEngine {}

impl SoundEngine {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No output device available"))?;
        
        let config = device.default_output_config()?;
        let sample_rate = config.sample_rate().0 as f32;
        let last_click = Arc::new(Mutex::new(None));
        
        let stream = match config.sample_format() {
            SampleFormat::F32 => Self::create_stream::<f32>(&device, &config.into(), last_click.clone())?,
            SampleFormat::I16 => Self::create_stream::<i16>(&device, &config.into(), last_click.clone())?,
            SampleFormat::U16 => Self::create_stream::<u16>(&device, &config.into(), last_click.clone())?,
            _ => return Err(anyhow::anyhow!("Unsupported sample format")),
        };

        stream.play()?;

        Ok(Self {
            _stream: stream,
            _sample_rate: sample_rate,
            last_click,
        })
    }

    fn create_stream<T>(device: &cpal::Device, config: &cpal::StreamConfig, last_click: Arc<Mutex<Option<Instant>>>) -> Result<Stream>
    where
        T: SizedSample + Sample + cpal::FromSample<f32>,
    {
        let sample_rate = config.sample_rate.0 as f32;
        let channels = config.channels as usize;
        let last_click = Arc::clone(&last_click);

        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                Self::write_data(data, channels, sample_rate, &last_click);
            },
            |err| eprintln!("Error in audio stream: {}", err),
            None,
        )?;

        Ok(stream)
    }

    fn write_data<T>(output: &mut [T], channels: usize, _sample_rate: f32, last_click: &Arc<Mutex<Option<Instant>>>)
    where
        T: SizedSample + Sample + cpal::FromSample<f32>,
    {
        let app_state = crate::APP_STATE.lock();
        if !app_state.enabled {
            for sample in output.iter_mut() {
                *sample = T::EQUILIBRIUM;
            }
            return;
        }

        let mut last_click = last_click.lock();
        if let Some(start_time) = *last_click {
            let elapsed = start_time.elapsed().as_secs_f32();
            
            for frame in output.chunks_mut(channels) {
                let value = Self::generate_sample(
                    app_state.frequency,
                    app_state.decay,
                    app_state.volume,
                    elapsed,
                );
                
                let sample_value = T::from_sample(value);
                for sample in frame.iter_mut() {
                    *sample = sample_value;
                }
            }

            // Stop generating sound after 0.1 seconds
            if elapsed > 0.1 {
                *last_click = None;
            }
        } else {
            // No active click, output silence
            for sample in output.iter_mut() {
                *sample = T::EQUILIBRIUM;
            }
        }
    }

    fn generate_sample(frequency: f32, decay: f32, volume: f32, t: f32) -> f32 {
        // Improved mechanical keyboard sound synthesis
        let base_freq = frequency;
        let overtone_freq = frequency * 1.5;
        let second_overtone_freq = frequency * 2.0;
        
        // Base click
        let base = (2.0 * PI * base_freq * t).sin() * (-t * decay).exp();
        
        // Add overtones for more "mechanical" character
        let overtone = 0.5 * (2.0 * PI * overtone_freq * t).sin() * (-t * (decay * 1.2)).exp();
        let second_overtone = 0.25 * (2.0 * PI * second_overtone_freq * t).sin() * (-t * (decay * 1.5)).exp();
        
        // Add some noise for the "clack" effect
        let noise = (t * 1000.0).sin() * 0.1 * (-t * (decay * 2.0)).exp();
        
        // Combine all components
        let value = volume * (base + overtone + second_overtone + noise);
        value.max(-1.0).min(1.0)
    }

    pub fn play_click(&self) {
        *self.last_click.lock() = Some(Instant::now());
    }
}

impl Drop for SoundEngine {
    fn drop(&mut self) {
        self._stream.pause().unwrap_or_else(|e| {
            eprintln!("Error stopping audio stream: {}", e);
        });
    }
} 