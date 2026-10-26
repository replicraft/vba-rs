//! `vba-wine-host`: the helper process of the vba-rs Wine bridge.
//!
//! Intended to be built for a Windows target and run under Wine by
//! `vba-wine`. It reads length-prefixed requests on stdin, answers on stdout
//! (nothing else is ever written to stdout) and is NOT a VBA interpreter, NOT
//! a sandbox and NOT meant to be run by users. On non-Windows hosts, or with
//! `--protocol-only`, it serves the protocol but answers every library
//! operation with an explicit `Unsupported` error.
//!
//! Test-only switches (undocumented for users): `--advertise-version=N` lies
//! about the protocol version in Hello; `--inject=exit|hang|garbage` makes the
//! helper misbehave on the first request after Hello.

mod backend;
#[cfg(windows)]
mod native;

use std::io::{self, Write};
use vba_wine::helper::Server;
use vba_wine::protocol::{read_frame, write_frame};

#[derive(Clone, Copy, PartialEq)]
enum Inject {
    None,
    Exit,
    Hang,
    Garbage,
}

struct Opts {
    protocol_only: bool,
    advertise: Option<u16>,
    inject: Inject,
}

fn parse() -> Result<Opts, String> {
    let mut o = Opts {
        protocol_only: !cfg!(windows),
        advertise: None,
        inject: Inject::None,
    };
    for a in std::env::args().skip(1) {
        match a.as_str() {
            "--protocol-only" => o.protocol_only = true,
            "--inject=exit" => o.inject = Inject::Exit,
            "--inject=hang" => o.inject = Inject::Hang,
            "--inject=garbage" => o.inject = Inject::Garbage,
            s if s.starts_with("--advertise-version=") => {
                o.advertise = Some(
                    s["--advertise-version=".len()..]
                        .parse()
                        .map_err(|_| format!("bad argument {s}"))?,
                );
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(o)
}

fn main() {
    let opts = match parse() {
        Ok(o) => o,
        Err(e) => {
            // stderr is discarded by the client; this is for manual runs.
            eprintln!("vba-wine-host: {e}");
            std::process::exit(2);
        }
    };
    let backend = backend::make(opts.protocol_only, opts.advertise);
    let mut server = Server::new(backend);
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut r = stdin.lock();
    let mut w = stdout.lock();
    let mut greeted = false;
    loop {
        let body = match read_frame(&mut r) {
            Ok(Some(b)) => b,
            Ok(None) => return,
            Err(_) => std::process::exit(3),
        };
        if greeted {
            match opts.inject {
                Inject::Exit => std::process::exit(7),
                Inject::Hang => loop {
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                },
                Inject::Garbage => {
                    let _ = write_frame(&mut w, &[0xEE, 1, 2, 3]);
                    let _ = w.flush();
                    continue;
                }
                Inject::None => {}
            }
        }
        let (out, bad) = server.handle_frame(&body);
        if write_frame(&mut w, &out).is_err() {
            std::process::exit(4);
        }
        greeted = true;
        if bad.is_some() {
            std::process::exit(5);
        }
        if server.is_stopped() {
            return;
        }
    }
}
