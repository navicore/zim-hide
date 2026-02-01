use crate::crypto::asymmetric::recipient_count;
use crate::format::EmbeddedData;
use crate::stego::{LsbSteganography, MetadataSteganography, StegoMethod, StegoMethodType};
use anyhow::{anyhow, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct InspectArgs {
    /// Input WAV file to inspect
    pub input: PathBuf,
}

pub fn run(args: InspectArgs) -> Result<()> {
    if !args.input.exists() {
        return Err(anyhow!("Input file does not exist: {}", args.input.display()));
    }

    // Try to extract and parse the embedded data
    let (data, method_used, capacity) = try_extract_with_info(&args.input)?;
    let embedded = EmbeddedData::from_bytes(&data)?;
    let flags = &embedded.header.flags;

    println!("VVW Embedded Data");
    println!("=================");
    println!();

    // Method info
    print!("Method: ");
    match method_used {
        StegoMethodType::Lsb => println!("LSB (Least Significant Bit)"),
        StegoMethodType::Metadata => println!("Metadata (RIFF chunk)"),
    }

    // Content type
    print!("Content: ");
    let mut content_parts = Vec::new();
    if flags.has_text {
        content_parts.push("text");
    }
    if flags.has_audio {
        content_parts.push("audio");
    }
    if content_parts.is_empty() {
        println!("none");
    } else {
        println!("{}", content_parts.join(", "));
    }

    // Payload size
    println!(
        "Payload size: {} bytes{}",
        embedded.header.payload_length,
        if flags.symmetric_encryption || flags.asymmetric_encryption {
            " (encrypted)"
        } else {
            ""
        }
    );

    // Encryption info
    print!("Encryption: ");
    if flags.symmetric_encryption {
        println!("symmetric (passphrase)");
    } else if flags.asymmetric_encryption {
        if let Some(count) = recipient_count(&embedded.payload) {
            println!("asymmetric ({} recipient{})", count, if count == 1 { "" } else { "s" });
        } else {
            println!("asymmetric");
        }
    } else {
        println!("none");
    }

    // Signature info
    print!("Signed: ");
    if flags.is_signed {
        if let Some(sig) = &embedded.signature {
            let fingerprint: String = sig.iter().take(6).map(|b| format!("{:02x}", b)).collect();
            println!("yes (sig: {}...)", fingerprint);
        } else {
            println!("yes");
        }
    } else {
        println!("no");
    }

    // Capacity info
    let total_size = embedded.total_size();
    let capacity_used = (total_size as f64 / capacity as f64) * 100.0;
    println!();
    println!("Total embedded: {} bytes", total_size);
    println!("Capacity used: {:.1}%", capacity_used);
    println!("Available: {} bytes", capacity.saturating_sub(total_size));

    Ok(())
}

fn try_extract_with_info(path: &PathBuf) -> Result<(Vec<u8>, StegoMethodType, usize)> {
    // Try metadata first
    let metadata_stego = MetadataSteganography::new();
    if let Ok(data) = metadata_stego.extract(path) {
        if data.len() >= 4 && &data[0..4] == b"VVW\x01" {
            let capacity = metadata_stego.capacity(path)?;
            return Ok((data, StegoMethodType::Metadata, capacity));
        }
    }

    // Try LSB with default options
    let lsb_stego = LsbSteganography::default();
    let data = lsb_stego.extract(path)?;

    if data.len() >= 4 && &data[0..4] == b"VVW\x01" {
        let capacity = lsb_stego.capacity(path)?;
        return Ok((data, StegoMethodType::Lsb, capacity));
    }

    Err(anyhow!("No valid VVW data found in file"))
}
