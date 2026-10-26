//! The `vbars` command-line executable.

mod diag;
mod inspect;
mod load;
mod run;
mod tools;

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
    /// Accepts a raw `vbaProject.bin` compound file, a directory of exported
    /// streams (files named by their stream path, `/` as a subdirectory:
    /// `VBA/dir`, `VBA/_VBA_PROJECT`, `PROJECT`, ...), or a standalone
    /// `.bas`, `.cls` or `.frm` source file.
    Inspect {
        /// The file or directory to inspect.
        file: PathBuf,
        /// Print machine-readable JSON (schema in docs/formats/vba-format.md).
        #[arg(long)]
        json: bool,
    },
    /// Print the tokens of a source file and the lexer diagnostics.
    ///
    /// Exits with 1 if there are lexical errors.
    Lex {
        /// The `.bas`, `.cls` or `.frm` file.
        file: PathBuf,
        /// Also print whitespace, comments and line breaks.
        #[arg(long)]
        trivia: bool,
        /// Language profile id (see `vbars profiles`).
        #[arg(long)]
        profile: Option<String>,
    },
    /// Parse a source file and print a summary and the parser diagnostics.
    ///
    /// Exits with 1 if there are syntax errors.
    Parse {
        /// The `.bas`, `.cls` or `.frm` file.
        file: PathBuf,
        /// Print the whole syntax tree.
        #[arg(long)]
        tree: bool,
        /// Language profile id (see `vbars profiles`).
        #[arg(long)]
        profile: Option<String>,
    },
    /// Analyse source files (or a vbaProject.bin) as one project.
    ///
    /// Prints syntax and semantic diagnostics; exits with 1 on errors.
    Check {
        /// The files of the project.
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Language profile id (see `vbars profiles`).
        #[arg(long)]
        profile: Option<String>,
    },
    /// Check that a source file survives the syntax tree unchanged and
    /// follows basic whitespace conventions.
    ///
    /// Exits with 1 if there are findings.
    FmtCheck {
        /// The `.bas`, `.cls` or `.frm` file.
        file: PathBuf,
        /// Language profile id (see `vbars profiles`).
        #[arg(long)]
        profile: Option<String>,
    },
    /// List the language profiles.
    Profiles,
    /// Run a procedure.
    ///
    /// Running is explicit opt-in and every capability is denied unless a
    /// `--grant` allows it: with no grants the program can compute and use
    /// `Debug.Print` (written to standard output) but cannot read or write
    /// files, start programs, create objects, call native code or show UI.
    ///
    /// Exit codes: 0 success; 1 bad input or the project does not compile;
    /// 2 unhandled run-time error; 3 permission denied; 4 resource limit or
    /// cancelled; 5 unsupported feature or internal error; 6 the project is
    /// compiled-only (modules without source).
    Run {
        /// Source files (`.bas`, `.cls`, `.frm`) or one vbaProject.bin.
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// The procedure to run: `Name` or `Module.Name`.
        #[arg(long, default_value = "Main")]
        entry: String,
        /// An argument for the procedure (repeatable; integers and numbers
        /// are passed as such, anything else as a string).
        #[arg(long = "arg")]
        args: Vec<String>,
        /// Allow a capability: `file-read`, `file-write`, `process-spawn`,
        /// `network`, `com-create`, `native-declare` or `ui-automation`,
        /// optionally scoped (repeatable). A scope is a directory or file for
        /// the file capabilities (`file-read=/data`), a program for
        /// `process-spawn=notepad`, a ProgID for `com-create=Scripting.Dictionary`
        /// or a namespace `com-create=Scripting.*`, `library!procedure` or
        /// `library!*` for `native-declare=kernel32!GetTickCount`, an action
        /// for `ui-automation=SendKeys`, a host for `network=example.com` or
        /// `*.example.com`. Without a scope the capability is granted for
        /// everything.
        #[arg(long = "grant", value_name = "CAPABILITY[=PATH]")]
        grants: Vec<String>,
        /// Language profile id (see `vbars profiles`).
        #[arg(long)]
        profile: Option<String>,
        /// Use an empty in-memory file system instead of the real one.
        #[arg(long)]
        memfs: bool,
        /// Stop after this many interpreter steps (default 100000000; a
        /// step is one instruction, and building large values or scanning
        /// long strings costs extra steps).
        #[arg(long)]
        max_steps: Option<u64>,
        /// Stop when the program holds more than this many bytes of strings,
        /// arrays, collections, dictionaries, in-memory files and output
        /// (default 268435456).
        #[arg(long)]
        max_memory: Option<u64>,
        /// Largest call depth (default 1000).
        #[arg(long)]
        max_call_depth: Option<u32>,
        /// Remove the default step and memory limits (only for code you
        /// trust; --max-steps and --max-memory still apply when given).
        #[arg(long)]
        unlimited: bool,
        /// Let `Now`, `Timer`, `Environ` read the real clock (UTC) and
        /// environment instead of the fixed 2000-01-01 clock and an empty
        /// environment.
        #[arg(long)]
        system_env: bool,
        /// Install the operating-system launcher for `Shell` (still needs
        /// `--grant process-spawn`).
        #[arg(long)]
        allow_shell: bool,
        /// The text `Command` returns.
        #[arg(long)]
        command: Option<String>,
        /// Accept a project in which only some modules have source; the
        /// modules without source are left out and calling into them is an
        /// error (exit 6). A fully compiled-only project is always refused.
        #[arg(long)]
        allow_partial: bool,
    },
}

fn finish(r: Result<tools::Report, tools::ToolError>) -> ExitCode {
    match r {
        Ok(rep) => {
            print!("{}", rep.text);
            if rep.clean {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            if matches!(
                e,
                tools::ToolError::Load(load::LoadError::CompiledOnly { .. })
            ) {
                ExitCode::from(run::exit::COMPILED_ONLY)
            } else {
                ExitCode::from(1)
            }
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { file, json } => match inspect::run(&file, json) {
            Ok(report) => {
                print!("{report}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Command::Lex {
            file,
            trivia,
            profile,
        } => finish(
            tools::profile_arg(profile.as_deref()).and_then(|p| tools::lex_file(&file, &p, trivia)),
        ),
        Command::Parse {
            file,
            tree,
            profile,
        } => finish(
            tools::profile_arg(profile.as_deref()).and_then(|p| tools::parse_file(&file, &p, tree)),
        ),
        Command::Check { files, profile } => finish(
            tools::profile_arg(profile.as_deref()).and_then(|p| tools::check_files(&files, &p)),
        ),
        Command::FmtCheck { file, profile } => {
            finish(tools::profile_arg(profile.as_deref()).and_then(|p| tools::fmt_check(&file, &p)))
        }
        Command::Profiles => {
            print!("{}", tools::list_profiles());
            ExitCode::SUCCESS
        }
        Command::Run {
            files,
            entry,
            args,
            grants,
            profile,
            memfs,
            max_steps,
            max_memory,
            max_call_depth,
            unlimited,
            system_env,
            allow_shell,
            command,
            allow_partial,
        } => ExitCode::from(run::run(run::RunArgs {
            files,
            entry,
            args,
            grants,
            profile,
            memfs,
            max_steps,
            max_memory,
            max_call_depth,
            unlimited,
            system_env,
            allow_shell,
            command,
            allow_partial,
        })),
    }
}
