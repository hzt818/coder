//! Voice module - audio input and output capabilities
//!
//! Provides audio recording from microphones and audio playback
//! using the `cpal` crate for audio I/O and `hound` for WAV encoding/decoding.

pub mod input;
pub mod output;

pub use input::AudioInput;
pub use output::AudioOutput;

/// Result type for voice operations
pub type VoiceResult<T> = std::result::Result<T, VoiceError>;

/// Errors that can occur during voice operations
#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    /// Audio input (microphone) error
    #[error("Audio input error: {0}")]
    AudioInput(String),
    /// Audio output (playback) error
    #[error("Audio output error: {0}")]
    AudioOutput(String),
    /// Device not found
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    /// Stream configuration error
    #[error("Stream config error: {0}")]
    StreamConfig(String),
    /// WAV format error
    #[error("WAV error: {0}")]
    Wav(String),
    /// Initialization failed
    #[error("Voice module initialization failed: {0}")]
    Init(String),
}

/// Audio format configuration
#[derive(Debug, Clone, Copy)]
pub struct AudioConfig {
    /// Sample rate in Hz (e.g., 44100)
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u16,
    /// Buffer duration in seconds
    pub buffer_seconds: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            channels: 1,
            buffer_seconds: 0.1,
        }
    }
}

/// Supported audio formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioFormat {
    /// WAV format
    Wav,
    /// Raw PCM format
    Pcm,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 44100);
        assert_eq!(config.channels, 1);
        assert!((config.buffer_seconds - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_audio_config_custom() {
        let config = AudioConfig {
            sample_rate: 48000,
            channels: 2,
            buffer_seconds: 0.05,
        };
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
    }

    #[test]
    fn test_voice_error_display() {
        let err = VoiceError::AudioInput("microphone unavailable".to_string());
        assert_eq!(err.to_string(), "Audio input error: microphone unavailable");
    }
}
