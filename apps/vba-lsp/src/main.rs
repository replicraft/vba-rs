//! `vba-lsp`: a language server for VBA over stdio.
//!
//! A thin protocol layer over `vba-lsp-core`: JSON-RPC framing and the
//! request loop live here, every language feature in the core library.

mod config;
mod convert;
mod server;

use lsp_server::Connection;

fn main() {
    let (conn, _io_threads) = Connection::stdio();
    let code = match server::Server::initialize(conn).and_then(server::Server::run) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("vba-lsp: {e}");
            1
        }
    };
    // The IO threads are deliberately not joined: the reader blocks on stdin
    // until the client closes it, which some clients do only after the
    // process has gone. By the time `exit` arrives the client has received
    // every response it was waiting for.
    std::process::exit(code);
}
