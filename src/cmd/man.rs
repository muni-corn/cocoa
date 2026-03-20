//! Man page generator for cocoa.
//!
//! Generates ROFF man page files for all cocoa commands into an output
//! directory. Run with:
//!
//! ```
//! cargo run --bin gen-man -- [OUTPUT_DIR]
//! ```
//!
//! `OUTPUT_DIR` defaults to `man/` relative to the working directory.

use std::path::PathBuf;

use clap::CommandFactory;
use cocoa::cli::Cli;

pub fn handle_man() -> std::io::Result<()> {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("man"));

    std::fs::create_dir_all(&out_dir)?;

    let cmd = Cli::command();
    clap_mangen::generate_to(cmd, &out_dir)?;

    eprintln!("man pages written to: {}", out_dir.display());

    Ok(())
}
