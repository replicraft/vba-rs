//! The machine-readable Win32 behavior catalog (`catalog.csv`).
//!
//! Columns: `dll,symbol,variants,family,signature,strategy,status,tests,last_error,notes`.
//! `variants` is `-`, `A`, `W` or `AW` (the exported names are `symbol` plus
//! the suffixes, or `symbol` alone for `-`). Fields never contain commas;
//! parameters in `signature` are separated by `;`.

use std::sync::OnceLock;

const CSV: &str = include_str!("../catalog.csv");

/// How a function is meant to be provided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Portable native shim written in Rust.
    NativeShim,
    /// Delegated to Wine (not implemented in this repository).
    WineFallback,
    /// Deliberately unsupported.
    Unsupported,
}

/// Implementation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// A registered provider implements it, with reference tests.
    Implemented,
    /// Nothing registered; calls fail with `Unsupported`.
    NotImplemented,
}

/// One catalog row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogRow {
    /// Lower-case library name without extension (`kernel32`).
    pub dll: String,
    /// Base symbol name.
    pub symbol: String,
    /// `-`, `A`, `W` or `AW`.
    pub variants: String,
    /// Functional family.
    pub family: String,
    /// Declare-style signature.
    pub signature: String,
    /// Preferred strategy.
    pub strategy: Strategy,
    /// Status.
    pub status: Status,
    /// Number of reference test cases per exported name, summed over variants.
    pub tests: usize,
    /// GetLastError semantics.
    pub last_error: String,
    /// Notes.
    pub notes: String,
}

impl CatalogRow {
    /// The exported symbol names this row covers.
    pub fn exports(&self) -> Vec<String> {
        match self.variants.as_str() {
            "-" => vec![self.symbol.clone()],
            v => v.chars().map(|c| format!("{}{c}", self.symbol)).collect(),
        }
    }
}

/// Parses catalog CSV text.
pub fn parse(text: &str) -> Result<Vec<CatalogRow>, String> {
    let mut rows = Vec::new();
    for (n, line) in text.lines().enumerate().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        if f.len() != 10 {
            return Err(format!(
                "line {}: expected 10 fields, found {}",
                n + 1,
                f.len()
            ));
        }
        let strategy = match f[5] {
            "native-shim" => Strategy::NativeShim,
            "wine-fallback" => Strategy::WineFallback,
            "unsupported" => Strategy::Unsupported,
            s => return Err(format!("line {}: bad strategy `{s}`", n + 1)),
        };
        let status = match f[6] {
            "IMPLEMENTED" => Status::Implemented,
            "NOT IMPLEMENTED" => Status::NotImplemented,
            s => return Err(format!("line {}: bad status `{s}`", n + 1)),
        };
        if !["-", "A", "W", "AW"].contains(&f[2]) {
            return Err(format!("line {}: bad variants `{}`", n + 1, f[2]));
        }
        let tests = f[7]
            .parse()
            .map_err(|_| format!("line {}: bad test count `{}`", n + 1, f[7]))?;
        rows.push(CatalogRow {
            dll: f[0].to_string(),
            symbol: f[1].to_string(),
            variants: f[2].to_string(),
            family: f[3].to_string(),
            signature: f[4].to_string(),
            strategy,
            status,
            tests,
            last_error: f[8].to_string(),
            notes: f[9].to_string(),
        });
    }
    Ok(rows)
}

/// The embedded catalog.
pub fn catalog() -> &'static [CatalogRow] {
    static ROWS: OnceLock<Vec<CatalogRow>> = OnceLock::new();
    ROWS.get_or_init(|| parse(CSV).expect("embedded catalog.csv is well formed"))
}

/// Finds the row exporting `symbol` from `dll` (exact, case-sensitive symbol).
pub fn lookup(dll: &str, symbol: &str) -> Option<&'static CatalogRow> {
    catalog()
        .iter()
        .find(|r| r.dll == dll && r.exports().iter().any(|e| e == symbol))
}
