//! Audio input (microphone recording) functionality using `cpal`

use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::{AudioConfig, VoiceError, VoiceResult};

/// Records audio from a microphone device
#[derive(Debug)]
pub struct AudioInput {
    /// Audio configuration
    config: AudioConfig,
    /// Name of the input device (None = default)
    device_name: Option<String>,
}

impl Default for AudioInput {
    fn default() -> Self {
        Self {
            config: AudioConfig::default(),
            device_name: None,
        }
    }
}

impl AudioInput {
    /// Create a new AudioInput with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new AudioInput with custom configuration
    pub fn with_config(config: AudioConfig) -> Self {
        Self {
            config,
            device_name: None,
        }
    }

    /// Set a specific input device by name
    pub fn with_device(mut self, name: &str) -> Self {
        self.device_name = Some(name.to_string());
        self
    }

    /// List available input devices
    pub fn list_devices() -> VoiceResult<Vec<String>> {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|e| VoiceError::AudioInput(format!("Failed to list input devices: {e}")))?;

        let names: Vec<String> = devices.filter_map(|d| d.name().ok()).collect();

        Ok(names)
    }

    /// Record audio for a specified duration
    ///
    /// Returns raw PCM audio samples as `Vec<f32>`.
    pub fn record(&self, duration_seconds: f32) -> VoiceResult<Vec<f32>> {
        let host = cpal::default_host();
        let device = self.select_device(&host)?;
        let config = device
            .default_input_config()
            .map_err(|e| VoiceError::StreamConfig(format!("Failed to get input config: {e}")))?;

        let err_fn = |err| eprintln!("Audio input stream error: {err}");
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let (tx, rx): (mpsc::Sender<f32>, Receiver<f32>) = mpsc::channel();
        let writer = Arc::new(tx);

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    for &sample in data {
                        let _ = writer.send(sample);
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| VoiceError::AudioInput(format!("Failed to build input stream: {e}")))?;

        stream
            .play()
            .map_err(|e| VoiceError::AudioInput(format!("Failed to start input stream: {e}")))?;

        let total_samples = (sample_rate as f32 * duration_seconds * channels as f32) as usize;
        let mut samples: Vec<f32> = Vec::with_capacity(total_samples);

        while samples.len() < total_samples {
            match rx.recv() {
                Ok(sample) => samples.push(sample),
                Err(_) => break,
            }
        }

        drop(stream);
        Ok(samples)
    }

    /// Select the input device (specific or default)
    fn select_device(&self, host: &cpal::Host) -> VoiceResult<cpal::Device> {
        if let Some(ref name) = self.device_name {
            let devices = host
                .input_devices()
                .map_err(|e| VoiceError::AudioInput(format!("Failed to list devices: {e}")))?;

            devices
                .into_iter()
                .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                .ok_or_else(|| {
                    VoiceError::DeviceNotFound(format!("Input device '{name}' not found"))
                })
        } else {
            host.default_input_device().ok_or_else(|| {
                VoiceError::DeviceNotFound("No default input device found".to_string())
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_input_new() {
        let input = AudioInput::new();
        assert_eq!(input.config.sample_rate, 44100);
        assert!(input.device_name.is_none());
    }

    #[test]
    fn test_audio_input_with_config() {
        let config = AudioConfig {
            sample_rate: 48000,
            channels: 2,
            buffer_seconds: 0.05,
        };
        let input = AudioInput::with_config(config);
        assert_eq!(input.config.sample_rate, 48000);
    }

    #[test]
    fn test_audio_input_with_device() {
        let input = AudioInput::new().with_device("Test Microphone");
        assert_eq!(input.device_name, Some("Test Microphone".to_string()));
    }
}
