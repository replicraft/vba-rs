//! The `vbars` command-line executable.

mod inspect;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Tools for working with Visual Basic for Applications projects.
#[derive(Parser)]
#[command(name = "vbars", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the contents of a VBA project file.
    ///
    /// Accepts a raw `vbaProject.bin` compound file, or a standalone `.bas`,
    /// `.cls` or `.frm` source file.
    Inspect {
        /// The file to inspect.
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { file } => match inspect::run(&file) {
            Ok(report) => {
                print!("{report}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
    }
}
