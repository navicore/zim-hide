use crate::crypto::keys::Keypair;
use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct KeygenArgs {
    /// Output base path (creates <name>.pub and <name>.priv)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

pub fn run(args: KeygenArgs) -> Result<()> {
    let keypair = Keypair::generate();

    if let Some(base_path) = args.output {
        keypair.save(&base_path)?;

        let pub_path = base_path.with_extension("pub");
        let priv_path = base_path.with_extension("priv");

        eprintln!("Generated keypair:");
        eprintln!("  Public key:  {}", pub_path.display());
        eprintln!("  Private key: {}", priv_path.display());
        eprintln!("  Fingerprint: {}", keypair.public.fingerprint());
    } else {
        // Output to stdout in a format that can be redirected
        let mut priv_bytes = Vec::with_capacity(64);
        priv_bytes.extend_from_slice(keypair.private.ed25519.as_bytes());
        priv_bytes.extend_from_slice(keypair.private.x25519.as_bytes());

        let mut pub_bytes = Vec::with_capacity(64);
        pub_bytes.extend_from_slice(keypair.public.ed25519.as_bytes());
        pub_bytes.extend_from_slice(keypair.public.x25519.as_bytes());

        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

        println!("# VVW Keypair");
        println!("# Fingerprint: {}", keypair.public.fingerprint());
        println!();
        println!("-----BEGIN VVW PRIVATE KEY-----");
        println!("{}", BASE64.encode(&priv_bytes));
        println!("-----END VVW PRIVATE KEY-----");
        println!();
        println!("-----BEGIN VVW PUBLIC KEY-----");
        println!("{}", BASE64.encode(&pub_bytes));
        println!("-----END VVW PUBLIC KEY-----");
    }

    Ok(())
}
