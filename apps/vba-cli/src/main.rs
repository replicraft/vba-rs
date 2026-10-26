//! The `vbars` command-line executable.

mod diag;
mod inspect;
mod load;
mod run;
mod tools;

use std::io::IsTerminal as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

const AFTER_HELP: &str = "\
Input files: wherever a file is expected, `-` reads standard input. Standard
input is a standard module named Stdin (or a vbaProject.bin compound file).

Exit codes:
  0  success; for lex, parse, check and fmt-check: no errors or findings
  1  findings (errors in the source) or bad input: unreadable or
     unrecognized files, unknown profile, a project that does not compile
  2  command-line usage error (and, for `run` only, an unhandled VBA
     run-time error)
  3  run: a capability that was not granted was needed
  4  run: a resource limit was exceeded or the run was cancelled
  5  run: a feature that is not implemented, or an internal error
  6  the project is compiled-only: modules without source text

Output: results go to standard output, errors to standard error prefixed
`error:`. Colours are used only on a terminal, never when NO_COLOR is set,
unless --color always is given.";

/// When diagnostics use ANSI colours.
#[derive(Clone, Copy, ValueEnum)]
enum ColorWhen {
    /// On a terminal, unless `NO_COLOR` is set or `TERM` is `dumb`.
    Auto,
    /// Always.
    Always,
    /// Never.
    Never,
}

impl ColorWhen {
    fn enabled(self) -> bool {
        match self {
            ColorWhen::Always => true,
            ColorWhen::Never => false,
            ColorWhen::Auto => {
                std::io::stdout().is_terminal()
                    && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
                    && std::env::var_os("TERM").is_none_or(|v| v != "dumb")
            }
        }
    }
}

/// Tools for working with Visual Basic for Applications projects.
#[derive(Parser)]
#[command(name = "vbars", version, about, after_long_help = AFTER_HELP)]
struct Cli {
    /// Colour the diagnostics.
    #[arg(long, value_enum, default_value = "auto", global = true)]
    color: ColorWhen,
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
        /// The file or directory to inspect, or `-` for standard input.
        file: PathBuf,
        /// Print machine-readable JSON (schema in docs/formats/vba-format.md).
        #[arg(long)]
        json: bool,
    },
    /// Print the tokens of a source file and the lexer diagnostics.
    ///
    /// Exits with 1 if there are lexical errors.
    Lex {
        /// The `.bas`, `.cls` or `.frm` file, or `-` for standard input.
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
        /// The `.bas`, `.cls` or `.frm` file, or `-` for standard input.
        file: PathBuf,
        /// Print the whole syntax tree.
        #[arg(long, conflicts_with = "json")]
        tree: bool,
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Language profile id (see `vbars profiles`).
        #[arg(long)]
        profile: Option<String>,
    },
    /// Analyse source files (or a vbaProject.bin) as one project.
    ///
    /// Prints syntax and semantic diagnostics; exits with 1 on errors.
    Check {
        /// The files of the project (`-` is standard input).
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Language profile id (see `vbars profiles`).
        #[arg(long)]
        profile: Option<String>,
        /// Report the spec-strict diagnostics (rules from the language
        /// specification that real VBA is not known to enforce) as errors.
        #[arg(long)]
        strict_spec: bool,
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Check that a source file survives the syntax tree unchanged and
    /// follows basic whitespace conventions.
    ///
    /// Exits with 1 if there are findings.
    FmtCheck {
        /// The `.bas`, `.cls` or `.frm` file, or `-` for standard input.
        file: PathBuf,
        /// Language profile id (see `vbars profiles`).
        #[arg(long)]
        profile: Option<String>,
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// List the language profiles.
    Profiles {
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Print a shell completion script to standard output.
    Completions {
        /// The shell to generate for.
        shell: Shell,
    },
    /// Print the manual page (roff) to standard output.
    Man,
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
        /// Source files (`.bas`, `.cls`, `.frm`) or one vbaProject.bin (`-` is
        /// standard input).
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
        /// Wall-clock seconds the run (and the shutdown) may take (default 60;
        /// 0 removes the limit). Checked every 1024 steps and inside long
        /// library calls, so it is not exact; a stuck host call is not
        /// interrupted.
        #[arg(long)]
        timeout: Option<u64>,
        /// Remove the default step, memory and time limits (only for code you
        /// trust; --max-steps, --max-memory and --timeout still apply when
        /// given).
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
        /// Report the spec-strict diagnostics (rules from the language
        /// specification that real VBA is not known to enforce) as errors
        /// that block loading; by default they are warnings.
        #[arg(long)]
        strict_spec: bool,
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

/// Stack of the thread that compiles and runs a program. The interpreter
/// budgets its own native nesting (`VmConfig::max_native_stack_bytes`); the
/// main thread of some systems (Windows: 1 MiB) is smaller than that budget.
const RUN_STACK_BYTES: usize = 64 << 20;

/// Runs `f` on a thread with [`RUN_STACK_BYTES`] of stack.
fn on_large_stack(f: impl FnOnce() -> u8 + Send + 'static) -> u8 {
    let spawned = std::thread::Builder::new()
        .stack_size(RUN_STACK_BYTES)
        .spawn(f);
    match spawned.map(|h| h.join()) {
        Ok(Ok(code)) => code,
        _ => {
            eprintln!("error: the run ended abnormally");
            5
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    diag::set_color(cli.color.enabled());
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
            json,
            profile,
        } => finish(
            tools::profile_arg(profile.as_deref())
                .and_then(|p| tools::parse_file(&file, &p, tree, json)),
        ),
        Command::Check {
            files,
            profile,
            strict_spec,
            json,
        } => finish(
            tools::profile_arg(profile.as_deref())
                .and_then(|p| tools::check_files(&files, &p, strict_spec, json)),
        ),
        Command::FmtCheck {
            file,
            profile,
            json,
        } => finish(
            tools::profile_arg(profile.as_deref()).and_then(|p| tools::fmt_check(&file, &p, json)),
        ),
        Command::Profiles { json } => {
            print!("{}", tools::list_profiles(json));
            ExitCode::SUCCESS
        }
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "vbars", &mut std::io::stdout());
            ExitCode::SUCCESS
        }
        Command::Man => {
            let mut out = Vec::new();
            match clap_mangen::Man::new(Cli::command()).render(&mut out) {
                Ok(()) => {
                    use std::io::Write as _;
                    let _ = std::io::stdout().write_all(&out);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::FAILURE
                }
            }
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
            timeout,
            unlimited,
            system_env,
            allow_shell,
            command,
            allow_partial,
            strict_spec,
        } => {
            let args = run::RunArgs {
                files,
                entry,
                args,
                grants,
                profile,
                memfs,
                max_steps,
                max_memory,
                max_call_depth,
                timeout,
                unlimited,
                system_env,
                allow_shell,
                command,
                allow_partial,
                strict_spec,
            };
            ExitCode::from(on_large_stack(move || run::run(args)))
        }
    }
}
