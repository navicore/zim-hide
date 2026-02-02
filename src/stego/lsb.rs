use super::traits::{ChannelMode, EmbedOptions, StegoMethod, StegoMethodType};
use anyhow::{Result, anyhow};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::path::Path;

pub struct LsbSteganography {
    pub options: EmbedOptions,
}

impl LsbSteganography {
    pub fn new(options: EmbedOptions) -> Self {
        Self { options }
    }

    fn get_spec_and_samples(path: &Path) -> Result<(WavSpec, Vec<i32>)> {
        let reader = WavReader::open(path)?;
        let spec = reader.spec();

        let samples: Vec<i32> = match spec.sample_format {
            SampleFormat::Int => reader
                .into_samples::<i32>()
                .collect::<Result<Vec<_>, _>>()?,
            SampleFormat::Float => {
                return Err(anyhow!("Float WAV files are not supported"));
            }
        };

        Ok((spec, samples))
    }

    fn should_use_sample(&self, sample_index: usize, num_channels: u16) -> bool {
        match self.options.channels {
            ChannelMode::Both => true,
            ChannelMode::Left => sample_index.is_multiple_of(num_channels as usize),
            ChannelMode::Right => sample_index % num_channels as usize == 1,
        }
    }

    fn usable_samples(&self, total_samples: usize, num_channels: u16) -> usize {
        match self.options.channels {
            ChannelMode::Both => total_samples,
            ChannelMode::Left | ChannelMode::Right => {
                if num_channels == 1 {
                    total_samples
                } else {
                    total_samples / 2
                }
            }
        }
    }
}

impl Default for LsbSteganography {
    fn default() -> Self {
        Self::new(EmbedOptions::default())
    }
}

impl StegoMethod for LsbSteganography {
    fn embed(&self, input_path: &Path, output_path: &Path, data: &[u8]) -> Result<()> {
        let (spec, mut samples) = Self::get_spec_and_samples(input_path)?;

        let bits_per_sample = self.options.bits_per_sample;
        if !(1..=4).contains(&bits_per_sample) {
            return Err(anyhow!("bits_per_sample must be between 1 and 4"));
        }

        // Calculate capacity
        let usable = self.usable_samples(samples.len(), spec.channels);
        let capacity_bits = usable * bits_per_sample as usize;
        let capacity_bytes = capacity_bits / 8;

        // We need 4 bytes for length prefix + data
        let total_size = 4 + data.len();
        if total_size > capacity_bytes {
            return Err(anyhow!(
                "Data too large: {} bytes needed, {} bytes available",
                total_size,
                capacity_bytes
            ));
        }

        // Prepare data with length prefix
        let mut payload = Vec::with_capacity(total_size);
        payload.extend_from_slice(&(data.len() as u32).to_le_bytes());
        payload.extend_from_slice(data);

        // Create bit iterator from payload
        let mask = (1u32 << bits_per_sample) - 1;
        let clear_mask = !(mask as i32);

        let mut bit_offset = 0usize;
        let total_bits = payload.len() * 8;

        for (sample_idx, sample) in samples.iter_mut().enumerate() {
            if bit_offset >= total_bits {
                break;
            }

            if !self.should_use_sample(sample_idx, spec.channels) {
                continue;
            }

            // Extract bits_per_sample bits from payload
            let mut bits = 0u32;
            for b in 0..bits_per_sample {
                let current_byte_idx = (bit_offset + b as usize) / 8;
                let current_bit_idx = (bit_offset + b as usize) % 8;
                if current_byte_idx < payload.len() {
                    let bit = (payload[current_byte_idx] >> current_bit_idx) & 1;
                    bits |= (bit as u32) << b;
                }
            }

            // Clear LSBs and set new bits
            *sample = (*sample & clear_mask) | (bits as i32);

            bit_offset += bits_per_sample as usize;
        }

        // Write output file
        let mut writer = WavWriter::create(output_path, spec)?;
        for sample in samples {
            match spec.bits_per_sample {
                8 => writer.write_sample(sample as i8)?,
                16 => writer.write_sample(sample as i16)?,
                24 | 32 => writer.write_sample(sample)?,
                _ => {
                    return Err(anyhow!(
                        "Unsupported bits per sample: {}",
                        spec.bits_per_sample
                    ));
                }
            }
        }
        writer.finalize()?;

        Ok(())
    }

    fn extract(&self, input_path: &Path) -> Result<Vec<u8>> {
        let (spec, samples) = Self::get_spec_and_samples(input_path)?;

        let bits_per_sample = self.options.bits_per_sample;
        let mask = (1u32 << bits_per_sample) - 1;

        // First, extract length (4 bytes = 32 bits)
        let bits_for_length = 32usize;
        let samples_for_length = bits_for_length.div_ceil(bits_per_sample as usize);

        let mut length_bits = Vec::new();
        let mut samples_used = 0;

        for (sample_idx, sample) in samples.iter().enumerate() {
            if samples_used >= samples_for_length {
                break;
            }
            if !self.should_use_sample(sample_idx, spec.channels) {
                continue;
            }

            let bits = (*sample as u32) & mask;
            for b in 0..bits_per_sample {
                length_bits.push((bits >> b) & 1);
            }
            samples_used += 1;
        }

        // Convert bits to length
        let mut length_bytes = [0u8; 4];
        for (i, chunk) in length_bits.chunks(8).take(4).enumerate() {
            let mut byte = 0u8;
            for (bit_idx, &bit) in chunk.iter().enumerate() {
                byte |= (bit as u8) << bit_idx;
            }
            length_bytes[i] = byte;
        }
        let data_length = u32::from_le_bytes(length_bytes) as usize;

        // Sanity check
        let usable = self.usable_samples(samples.len(), spec.channels);
        let max_bytes = (usable * bits_per_sample as usize) / 8;
        if data_length > max_bytes || data_length > 100_000_000 {
            return Err(anyhow!(
                "Invalid data length: {} (max possible: {})",
                data_length,
                max_bytes
            ));
        }

        // Now extract the actual data
        let total_bits = (4 + data_length) * 8;
        let mut all_bits = Vec::with_capacity(total_bits);

        for (sample_idx, sample) in samples.iter().enumerate() {
            if all_bits.len() >= total_bits {
                break;
            }
            if !self.should_use_sample(sample_idx, spec.channels) {
                continue;
            }

            let bits = (*sample as u32) & mask;
            for b in 0..bits_per_sample {
                if all_bits.len() < total_bits {
                    all_bits.push((bits >> b) & 1);
                }
            }
        }

        // Skip the length prefix bits and convert remaining to bytes
        let data_bits = &all_bits[32..];
        let mut data = Vec::with_capacity(data_length);
        for chunk in data_bits.chunks(8) {
            if data.len() >= data_length {
                break;
            }
            let mut byte = 0u8;
            for (bit_idx, &bit) in chunk.iter().enumerate() {
                byte |= (bit as u8) << bit_idx;
            }
            data.push(byte);
        }

        Ok(data)
    }

    fn capacity(&self, input_path: &Path) -> Result<usize> {
        let reader = WavReader::open(input_path)?;
        let spec = reader.spec();
        let total_samples = reader.len() as usize;

        let usable = self.usable_samples(total_samples, spec.channels);
        let capacity_bits = usable * self.options.bits_per_sample as usize;
        let capacity_bytes = capacity_bits / 8;

        // Subtract 4 bytes for length prefix
        Ok(capacity_bytes.saturating_sub(4))
    }

    fn method_type(&self) -> StegoMethodType {
        StegoMethodType::Lsb
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_wav() -> NamedTempFile {
        let temp = NamedTempFile::new().unwrap();
        let spec = WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(temp.path(), spec).unwrap();

        // Write some samples
        for i in 0..44100 {
            let sample =
                ((i as f32 / 44100.0 * 440.0 * 2.0 * std::f32::consts::PI).sin() * 10000.0) as i16;
            writer.write_sample(sample).unwrap();
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        temp
    }

    #[test]
    fn test_embed_extract_roundtrip() {
        let input = create_test_wav();
        let output = NamedTempFile::new().unwrap();

        let stego = LsbSteganography::default();
        let data = b"Hello, world! This is a test message.";

        stego.embed(input.path(), output.path(), data).unwrap();
        let extracted = stego.extract(output.path()).unwrap();

        assert_eq!(data.as_slice(), extracted.as_slice());
    }

    #[test]
    fn test_capacity() {
        let input = create_test_wav();
        let stego = LsbSteganography::default();
        let capacity = stego.capacity(input.path()).unwrap();

        // 44100 samples * 2 channels * 1 bit / 8 = 11025 bytes, minus 4 for length
        assert_eq!(capacity, 11021);
    }
}
