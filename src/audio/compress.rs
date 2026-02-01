use anyhow::Result;
use std::path::Path;

pub fn compress_audio(_path: &Path) -> Result<Vec<u8>> {
    // For now, just read the raw WAV file bytes
    // In Phase 3, this will use Opus compression
    let bytes = std::fs::read(_path)?;
    Ok(bytes)
}

pub fn decompress_audio(data: &[u8], output_path: &Path) -> Result<()> {
    // For now, just write the raw bytes
    // In Phase 3, this will decompress Opus to WAV
    std::fs::write(output_path, data)?;
    Ok(())
}
