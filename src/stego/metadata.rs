use super::traits::{StegoMethod, StegoMethodType};
use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const CHUNK_ID: &[u8; 4] = b"zimH";

pub struct MetadataSteganography;

impl MetadataSteganography {
    pub fn new() -> Self {
        Self
    }

    fn find_chunk(file: &mut File) -> Result<Option<(u64, u32)>> {
        file.seek(SeekFrom::Start(0))?;

        let mut header = [0u8; 12];
        file.read_exact(&mut header)?;

        if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
            return Err(anyhow!("Not a valid WAV file"));
        }

        let file_size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as u64;
        let end_pos = 8 + file_size;

        let mut pos = 12u64;
        while pos + 8 <= end_pos {
            file.seek(SeekFrom::Start(pos))?;

            let mut chunk_header = [0u8; 8];
            if file.read_exact(&mut chunk_header).is_err() {
                break;
            }

            let chunk_id = &chunk_header[0..4];
            let chunk_size = u32::from_le_bytes([
                chunk_header[4],
                chunk_header[5],
                chunk_header[6],
                chunk_header[7],
            ]);

            if chunk_id == CHUNK_ID {
                return Ok(Some((pos, chunk_size)));
            }

            // Move to next chunk (chunks are word-aligned)
            pos += 8 + chunk_size as u64;
            if chunk_size % 2 != 0 {
                pos += 1;
            }
        }

        Ok(None)
    }
}

impl Default for MetadataSteganography {
    fn default() -> Self {
        Self::new()
    }
}

impl StegoMethod for MetadataSteganography {
    fn embed(&self, input_path: &Path, output_path: &Path, data: &[u8]) -> Result<()> {
        let mut input = File::open(input_path)
            .with_context(|| format!("Failed to open input file: {}", input_path.display()))?;

        // Read entire input file
        let mut contents = Vec::new();
        input
            .read_to_end(&mut contents)
            .with_context(|| format!("Failed to read input file: {}", input_path.display()))?;

        if contents.len() < 12 || &contents[0..4] != b"RIFF" || &contents[8..12] != b"WAVE" {
            return Err(anyhow!(
                "Not a valid WAV file: {}\nExpected RIFF/WAVE headers not found",
                input_path.display()
            ));
        }

        // Remove existing zimH chunk if present
        let mut clean_contents = Vec::new();
        clean_contents.extend_from_slice(&contents[0..12]);

        let mut pos = 12;
        while pos + 8 <= contents.len() {
            let chunk_id = &contents[pos..pos + 4];
            let chunk_size = u32::from_le_bytes([
                contents[pos + 4],
                contents[pos + 5],
                contents[pos + 6],
                contents[pos + 7],
            ]) as usize;

            let chunk_total = 8 + chunk_size + (chunk_size % 2); // Include padding

            if chunk_id != CHUNK_ID {
                let end = (pos + chunk_total).min(contents.len());
                clean_contents.extend_from_slice(&contents[pos..end]);
            }

            pos += chunk_total;
        }

        // Create new zimH chunk
        let chunk_size = data.len() as u32;
        let mut chunk = Vec::with_capacity(8 + data.len() + (data.len() % 2));
        chunk.extend_from_slice(CHUNK_ID);
        chunk.extend_from_slice(&chunk_size.to_le_bytes());
        chunk.extend_from_slice(data);
        if !data.len().is_multiple_of(2) {
            chunk.push(0); // Padding byte
        }

        // Append chunk
        clean_contents.extend_from_slice(&chunk);

        // Update RIFF size
        let riff_size = (clean_contents.len() - 8) as u32;
        clean_contents[4..8].copy_from_slice(&riff_size.to_le_bytes());

        // Write output
        let mut output = File::create(output_path)
            .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
        output
            .write_all(&clean_contents)
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

        Ok(())
    }

    fn extract(&self, input_path: &Path) -> Result<Vec<u8>> {
        let mut file = File::open(input_path)
            .with_context(|| format!("Failed to open file: {}", input_path.display()))?;

        if let Some((pos, size)) = Self::find_chunk(&mut file)? {
            file.seek(SeekFrom::Start(pos + 8))?;
            let mut data = vec![0u8; size as usize];
            file.read_exact(&mut data)
                .with_context(|| format!("Failed to read zimH chunk from: {}", input_path.display()))?;
            Ok(data)
        } else {
            Err(anyhow!(
                "No zimH chunk found in: {}\nFile may not contain embedded zimhide data",
                input_path.display()
            ))
        }
    }

    fn capacity(&self, _input_path: &Path) -> Result<usize> {
        // Metadata method has effectively unlimited capacity
        // (limited only by file system and RIFF format's 4GB limit)
        Ok(u32::MAX as usize - 1024)
    }

    fn method_type(&self) -> StegoMethodType {
        StegoMethodType::Metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavSpec, WavWriter};
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

        for i in 0..1000 {
            writer.write_sample((i % 1000) as i16).unwrap();
            writer.write_sample((i % 1000) as i16).unwrap();
        }
        writer.finalize().unwrap();
        temp
    }

    #[test]
    fn test_metadata_roundtrip() {
        let input = create_test_wav();
        let output = NamedTempFile::new().unwrap();

        let stego = MetadataSteganography::new();
        let data = b"Secret metadata message!";

        stego.embed(input.path(), output.path(), data).unwrap();
        let extracted = stego.extract(output.path()).unwrap();

        assert_eq!(data.as_slice(), extracted.as_slice());
    }
}
