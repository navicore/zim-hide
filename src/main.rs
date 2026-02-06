use anyhow::Result;
use clap::Parser;
use zimhide::{Cli, Commands, Verbosity};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let verbosity = Verbosity::from_flags(cli.quiet, cli.verbose);

    match cli.command {
        Commands::Encode(args) => zimhide::commands::encode::run(args, verbosity),
        Commands::Decode(args) => zimhide::commands::decode::run(args, verbosity),
        Commands::Play(args) => zimhide::commands::play::run(args, verbosity),
        Commands::Keygen(args) => zimhide::commands::keygen::run(args, verbosity),
        Commands::Inspect(args) => zimhide::commands::inspect::run(args, verbosity),
        Commands::Completions(args) => {
            zimhide::commands::completions::run(args);
            Ok(())
        }
    }
}
