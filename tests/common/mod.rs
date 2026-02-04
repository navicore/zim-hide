//! Shared test utilities for generating synthetic WAV files with various characteristics.

use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::Path;
use tempfile::NamedTempFile;

/// Audio pattern types for test generation
#[derive(Debug, Clone, Copy)]
pub enum AudioPattern {
    /// Pure sine wave at specified frequency (Hz)
    Sine(f32),
    /// Digital silence (all zeros)
    Silence,
    /// White noise (random samples)
    WhiteNoise,
    /// Multiple mixed frequencies for complex waveform
    MultiFrequency,
    /// Amplitude sweep from quiet to loud
    AmplitudeSweep,
    /// Near-clipping loud audio (tests edge of sample range)
    LoudClipping,
    /// Very quiet audio (tests low amplitude samples)
    VeryQuiet,
    /// Square wave (harsh transitions, tests LSB behavior)
    Square(f32),
}

/// Configuration for test WAV generation
#[derive(Debug, Clone)]
pub struct TestWavConfig {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub duration_secs: f32,
    pub pattern: AudioPattern,
    /// Amplitude multiplier (0.0 to 1.0, where 1.0 = max i16)
    pub amplitude: f32,
}

impl Default for TestWavConfig {
    fn default() -> Self {
        Self {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            duration_secs: 1.0,
            pattern: AudioPattern::Sine(440.0),
            amplitude: 0.6,
        }
    }
}

impl TestWavConfig {
    pub fn mono(mut self) -> Self {
        self.channels = 1;
        self
    }

    #[allow(dead_code)]
    pub fn stereo(mut self) -> Self {
        self.channels = 2;
        self
    }

    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }

    pub fn duration(mut self, secs: f32) -> Self {
        self.duration_secs = secs;
        self
    }

    pub fn pattern(mut self, pattern: AudioPattern) -> Self {
        self.pattern = pattern;
        self
    }

    #[allow(dead_code)]
    pub fn amplitude(mut self, amp: f32) -> Self {
        self.amplitude = amp.clamp(0.0, 1.0);
        self
    }

    /// Create a temporary WAV file with this configuration
    #[allow(dead_code)]
    pub fn create_temp_file(&self) -> NamedTempFile {
        let temp = NamedTempFile::new().expect("Failed to create temp file");
        self.write_to_path(temp.path());
        temp
    }

    /// Write WAV data to a specific path
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn write_to_path(&self, path: &Path) {
        let spec = WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: self.bits_per_sample,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(path, spec).expect("Failed to create WAV writer");

        let total_samples = (self.sample_rate as f32 * self.duration_secs) as u32;
        let max_amplitude = i16::MAX as f32 * self.amplitude;

        // Simple PRNG for reproducible "random" noise (xorshift)
        let mut rng_state: u32 = 0xDEAD_BEEF;
        let mut next_random = || {
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 17;
            rng_state ^= rng_state << 5;
            // Convert to -1.0 to 1.0 range
            (rng_state as f32 / u32::MAX as f32).mul_add(2.0, -1.0)
        };

        for i in 0..total_samples {
            let t = i as f32 / self.sample_rate as f32;

            let sample_value = match self.pattern {
                AudioPattern::Sine(freq) => {
                    (t * freq * 2.0 * std::f32::consts::PI).sin() * max_amplitude
                }
                AudioPattern::Silence => 0.0,
                AudioPattern::WhiteNoise => next_random() * max_amplitude,
                AudioPattern::MultiFrequency => {
                    // Mix of 220Hz, 440Hz, 880Hz, 1760Hz (harmonics)
                    let base = 220.0;
                    let s1 = (t * base * 2.0 * std::f32::consts::PI).sin();
                    let s2 = (t * base * 2.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
                    let s3 = (t * base * 4.0 * 2.0 * std::f32::consts::PI).sin() * 0.25;
                    let s4 = (t * base * 8.0 * 2.0 * std::f32::consts::PI).sin() * 0.125;
                    (s1 + s2 + s3 + s4) / 1.875 * max_amplitude
                }
                AudioPattern::AmplitudeSweep => {
                    let progress = t / self.duration_secs;
                    let envelope = progress; // Linear ramp up
                    (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * max_amplitude * envelope
                }
                AudioPattern::LoudClipping => {
                    // Near-maximum amplitude, will clip slightly
                    let raw =
                        (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * i16::MAX as f32 * 1.1;
                    raw.clamp(i16::MIN as f32, i16::MAX as f32)
                }
                AudioPattern::VeryQuiet => {
                    // Very low amplitude - only uses lower bits
                    (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 100.0
                }
                AudioPattern::Square(freq) => {
                    // Square wave: harsh transitions between max and min
                    let phase = (t * freq) % 1.0;
                    if phase < 0.5 {
                        max_amplitude
                    } else {
                        -max_amplitude
                    }
                }
            };

            let sample = sample_value.round() as i16;

            // Write sample for each channel
            for _ in 0..self.channels {
                writer.write_sample(sample).expect("Failed to write sample");
            }
        }
        writer.finalize().expect("Failed to finalize WAV");
    }
}

/// Convenience functions for common test scenarios
pub mod presets {
    use super::*;

    /// Standard test WAV: 1 second stereo 44.1kHz sine wave
    pub fn standard() -> TestWavConfig {
        TestWavConfig::default()
    }

    /// Silent WAV file
    pub fn silence() -> TestWavConfig {
        TestWavConfig::default().pattern(AudioPattern::Silence)
    }

    /// White noise
    pub fn noise() -> TestWavConfig {
        TestWavConfig::default().pattern(AudioPattern::WhiteNoise)
    }

    /// Mono file at 22050 Hz (common for speech)
    pub fn mono_22k() -> TestWavConfig {
        TestWavConfig::default()
            .mono()
            .sample_rate(22050)
            .pattern(AudioPattern::Sine(440.0))
    }

    /// High sample rate stereo (48kHz, common for video)
    pub fn stereo_48k() -> TestWavConfig {
        TestWavConfig::default().sample_rate(48000)
    }

    /// Very short file (100ms) - edge case for capacity
    pub fn short_100ms() -> TestWavConfig {
        TestWavConfig::default().duration(0.1)
    }

    /// Complex multi-frequency waveform
    pub fn complex() -> TestWavConfig {
        TestWavConfig::default().pattern(AudioPattern::MultiFrequency)
    }

    /// Near-clipping loud audio
    pub fn loud() -> TestWavConfig {
        TestWavConfig::default().pattern(AudioPattern::LoudClipping)
    }

    /// Very quiet audio (low amplitude)
    pub fn quiet() -> TestWavConfig {
        TestWavConfig::default().pattern(AudioPattern::VeryQuiet)
    }

    /// Square wave (harsh digital sound)
    pub fn square() -> TestWavConfig {
        TestWavConfig::default().pattern(AudioPattern::Square(440.0))
    }

    /// Amplitude sweep from silent to loud
    pub fn sweep() -> TestWavConfig {
        TestWavConfig::default().pattern(AudioPattern::AmplitudeSweep)
    }
}
