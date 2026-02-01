use crate::crypto::{
    encrypt_asymmetric, encrypt_symmetric, sign_message, PrivateKey, PublicKey,
};
use crate::format::{EmbeddedData, Flags, Header, Payload};
use crate::stego::{LsbSteganography, MetadataSteganography, StegoMethod, StegoMethodType};
use crate::stego::traits::{ChannelMode, EmbedOptions};
use anyhow::{anyhow, Result};
use clap::Args;
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct EncodeArgs {
    /// Input WAV file
    pub input: PathBuf,

    /// Output WAV file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Text message to embed
    #[arg(long, conflicts_with = "message_file")]
    pub message: Option<String>,

    /// File containing text message to embed
    #[arg(long, conflicts_with = "message")]
    pub message_file: Option<PathBuf>,

    /// Audio file to embed
    #[arg(long)]
    pub audio: Option<PathBuf>,

    /// Passphrase for symmetric encryption
    #[arg(long, conflicts_with = "encrypt_to")]
    pub passphrase: Option<String>,

    /// Public key file(s) for asymmetric encryption (can be repeated)
    #[arg(long = "encrypt-to", conflicts_with = "passphrase")]
    pub encrypt_to: Vec<PathBuf>,

    /// Sign the message
    #[arg(long, requires = "key")]
    pub sign: bool,

    /// Private key file for signing
    #[arg(long)]
    pub key: Option<PathBuf>,

    /// Steganography method
    #[arg(long, value_enum, default_value = "lsb")]
    pub method: StegoMethodType,

    /// Bits per sample for LSB method (1-4)
    #[arg(long, default_value = "1")]
    pub bits: u8,

    /// Channels to use for LSB method
    #[arg(long, value_enum, default_value = "both")]
    pub channels: ChannelMode,
}

pub fn run(args: EncodeArgs) -> Result<()> {
    // Validate input file exists
    if !args.input.exists() {
        return Err(anyhow!("Input file does not exist: {}", args.input.display()));
    }

    // Get message content
    let text = if let Some(ref msg) = args.message {
        Some(msg.clone())
    } else if let Some(ref path) = args.message_file {
        Some(fs::read_to_string(path)?)
    } else {
        None
    };

    // Get audio content
    let audio = if let Some(ref path) = args.audio {
        Some(crate::audio::compress_audio(path)?)
    } else {
        None
    };

    if text.is_none() && audio.is_none() {
        return Err(anyhow!(
            "Nothing to embed. Use --message, --message-file, or --audio"
        ));
    }

    // Build payload
    let payload = Payload { text, audio };
    let mut payload_bytes = payload.to_bytes();

    // Encryption
    let mut flags = Flags {
        has_text: payload.text.is_some(),
        has_audio: payload.audio.is_some(),
        ..Default::default()
    };

    if let Some(ref passphrase) = args.passphrase {
        payload_bytes = encrypt_symmetric(&payload_bytes, passphrase)?;
        flags.symmetric_encryption = true;
    } else if !args.encrypt_to.is_empty() {
        let recipients: Vec<PublicKey> = args
            .encrypt_to
            .iter()
            .map(|p| PublicKey::load(p))
            .collect::<Result<Vec<_>>>()?;
        payload_bytes = encrypt_asymmetric(&payload_bytes, &recipients)?;
        flags.asymmetric_encryption = true;
    }

    // Signing
    let signature = if args.sign {
        let key_path = args.key.as_ref().ok_or_else(|| anyhow!("--key is required for signing"))?;
        let private_key = PrivateKey::load(key_path)?;
        flags.is_signed = true;
        Some(sign_message(&payload_bytes, &private_key))
    } else {
        None
    };

    // Build embedded data
    let method_id = match args.method {
        StegoMethodType::Lsb => crate::format::payload::StegoMethodId::Lsb,
        StegoMethodType::Metadata => crate::format::payload::StegoMethodId::Metadata,
    };

    let header = Header {
        flags,
        method: method_id,
        payload_length: payload_bytes.len() as u32,
    };

    let embedded = EmbeddedData {
        header,
        payload: payload_bytes,
        signature,
    };

    let data_bytes = embedded.to_bytes();

    // Choose steganography method
    let stego: Box<dyn StegoMethod> = match args.method {
        StegoMethodType::Lsb => {
            let options = EmbedOptions {
                bits_per_sample: args.bits,
                channels: args.channels,
            };
            Box::new(LsbSteganography::new(options))
        }
        StegoMethodType::Metadata => Box::new(MetadataSteganography::new()),
    };

    // Check capacity
    let capacity = stego.capacity(&args.input)?;
    if data_bytes.len() > capacity {
        return Err(anyhow!(
            "Data too large: {} bytes needed, {} bytes available. Try using --method metadata or a longer audio file.",
            data_bytes.len(),
            capacity
        ));
    }

    // Embed data
    stego.embed(&args.input, &args.output, &data_bytes)?;

    let capacity_used = (data_bytes.len() as f64 / capacity as f64) * 100.0;
    eprintln!(
        "Embedded {} bytes into {} (capacity: {:.1}%)",
        data_bytes.len(),
        args.output.display(),
        capacity_used
    );

    Ok(())
}
