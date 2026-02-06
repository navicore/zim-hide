//! Generate man pages for zimhide.
//!
//! Run with: cargo run --bin gen-man
//! Man pages are written to the `man/` directory.

use clap_mangen::Man;
use std::fs;
use std::path::Path;

fn main() -> std::io::Result<()> {
    let out_dir = Path::new("man");
    fs::create_dir_all(out_dir)?;

    let cmd = zimhide::Cli::cmd();

    // Generate main man page
    let man = Man::new(cmd.clone());
    let mut buffer = Vec::new();
    man.render(&mut buffer)?;
    fs::write(out_dir.join("zimhide.1"), buffer)?;
    println!("Generated: man/zimhide.1");

    // Generate subcommand man pages
    for subcommand in cmd.get_subcommands() {
        let name = subcommand.get_name();
        let man = Man::new(subcommand.clone());
        let mut buffer = Vec::new();
        man.render(&mut buffer)?;
        let filename = format!("zimhide-{}.1", name);
        fs::write(out_dir.join(&filename), buffer)?;
        println!("Generated: man/{}", filename);
    }

    println!("\nInstall with: sudo cp man/*.1 /usr/local/share/man/man1/");
    Ok(())
}
