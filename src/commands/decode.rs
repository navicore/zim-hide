use crate::crypto::{
    decrypt_asymmetric, decrypt_symmetric, verify_signature, PrivateKey, PublicKey,
};
use crate::format::{EmbeddedData, Payload};
use crate::stego::{LsbSteganography, MetadataSteganography, StegoMethod};
use crate::stego::traits::{ChannelMode, EmbedOptions};
use anyhow::{anyhow, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct DecodeArgs {
    /// Input WAV file with embedded data
    pub input: PathBuf,

    /// Passphrase for symmetric decryption
    #[arg(long, conflicts_with = "key")]
    pub passphrase: Option<String>,

    /// Private key file for asymmetric decryption
    #[arg(long, conflicts_with = "passphrase")]
    pub key: Option<PathBuf>,

    /// Public key file to verify signature
    #[arg(long)]
    pub verify: Option<PathBuf>,

    /// Bits per sample for LSB method (must match encoding)
    #[arg(long, default_value = "1")]
    pub bits: u8,

    /// Channels used for LSB method (must match encoding)
    #[arg(long, value_enum, default_value = "both")]
    pub channels: ChannelMode,
}

pub fn run(args: DecodeArgs) -> Result<()> {
    if !args.input.exists() {
        return Err(anyhow!("Input file does not exist: {}", args.input.display()));
    }

    // Try LSB first, then metadata
    let data = try_extract(&args)?;

    // Parse embedded data
    let embedded = EmbeddedData::from_bytes(&data)?;
    let flags = &embedded.header.flags;

    // Verify signature if requested
    if let Some(ref verify_path) = args.verify {
        if !flags.is_signed {
            return Err(anyhow!("Message is not signed"));
        }
        let public_key = PublicKey::load(verify_path)?;
        let signature = embedded
            .signature
            .as_ref()
            .ok_or_else(|| anyhow!("No signature found"))?;
        verify_signature(&embedded.payload, signature, &public_key)?;
        eprintln!("Signature verified successfully");
    } else if flags.is_signed {
        eprintln!("Note: Message is signed. Use --verify to verify the signature.");
    }

    // Decrypt payload
    let payload_bytes = if flags.symmetric_encryption {
        let passphrase = args
            .passphrase
            .as_ref()
            .ok_or_else(|| anyhow!("Message is encrypted. Use --passphrase to decrypt."))?;
        decrypt_symmetric(&embedded.payload, passphrase)?
    } else if flags.asymmetric_encryption {
        let key_path = args
            .key
            .as_ref()
            .ok_or_else(|| anyhow!("Message is encrypted. Use --key to decrypt."))?;
        let private_key = PrivateKey::load(key_path)?;
        decrypt_asymmetric(&embedded.payload, &private_key)?
    } else {
        embedded.payload.clone()
    };

    // Parse payload
    let payload = Payload::from_bytes(&payload_bytes)?;

    if let Some(text) = payload.text {
        println!("{}", text);
    }

    if payload.audio.is_some() {
        eprintln!("Note: Audio content is embedded. Use 'vvw play' to extract/play it.");
    }

    Ok(())
}

fn try_extract(args: &DecodeArgs) -> Result<Vec<u8>> {
    // Try metadata first (quick check)
    let metadata_stego = MetadataSteganography::new();
    if let Ok(data) = metadata_stego.extract(&args.input) {
        // Verify it's valid VVW data
        if data.len() >= 4 && &data[0..4] == b"VVW\x01" {
            return Ok(data);
        }
    }

    // Try LSB
    let options = EmbedOptions {
        bits_per_sample: args.bits,
        channels: args.channels,
    };
    let lsb_stego = LsbSteganography::new(options);
    let data = lsb_stego.extract(&args.input)?;

    // Verify it's valid VVW data
    if data.len() >= 4 && &data[0..4] == b"VVW\x01" {
        return Ok(data);
    }

    Err(anyhow!(
        "No valid VVW data found in file. The file may not contain embedded data, or you may need to specify --bits and --channels to match the encoding."
    ))
}
