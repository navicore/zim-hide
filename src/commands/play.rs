use crate::crypto::{PrivateKey, decrypt_asymmetric, decrypt_symmetric};
use crate::format::{EmbeddedData, Payload};
use crate::stego::traits::{ChannelMode, EmbedOptions};
use crate::stego::{LsbSteganography, MetadataSteganography, StegoMethod};
use anyhow::{Result, anyhow};
use clap::Args;
use std::path::PathBuf;
use std::process::Command;

#[derive(Args)]
pub struct PlayArgs {
    /// Input WAV file with embedded audio
    pub input: PathBuf,

    /// Passphrase for symmetric decryption
    #[arg(long, conflicts_with = "key")]
    pub passphrase: Option<String>,

    /// Private key file for asymmetric decryption
    #[arg(long, conflicts_with = "passphrase")]
    pub key: Option<PathBuf>,

    /// Extract to file instead of playing
    #[arg(long = "extract-to")]
    pub extract_to: Option<PathBuf>,

    /// Audio player to use
    #[arg(long, default_value = "afplay")]
    pub player: String,

    /// Bits per sample for LSB method (must match encoding)
    #[arg(long, default_value = "1")]
    pub bits: u8,

    /// Channels used for LSB method (must match encoding)
    #[arg(long, value_enum, default_value = "both")]
    pub channels: ChannelMode,
}

pub fn run(args: PlayArgs) -> Result<()> {
    if !args.input.exists() {
        return Err(anyhow!(
            "Input file does not exist: {}",
            args.input.display()
        ));
    }

    // Extract embedded data
    let data = try_extract(&args)?;
    let embedded = EmbeddedData::from_bytes(&data)?;
    let flags = &embedded.header.flags;

    if !flags.has_audio {
        return Err(anyhow!("No audio content is embedded in this file"));
    }

    // Decrypt payload
    let payload_bytes = if flags.symmetric_encryption {
        let passphrase = args
            .passphrase
            .as_ref()
            .ok_or_else(|| anyhow!("Audio is encrypted. Use --passphrase to decrypt."))?;
        decrypt_symmetric(&embedded.payload, passphrase)?
    } else if flags.asymmetric_encryption {
        let key_path = args
            .key
            .as_ref()
            .ok_or_else(|| anyhow!("Audio is encrypted. Use --key to decrypt."))?;
        let private_key = PrivateKey::load(key_path)?;
        decrypt_asymmetric(&embedded.payload, &private_key)?
    } else {
        embedded.payload.clone()
    };

    // Parse payload
    let payload = Payload::from_bytes(&payload_bytes)?;
    let audio_data = payload
        .audio
        .ok_or_else(|| anyhow!("No audio content found in payload"))?;

    // Output to file or play
    if let Some(ref output_path) = args.extract_to {
        crate::audio::decompress_audio(&audio_data, output_path)?;
        eprintln!("Extracted audio to: {}", output_path.display());
    } else {
        // Create temp file and play
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().join("extracted.wav");
        crate::audio::decompress_audio(&audio_data, &temp_path)?;

        // Find and run player
        let player = find_player(&args.player)?;
        eprintln!("Playing with: {}", player);

        let status = Command::new(&player).arg(&temp_path).status()?;

        if !status.success() {
            return Err(anyhow!("Player exited with error"));
        }
    }

    Ok(())
}

fn try_extract(args: &PlayArgs) -> Result<Vec<u8>> {
    // Try metadata first
    let metadata_stego = MetadataSteganography::new();
    if let Ok(data) = metadata_stego.extract(&args.input)
        && data.len() >= 4
        && &data[0..4] == b"VVW\x01"
    {
        return Ok(data);
    }

    // Try LSB
    let options = EmbedOptions {
        bits_per_sample: args.bits,
        channels: args.channels,
    };
    let lsb_stego = LsbSteganography::new(options);
    let data = lsb_stego.extract(&args.input)?;

    if data.len() >= 4 && &data[0..4] == b"VVW\x01" {
        return Ok(data);
    }

    Err(anyhow!("No valid VVW data found in file"))
}

fn find_player(preferred: &str) -> Result<String> {
    // Check preferred player
    if which::which(preferred).is_ok() {
        return Ok(preferred.to_string());
    }

    // Try common players
    let players = ["afplay", "mpv", "ffplay", "aplay", "paplay"];
    for player in players {
        if which::which(player).is_ok() {
            return Ok(player.to_string());
        }
    }

    Err(anyhow!(
        "No audio player found. Install mpv, ffplay, or specify --player"
    ))
}
