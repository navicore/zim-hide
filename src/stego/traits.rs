use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum StegoMethodType {
    /// LSB (Least Significant Bit) embedding
    #[default]
    Lsb,
    /// RIFF metadata chunk embedding
    Metadata,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum ChannelMode {
    /// Embed in left channel only
    Left,
    /// Embed in right channel only
    Right,
    /// Embed in both channels
    #[default]
    Both,
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
