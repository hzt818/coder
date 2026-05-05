//! Audio output (playback) functionality using `cpal` and `hound`

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::{AudioConfig, AudioFormat, VoiceError, VoiceResult};

/// Plays audio through speakers or output devices
#[derive(Debug)]
pub struct AudioOutput {
    /// Audio configuration
    config: AudioConfig,
    /// Name of the output device (None = default)
    device_name: Option<String>,
}

impl Default for AudioOutput {
    fn default() -> Self {
        Self {
            config: AudioConfig::default(),
            device_name: None,
        }
    }
}

impl AudioOutput {
    /// Create a new AudioOutput with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new AudioOutput with custom configuration
    pub fn with_config(config: AudioConfig) -> Self {
        Self {
            config,
            device_name: None,
        }
    }

    /// Set a specific output device by name
    pub fn with_device(mut self, name: &str) -> Self {
        self.device_name = Some(name.to_string());
        self
    }

    /// List available output devices
    pub fn list_devices() -> VoiceResult<Vec<String>> {
        let host = cpal::default_host();
        let devices = host
            .output_devices()
            .map_err(|e| VoiceError::AudioOutput(format!("Failed to list output devices: {e}")))?;

        let names: Vec<String> = devices
            .filter_map(|d| d.name().ok())
            .collect();

        Ok(names)
    }

    /// Play raw PCM audio samples
    pub fn play_samples(&self, samples: &[f32]) -> VoiceResult<()> {
        let host = cpal::default_host();
        let device = self.select_device(&host)?;
        let config = device
            .default_output_config()
            .map_err(|e| VoiceError::StreamConfig(format!("Failed to get output config: {e}")))?;

        let err_fn = |err| eprintln!("Audio output stream error: {err}");
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let samples: Vec<f32> = samples.to_vec();
        let mut write_offset = 0;

        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for sample in data.iter_mut() {
                        *sample = samples.get(write_offset).copied().unwrap_or(0.0);
                        write_offset = (write_offset + 1) % samples.len().max(1);
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| VoiceError::AudioOutput(format!("Failed to build output stream: {e}")))?;

        stream
            .play()
            .map_err(|e| VoiceError::AudioOutput(format!("Failed to start output stream: {e}")))?;

        // Let playback run for the duration of the samples
        let duration_secs = samples.len() as f32 / sample_rate as f32 / channels as f32;
        std::thread::sleep(std::time::Duration::from_secs_f32(duration_secs));

        drop(stream);
        Ok(())
    }

    /// Read WAV audio data from a byte buffer and play it
    pub fn play_wav_bytes(&self, wav_data: &[u8]) -> VoiceResult<()> {
        let reader = std::io::Cursor::new(wav_data);
        let mut reader = hound::WavReader::new(reader)
            .map_err(|e| VoiceError::Wav(format!("Failed to read WAV data: {e}")))?;

        let spec = reader.spec();
        let samples: Vec<f32> = reader
            .samples::<i16>()
            .filter_map(|s| s.ok())
            .map(|s| s as f32 / i16::MAX as f32)
            .collect();

        let config = AudioConfig {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            ..self.config
        };

        let output = Self::with_config(config);
        output.play_samples(&samples)
    }

    /// Write audio samples to a WAV file
    pub fn save_as_wav(&self, samples: &[f32], path: &std::path::Path) -> VoiceResult<()> {
        let spec = hound::WavSpec {
            channels: self.config.channels,
            sample_rate: self.config.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec)
            .map_err(|e| VoiceError::Wav(format!("Failed to create WAV file: {e}")))?;

        for &sample in samples {
            let amplitude = (sample * i16::MAX as f32) as i16;
            writer
                .write_sample(amplitude)
                .map_err(|e| VoiceError::Wav(format!("Failed to write sample: {e}")))?;
        }

        writer
            .finalize()
            .map_err(|e| VoiceError::Wav(format!("Failed to finalize WAV: {e}")))?;

        Ok(())
    }

    /// Get the format of this output
    pub fn format(&self) -> AudioFormat {
        AudioFormat::Pcm
    }

    /// Select the output device (specific or default)
    fn select_device(&self, host: &cpal::Host) -> VoiceResult<cpal::Device> {
        if let Some(ref name) = self.device_name {
            let devices = host
                .output_devices()
                .map_err(|e| VoiceError::AudioOutput(format!("Failed to list devices: {e}")))?;

            devices
                .into_iter()
                .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                .ok_or_else(|| VoiceError::DeviceNotFound(format!("Output device '{name}' not found")))
        } else {
            host.default_output_device()
                .ok_or_else(|| VoiceError::DeviceNotFound("No default output device found".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_output_new() {
        let output = AudioOutput::new();
        assert_eq!(output.config.sample_rate, 44100);
        assert!(output.device_name.is_none());
    }

    #[test]
    fn test_audio_output_with_config() {
        let config = AudioConfig {
            sample_rate: 48000,
            channels: 2,
            buffer_seconds: 0.05,
        };
        let output = AudioOutput::with_config(config);
        assert_eq!(output.config.sample_rate, 48000);
    }

    #[test]
    fn test_audio_output_with_device() {
        let output = AudioOutput::new().with_device("Test Speaker");
        assert_eq!(output.device_name, Some("Test Speaker".to_string()));
    }

    #[test]
    fn test_save_as_wav() {
        let output = AudioOutput::new();
        let samples = vec![0.0f32; 100];
        let dir = std::env::temp_dir();
        let path = dir.join("test_output.wav");
        let result = output.save_as_wav(&samples, &path);
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_audio_format() {
        let output = AudioOutput::new();
        assert_eq!(output.format(), AudioFormat::Pcm);
    }
}
