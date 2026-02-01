use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum StegoMethodType {
    /// LSB (Least Significant Bit) embedding
    Lsb,
    /// RIFF metadata chunk embedding
    Metadata,
}

impl Default for StegoMethodType {
    fn default() -> Self {
        Self::Lsb
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ChannelMode {
    /// Embed in left channel only
    Left,
    /// Embed in right channel only
    Right,
    /// Embed in both channels
    Both,
}

impl Default for ChannelMode {
    fn default() -> Self {
        Self::Both
    }
}

pub struct EmbedOptions {
    pub bits_per_sample: u8,
    pub channels: ChannelMode,
}

impl Default for EmbedOptions {
    fn default() -> Self {
        Self {
            bits_per_sample: 1,
            channels: ChannelMode::Both,
        }
    }
}

pub trait StegoMethod {
    fn embed(&self, input_path: &Path, output_path: &Path, data: &[u8]) -> Result<()>;

    fn extract(&self, input_path: &Path) -> Result<Vec<u8>>;

    fn capacity(&self, input_path: &Path) -> Result<usize>;

    #[allow(dead_code)]
    fn method_type(&self) -> StegoMethodType;
}
