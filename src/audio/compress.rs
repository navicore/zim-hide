//! Audio compression/decompression for embedded audio.
//!
//! With the `opus-compression` feature (default): Uses Opus codec at 48kHz for ~10x compression.
//! Without the feature: Embeds raw WAV bytes (larger but no libopus dependency).

use anyhow::{Context, Result};
use std::path::Path;

// ============================================================================
// Opus compression (default)
// ============================================================================

#[cfg(feature = "opus-compression")]
mod opus_impl {
    use super::*;
    use anyhow::bail;
    use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
    use opus::{Application, Bitrate, Channels, Decoder, Encoder};

    /// Header format (8 bytes):
    /// - [0..4] Original sample rate (u32 LE) - preserved for output WAV
    /// - [4..6] Channels (u16 LE) - 1 = mono, 2 = stereo
    /// - [6..8] Frame count (u16 LE)
    const HEADER_SIZE: usize = 8;

    /// Opus frame size: 20ms at 48kHz = 960 samples per channel
    const FRAME_SIZE: usize = 960;

    /// Maximum Opus packet size (conservative)
    const MAX_PACKET_SIZE: usize = 4000;

    /// Compress a WAV file to Opus format.
    ///
    /// Returns a compact binary representation with minimal framing overhead.
    /// Input must be 48kHz, 16-bit, mono or stereo.
    pub fn compress_audio(path: &Path) -> Result<Vec<u8>> {
        let reader = WavReader::open(path)
            .with_context(|| format!("Failed to open audio file: {}", path.display()))?;

        let spec = reader.spec();

        // Validate sample rate
        if spec.sample_rate != 48000 {
            bail!(
                "Audio must be 48kHz for Opus encoding (got {}Hz). \
                Convert with: ffmpeg -i input.wav -ar 48000 output.wav",
                spec.sample_rate
            );
        }

        // Validate format
        if spec.bits_per_sample != 16 {
            bail!(
                "Audio must be 16-bit (got {}-bit). \
                Convert with: ffmpeg -i input.wav -ar 48000 -sample_fmt s16 output.wav",
                spec.bits_per_sample
            );
        }

        let channels = match spec.channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            n => bail!(
                "Unsupported channel count: {}. Only mono and stereo are supported.",
                n
            ),
        };

        // Read all samples
        let samples: Vec<i16> = reader
            .into_samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to read audio samples")?;

        // Create Opus encoder
        let mut encoder = Encoder::new(48000, channels, Application::Audio)
            .context("Failed to create Opus encoder")?;

        // Set bitrate: 64kbps for mono, 96kbps for stereo
        let bitrate = if spec.channels == 1 { 64000 } else { 96000 };
        encoder
            .set_bitrate(Bitrate::Bits(bitrate))
            .context("Failed to set encoder bitrate")?;

        // Calculate frame count
        let samples_per_frame = FRAME_SIZE * spec.channels as usize;
        let frame_count = samples.len().div_ceil(samples_per_frame);

        if frame_count > u16::MAX as usize {
            bail!("Audio too long: max {} frames supported", u16::MAX);
        }

        // Build output with header
        let mut output = Vec::new();

        // Write header
        output.extend(&spec.sample_rate.to_le_bytes());
        output.extend(&spec.channels.to_le_bytes());
        output.extend(&(frame_count as u16).to_le_bytes());

        // Encode frames
        let mut packet = [0u8; MAX_PACKET_SIZE];

        for chunk in samples.chunks(samples_per_frame) {
            // Pad last frame with zeros if needed
            let frame: Vec<i16> = if chunk.len() < samples_per_frame {
                let mut padded = chunk.to_vec();
                padded.resize(samples_per_frame, 0);
                padded
            } else {
                chunk.to_vec()
            };

            let len = encoder
                .encode(&frame, &mut packet)
                .context("Failed to encode Opus frame")?;

            // Write frame size (u16) and packet data
            output.extend(&(len as u16).to_le_bytes());
            output.extend(&packet[..len]);
        }

        Ok(output)
    }

    /// Decompress Opus data back to a WAV file.
    pub fn decompress_audio(data: &[u8], output_path: &Path) -> Result<()> {
        if data.len() < HEADER_SIZE {
            bail!("Invalid Opus data: too short for header");
        }

        // Parse header
        let sample_rate = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let channel_count = u16::from_le_bytes([data[4], data[5]]);
        let frame_count = u16::from_le_bytes([data[6], data[7]]) as usize;

        let channels = match channel_count {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            n => bail!("Invalid channel count in header: {}", n),
        };

        // Create Opus decoder
        let mut decoder = Decoder::new(48000, channels).context("Failed to create Opus decoder")?;

        // Decode all frames
        let mut samples: Vec<i16> = Vec::new();
        let mut offset = HEADER_SIZE;

        // Buffer for decoded PCM (max Opus frame is 120ms = 5760 samples/channel)
        let mut pcm = vec![0i16; 5760 * channel_count as usize];

        for _ in 0..frame_count {
            if offset + 2 > data.len() {
                bail!("Invalid Opus data: unexpected end of frame headers");
            }

            let frame_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2;

            if offset + frame_len > data.len() {
                bail!("Invalid Opus data: frame extends beyond data");
            }

            let decoded = decoder
                .decode(&data[offset..offset + frame_len], &mut pcm, false)
                .context("Failed to decode Opus frame")?;

            samples.extend(&pcm[..decoded * channel_count as usize]);
            offset += frame_len;
        }

        // Write WAV file
        let spec = WavSpec {
            channels: channel_count,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let mut writer = WavWriter::create(output_path, spec)
            .with_context(|| format!("Failed to create output WAV: {}", output_path.display()))?;

        for sample in samples {
            writer
                .write_sample(sample)
                .context("Failed to write sample")?;
        }

        writer.finalize().context("Failed to finalize WAV file")?;

        Ok(())
    }
}

// ============================================================================
// Raw WAV fallback (no opus feature)
// ============================================================================

#[cfg(not(feature = "opus-compression"))]
mod raw_impl {
    use super::*;

    /// Read raw WAV bytes (no compression).
    pub fn compress_audio(path: &Path) -> Result<Vec<u8>> {
        std::fs::read(path)
            .with_context(|| format!("Failed to read audio file: {}", path.display()))
    }

    /// Write raw WAV bytes to file.
    pub fn decompress_audio(data: &[u8], output_path: &Path) -> Result<()> {
        std::fs::write(output_path, data)
            .with_context(|| format!("Failed to write audio file: {}", output_path.display()))
    }
}

// ============================================================================
// Public API
// ============================================================================

#[cfg(feature = "opus-compression")]
pub use opus_impl::{compress_audio, decompress_audio};

#[cfg(not(feature = "opus-compression"))]
pub use raw_impl::{compress_audio, decompress_audio};

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "opus-compression"))]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavSpec, WavWriter};
    use tempfile::tempdir;

    fn create_test_wav(path: &Path, channels: u16, duration_ms: u32) {
        let spec = WavSpec {
            channels,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(path, spec).unwrap();

        let total_samples = (48000 * duration_ms / 1000) as usize;
        for i in 0..total_samples {
            let t = i as f32 / 48000.0;
            let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 16000.0;
            for _ in 0..channels {
                writer.write_sample(sample as i16).unwrap();
            }
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn test_compress_decompress_mono() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input.wav");
        let output = dir.path().join("output.wav");

        create_test_wav(&input, 1, 500);

        let compressed = compress_audio(&input).unwrap();
        decompress_audio(&compressed, &output).unwrap();

        // Verify output is a valid WAV
        let reader = hound::WavReader::open(&output).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 48000);
    }

    #[test]
    fn test_compress_decompress_stereo() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input.wav");
        let output = dir.path().join("output.wav");

        create_test_wav(&input, 2, 500);

        let compressed = compress_audio(&input).unwrap();
        decompress_audio(&compressed, &output).unwrap();

        let reader = hound::WavReader::open(&output).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 48000);
    }

    #[test]
    fn test_compression_ratio() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input.wav");

        create_test_wav(&input, 2, 1000);

        let original_size = std::fs::metadata(&input).unwrap().len();
        let compressed = compress_audio(&input).unwrap();

        // Should achieve significant compression (expect ~10x)
        let ratio = original_size as f64 / compressed.len() as f64;
        assert!(
            ratio > 5.0,
            "Expected compression ratio > 5x, got {:.1}x",
            ratio
        );
    }

    #[test]
    fn test_reject_non_48k() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input.wav");

        // Create 44.1kHz file
        let spec = WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&input, spec).unwrap();
        for _ in 0..44100 {
            writer.write_sample(0i16).unwrap();
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();

        let result = compress_audio(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("48kHz"));
    }
}

#[cfg(all(test, not(feature = "opus-compression")))]
mod tests_no_opus {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_raw_roundtrip() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input.wav");
        let output = dir.path().join("output.wav");

        // Write some test data
        let test_data = b"RIFF\x00\x00\x00\x00WAVEfmt test data";
        std::fs::write(&input, test_data).unwrap();

        let compressed = compress_audio(&input).unwrap();
        decompress_audio(&compressed, &output).unwrap();

        let result = std::fs::read(&output).unwrap();
        assert_eq!(result, test_data);
    }
}
