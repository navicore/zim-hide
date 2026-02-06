//! Zimhide - WAV steganography toolkit library.
//!
//! This module exposes the CLI types for tools like man page generation.

use clap::{CommandFactory, Parser, Subcommand};

pub mod audio;
pub mod commands;
pub mod crypto;
pub mod format;
pub mod progress;
pub mod stego;
pub mod verbosity;
pub mod wav;

pub use progress::Progress;
pub use verbosity::Verbosity;

#[derive(Parser)]
#[command(name = "zimhide")]
#[command(about = "Zim Steganography Toolkit - embed and extract encrypted content in WAV files")]
#[command(version)]
pub struct Cli {
    /// Suppress all output except errors and requested content
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Show detailed output
    #[arg(short, long, global = true, conflicts_with = "quiet")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// Get the clap Command for man page generation.
    pub fn cmd() -> clap::Command {
        <Self as CommandFactory>::command()
    }
}

#[derive(Subcommand)]
pub enum Commands {
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

    /// Generate shell completions
    Completions(commands::completions::CompletionsArgs),
}
