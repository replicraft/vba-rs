//! The `vbars` command-line executable.

use clap::Parser;

/// Tools for working with Visual Basic for Applications projects.
#[derive(Parser)]
#[command(name = "vbars", version, about)]
struct Cli {}

fn main() {
    Cli::parse();
}
