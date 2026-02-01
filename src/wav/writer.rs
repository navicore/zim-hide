#![allow(dead_code)]

use anyhow::{anyhow, Result};
use hound::WavSpec;
use std::path::Path;

pub struct WavWriter;

impl WavWriter {
    pub fn write(path: &Path, spec: WavSpec, samples: &[i32]) -> Result<()> {
        let mut writer = hound::WavWriter::create(path, spec)?;

        for sample in samples {
            match spec.bits_per_sample {
                8 => writer.write_sample(*sample as i8)?,
                16 => writer.write_sample(*sample as i16)?,
                24 | 32 => writer.write_sample(*sample)?,
                _ => return Err(anyhow!("Unsupported bits per sample: {}", spec.bits_per_sample)),
            }
        }

        writer.finalize()?;
        Ok(())
    }
}
