#![allow(dead_code)]

use anyhow::{anyhow, Result};
use hound::{SampleFormat, WavSpec};
use std::path::Path;

pub struct WavReader {
    pub spec: WavSpec,
    pub samples: Vec<i32>,
}

impl WavReader {
    pub fn open(path: &Path) -> Result<Self> {
        let reader = hound::WavReader::open(path)?;
        let spec = reader.spec();

        let samples: Vec<i32> = match spec.sample_format {
            SampleFormat::Int => reader
                .into_samples::<i32>()
                .collect::<Result<Vec<_>, _>>()?,
            SampleFormat::Float => {
                return Err(anyhow!("Float WAV files are not supported"));
            }
        };

        Ok(Self { spec, samples })
    }

    pub fn duration_seconds(&self) -> f64 {
        self.samples.len() as f64 / self.spec.channels as f64 / self.spec.sample_rate as f64
    }

    pub fn total_samples(&self) -> usize {
        self.samples.len()
    }
}
