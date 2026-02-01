use anyhow::Result;
use clap::{Parser, Subcommand};

mod audio;
mod commands;
mod crypto;
mod format;
mod stego;
mod wav;

#[derive(Parser)]
#[command(name = "vvw")]
#[command(about = "WAV steganography toolkit for embedding and extracting encrypted content")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Embed text or audio into a WAV file
    Encode(commands::encode::EncodeArgs),

    /// Extract text content from a WAV file
    Decode(commands::decode::DecodeArgs),

    /// Extract and play embedded audio from a WAV file
    Play(commands::play::PlayArgs),

    /// Generate a keypair for encryption and signing
    Keygen(commands::keygen::KeygenArgs),

    /// Inspect embedded content metadata without decrypting
    Inspect(commands::inspect::InspectArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode(args) => commands::encode::run(args),
        Commands::Decode(args) => commands::decode::run(args),
        Commands::Play(args) => commands::play::run(args),
        Commands::Keygen(args) => commands::keygen::run(args),
        Commands::Inspect(args) => commands::inspect::run(args),
    }
}
