//! How a scoped grant matches a requested resource, per capability.
//!
//! A grant that names resources is a list of *scopes*. What a scope means
//! depends on what the resource is, because a text prefix is the wrong
//! question for most of them: `notepad` is not a prefix of a program that is
//! allowed to run, and `Scripting.` does not mean `Scripting.Evil` is
//! harmless.
//!
//! | Capability | Resource | A scope matches when |
//! | --- | --- | --- |
//! | `FileRead`, `FileWrite` | normalised path | whole path segments: `/data` covers `/data` and `/data/x`, not `/data-x`; exact on POSIX paths, Windows rules on drive and `//server/share` paths |
//! | `ProcessSpawn` | command line | the program (first word) is the scope, the scope plus `.exe`, or below the scope as a directory; arguments are never part of the match |
//! | `ComCreate` | ProgID or class | equal, or the scope ends in `.*` or `.` and the ProgID continues it with whole dot segments |
//! | `NativeDeclare` | `library!symbol` | `library!symbol` equal, or `library!*` (library names are normalised like `vba-winapi`: directory and `.dll` dropped, case folded) |
//! | `UiAutomation` | `Action` or `Action:target` | the action name is equal (a scope `SendKeys` allows `SendKeys`), or `Action:target` is equal |
//! | `Network` | `host` or `host:port` | equal, a host scope covers every port of that host, or `*.domain` covers hosts below `domain` |
//!
//! Comparison ignores case except for POSIX file paths. Every other spelling fails closed.

use crate::capability::Capability;
use crate::command::command_program;

/// Checks that `scope` is a well-formed scope for `cap`, so that a mistyped
/// grant is reported instead of silently matching nothing. [`Policy`] itself
/// accepts any text and fails closed.
///
/// [`Policy`]: crate::Policy
pub fn validate_scope(cap: Capability, scope: &str) -> Result<(), String> {
    if scope.is_empty() {
        return Err("the scope is empty".into());
    }
    match cap {
        Capability::FileRead | Capability::FileWrite | Capability::UiAutomation => Ok(()),
        Capability::ProcessSpawn => Ok(()),
        Capability::ComCreate => {
            let bare = scope
                .strip_suffix(".*")
                .or_else(|| scope.strip_suffix('.'))
                .unwrap_or(scope);
            if scope == "*" || !bare.contains('*') {
                Ok(())
            } else {
                Err("a ProgID scope is exact, `Namespace.*`, `Namespace.` or `*`".into())
            }
        }
        Capability::NativeDeclare => match split_declare(scope) {
            Some((lib, sym)) if !lib.contains('*') && (sym == "*" || !sym.contains('*')) => Ok(()),
            _ => Err("a Declare scope is `library!procedure` or `library!*`".into()),
        },
        Capability::Network => {
            let rest = scope.strip_prefix("*.").unwrap_or(scope);
            if rest.contains('*') {
                Err("a network scope is `host`, `host:port` or `*.domain`".into())
            } else {
                Ok(())
            }
        }
    }
}

/// Whether `scope` (one entry of a scoped grant) covers `resource` for `cap`.
pub(crate) fn scope_matches(cap: Capability, scope: &str, resource: &str) -> bool {
    match cap {
        Capability::FileRead | Capability::FileWrite => path_scope(scope, resource),
        Capability::ProcessSpawn => process_scope(scope, resource),
        Capability::ComCreate => progid_scope(scope, resource),
        Capability::NativeDeclare => declare_scope(scope, resource),
        Capability::UiAutomation => ui_scope(scope, resource),
        Capability::Network => network_scope(scope, resource),
    }
}

fn norm_path(p: &str) -> String {
    p.to_lowercase().replace('\\', "/")
}

fn has_parent_segment(p: &str) -> bool {
    p.split('/').any(|seg| seg == "..")
}

/// `/data` covers `/data` and `/data/x` but not `/data-secret/x`; a scope
/// that ends in a separator covers everything below it. A resource with a
/// `..` segment never matches.
fn segment_prefix(resource: &str, scope: &str) -> bool {
    if scope.is_empty() || has_parent_segment(resource) {
        return false;
    }
    match resource.strip_prefix(scope) {
        None => false,
        Some(rest) => rest.is_empty() || scope.ends_with('/') || rest.starts_with('/'),
    }
}

/// Which file system a path is written for. Only the Windows forms fold
/// case and drop trailing dots and spaces, because only those file systems do.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PathForm {
    Posix,
    Drive,
    Unc,
}

/// Splits a path into its form and segments (empty and `.` segments are
/// dropped). A drive is lower-cased and becomes the first segment.
fn parse_path(p: &str) -> (PathForm, Vec<String>) {
    let p = p.replace('\\', "/");
    let b = p.as_bytes();
    let (form, rest) = if b.len() >= 3 && b[0] == b'/' && b[1] == b'/' && b[2] != b'/' {
        (PathForm::Unc, &p[2..])
    } else if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        (PathForm::Drive, p.as_str())
    } else {
        (PathForm::Posix, p.as_str())
    };
    let mut segs: Vec<String> = Vec::new();
    for (i, s) in rest.split('/').enumerate() {
        if form == PathForm::Drive && i == 0 {
            // `c:` or `c:name`: the drive, then a relative remainder.
            segs.push(s[..2].to_ascii_lowercase());
            if s.len() > 2 {
                segs.push(s[2..].to_owned());
            }
        } else if form == PathForm::Unc && i == 0 || !s.is_empty() && s != "." {
            segs.push(s.to_owned());
        }
    }
    (form, segs)
}

/// The device names Windows resolves from any directory, with or without an
/// extension.
fn is_reserved_device(segment: &str) -> bool {
    let stem = segment
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches(' ')
        .to_ascii_lowercase();
    if matches!(
        stem.as_str(),
        "con" | "prn" | "aux" | "nul" | "conin$" | "conout$"
    ) {
        return true;
    }
    match (stem.get(..3), stem.get(3..)) {
        (Some("com" | "lpt"), Some(n)) => matches!(n.as_bytes(), [b'1'..=b'9']),
        _ => false,
    }
}

/// Whether a segment of a Windows path could mean something else than its
/// text says: a parent reference spelled with dots and spaces, an alternate
/// data stream, a device name or a short (8.3) name.
fn risky_windows_segment(segment: &str) -> bool {
    let trimmed = segment.trim_end_matches(['.', ' ']);
    if trimmed.is_empty() || segment.contains(':') || is_reserved_device(trimmed) {
        return true;
    }
    trimmed
        .as_bytes()
        .windows(2)
        .any(|w| w[0] == b'~' && w[1].is_ascii_digit())
}

/// What the Windows file system compares: case folded, trailing dots and
/// spaces dropped.
fn windows_key(segment: &str) -> String {
    segment.trim_end_matches(['.', ' ']).to_lowercase()
}

/// A scope covers the resource when its segments are a prefix of the
/// resource's. POSIX paths compare exactly; Windows paths (a drive or a UNC
/// name) compare the way Windows does, and a resource with a segment that
/// Windows could read differently never matches.
fn path_scope(scope: &str, resource: &str) -> bool {
    if scope.is_empty() {
        return false;
    }
    let ((sform, sseg), (rform, rseg)) = (parse_path(scope), parse_path(resource));
    if sform != rform || rseg.iter().any(|s| s == "..") || sseg.len() > rseg.len() {
        return false;
    }
    match rform {
        PathForm::Posix => sseg.iter().zip(&rseg).all(|(a, b)| a == b),
        PathForm::Drive | PathForm::Unc => {
            // The drive letter is not a name: it is compared as written.
            let skip = usize::from(rform == PathForm::Drive);
            let device_namespace = rform == PathForm::Unc
                && matches!(rseg.first().map(String::as_str), Some("?" | "."));
            if device_namespace || rseg.iter().skip(skip).any(|s| risky_windows_segment(s)) {
                return false;
            }
            sseg.iter().zip(&rseg).enumerate().all(|(i, (a, b))| {
                if i < skip {
                    a == b
                } else {
                    windows_key(a) == windows_key(b)
                }
            })
        }
    }
}

fn process_scope(scope: &str, command_line: &str) -> bool {
    let Some(program) = command_program(command_line) else {
        return false;
    };
    let (program, scope) = (norm_path(&program), norm_path(scope));
    if scope.is_empty() {
        return false;
    }
    // `notepad` names `notepad.exe` as well, never `notepad-x.exe`.
    let with_ext = format!("{scope}.exe");
    (program == scope || program == with_ext || segment_prefix(&program, &scope))
        && !has_parent_segment(&program)
}

fn progid_scope(scope: &str, progid: &str) -> bool {
    let (scope, progid) = (scope.to_lowercase(), progid.to_lowercase());
    if scope == "*" {
        return !progid.is_empty();
    }
    let namespace = scope
        .strip_suffix(".*")
        .or_else(|| scope.strip_suffix('.'))
        .filter(|ns| !ns.is_empty() && !ns.contains('*'));
    match namespace {
        Some(ns) => progid
            .strip_prefix(ns)
            .and_then(|rest| rest.strip_prefix('.'))
            .is_some_and(|rest| rest.split('.').all(|seg| !seg.is_empty())),
        None => !scope.contains('*') && scope == progid,
    }
}

/// Mirrors `vba_winapi::normalize_library`: drops any directory, lower-cases
/// and strips a trailing `.dll`.
pub fn normalize_library(lib: &str) -> String {
    let file = lib
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(lib)
        .to_ascii_lowercase();
    file.strip_suffix(".dll")
        .map(str::to_string)
        .unwrap_or(file)
}

/// Splits `library!symbol` at the last `!` and normalises the library.
fn split_declare(s: &str) -> Option<(String, String)> {
    let (lib, sym) = s.rsplit_once('!')?;
    let lib = normalize_library(lib);
    if lib.is_empty() || sym.is_empty() {
        return None;
    }
    Some((lib, sym.to_lowercase()))
}

fn declare_scope(scope: &str, resource: &str) -> bool {
    let (Some((sl, ss)), Some((rl, rs))) = (split_declare(scope), split_declare(resource)) else {
        return false;
    };
    sl == rl && (ss == "*" || ss == rs)
}

fn ui_scope(scope: &str, resource: &str) -> bool {
    let (scope, resource) = (scope.to_lowercase(), resource.to_lowercase());
    if scope == resource {
        return !scope.is_empty();
    }
    // A scope naming only the action allows every target of it.
    let action = resource.split(':').next().unwrap_or("");
    !scope.contains(':') && !scope.is_empty() && scope == action
}

fn network_scope(scope: &str, resource: &str) -> bool {
    let (scope, resource) = (scope.to_lowercase(), resource.to_lowercase());
    if scope.is_empty() || resource.is_empty() {
        return false;
    }
    if scope == resource {
        return true;
    }
    let host = match resource.rsplit_once(':') {
        Some((h, port)) if port.chars().all(|c| c.is_ascii_digit()) => h,
        _ => resource.as_str(),
    };
    if let Some(domain) = scope.strip_prefix("*.") {
        return !domain.is_empty()
            && host.len() > domain.len() + 1
            && host.ends_with(domain)
            && host[..host.len() - domain.len()].ends_with('.');
    }
    // `host` covers every port of `host`, not `host.evil.example`.
    !scope.contains('*') && scope == host
}

#[cfg(test)]
mod tests {
    use super::*;
    use Capability::*;

    fn yes(cap: Capability, scope: &str, resource: &str) {
        assert!(
            scope_matches(cap, scope, resource),
            "{scope} should cover {resource}"
        );
    }
    fn no(cap: Capability, scope: &str, resource: &str) {
        assert!(
            !scope_matches(cap, scope, resource),
            "{scope} must not cover {resource}"
        );
    }

    #[test]
    fn processes() {
        yes(ProcessSpawn, "notepad", "notepad");
        yes(ProcessSpawn, "notepad", "notepad.exe C:\\a.txt");
        yes(ProcessSpawn, "notepad", "NOTEPAD.EXE");
        yes(ProcessSpawn, "notepad", "\"notepad\" x");
        yes(ProcessSpawn, "C:\\Tools\\", "c:/tools/a.exe /x");
        yes(ProcessSpawn, "/usr/bin", "/usr/bin/ls -l");
        yes(ProcessSpawn, "/usr/bin/ls", "/usr/bin/ls -l");
        no(ProcessSpawn, "notepad", "notepad-x.exe");
        no(ProcessSpawn, "notepad", "notepadx");
        no(ProcessSpawn, "notepad", "notepad2.exe a");
        no(ProcessSpawn, "notepad", "evil.exe notepad");
        no(ProcessSpawn, "notepad", "evil.exe & notepad");
        no(ProcessSpawn, "notepad", "\"notepad evil.exe\"");
        no(ProcessSpawn, "notepad", "");
        no(ProcessSpawn, "notepad", "   ");
        no(ProcessSpawn, "", "notepad");
        no(ProcessSpawn, "/usr/bin", "/usr/binx/ls");
        no(ProcessSpawn, "/usr/bin", "/usr/bin/../sbin/x");
        no(ProcessSpawn, "/usr/bin/", "/usr/bin/../../tmp/x");
        no(ProcessSpawn, "/usr/bin", "/usr/bin-evil/x");
        no(ProcessSpawn, "notepad", "notepad.exe.evil");
        no(ProcessSpawn, "notepad", "notepad/../evil");
    }

    #[test]
    fn progids() {
        yes(ComCreate, "Scripting.Dictionary", "scripting.dictionary");
        yes(ComCreate, "Scripting.*", "Scripting.Dictionary");
        yes(ComCreate, "Scripting.", "Scripting.FileSystemObject");
        yes(ComCreate, "Scripting.*", "Scripting.A.B");
        yes(ComCreate, "*", "Anything.Here");
        no(ComCreate, "Scripting.Dictionary", "Scripting.Dictionary.1");
        no(ComCreate, "Scripting.Dictionary", "Scripting.DictionaryX");
        no(ComCreate, "Scripting.Dictionary", "Scripting.Evil");
        no(ComCreate, "Scripting", "Scripting.Evil");
        no(ComCreate, "Scripting", "ScriptingX.Evil");
        no(ComCreate, "Scripting.*", "ScriptingX.Evil");
        no(ComCreate, "Scripting.*", "Scripting");
        no(ComCreate, "Scripting.*", "Scripting.");
        no(ComCreate, "Scripting.*", "Scripting..Evil");
        no(ComCreate, "Scripting.*", "Scripting.A.");
        no(ComCreate, "Scripting.D*", "Scripting.Dictionary");
        no(ComCreate, "*.Dictionary", "Scripting.Dictionary");
        no(ComCreate, ".*", "x");
        no(ComCreate, ".", "x.y");
        no(ComCreate, "", "x");
        no(ComCreate, "Scripting.Dictionary", "Scripting.Dictionary ");
        no(ComCreate, "Scripting.*", " Scripting.Dictionary");
    }

    #[test]
    fn declares() {
        yes(
            NativeDeclare,
            "kernel32!GetTickCount",
            "kernel32!GetTickCount",
        );
        yes(
            NativeDeclare,
            "kernel32!GetTickCount",
            "KERNEL32.DLL!gettickcount",
        );
        yes(
            NativeDeclare,
            "kernel32.dll!GetTickCount",
            "C:\\Windows\\System32\\kernel32!GetTickCount",
        );
        yes(NativeDeclare, "kernel32!*", "kernel32!Sleep");
        yes(NativeDeclare, "Kernel32.DLL!*", "kernel32!Sleep");
        no(
            NativeDeclare,
            "kernel32!GetTickCount",
            "kernel32!GetTickCount64",
        );
        no(NativeDeclare, "kernel32!GetTickCount", "kernel32!Get");
        no(NativeDeclare, "kernel32!Get*", "kernel32!GetTickCount");
        no(NativeDeclare, "kernel32", "kernel32!Sleep");
        no(NativeDeclare, "kernel32!", "kernel32!Sleep");
        no(NativeDeclare, "kernel32!*", "kernel32x!Sleep");
        no(NativeDeclare, "kernel32!*", "xkernel32!Sleep");
        no(NativeDeclare, "user32!*", "user32!/kernel32!Sleep");
        yes(NativeDeclare, "kernel32!*", "user32!/kernel32!Sleep");
        no(NativeDeclare, "user32!*", "kernel32!user32!Sleep");
        no(NativeDeclare, "*!*", "kernel32!Sleep");
        no(NativeDeclare, "!*", "!Sleep");
        no(NativeDeclare, "kernel32!*", "kernel32!");
        no(NativeDeclare, "kernel32!*", "kernel32");
    }

    #[test]
    fn ui_actions() {
        yes(UiAutomation, "SendKeys", "SendKeys");
        yes(UiAutomation, "sendkeys", "SendKeys");
        yes(UiAutomation, "AppActivate", "AppActivate:Notepad");
        yes(UiAutomation, "AppActivate:Notepad", "AppActivate:notepad");
        no(UiAutomation, "AppActivate:Notepad", "AppActivate:Notepad++");
        no(UiAutomation, "AppActivate:Notepad", "AppActivate");
        no(UiAutomation, "SendKeys", "SendKeysX");
        no(UiAutomation, "Send", "SendKeys");
        no(UiAutomation, "SendKeys", "AppActivate:SendKeys");
        no(UiAutomation, "", "");
        no(UiAutomation, "AppActivate", "AppActivateX:t");
    }

    #[test]
    fn network() {
        yes(Network, "example.com", "example.com");
        yes(Network, "example.com", "EXAMPLE.com:8080");
        yes(Network, "example.com:443", "example.com:443");
        yes(Network, "*.example.com", "www.example.com");
        yes(Network, "*.example.com", "a.b.example.com:80");
        no(Network, "example.com", "example.com.evil.io");
        no(Network, "example.com", "evilexample.com");
        no(Network, "example.com:443", "example.com:80");
        no(Network, "*.example.com", "example.com");
        no(Network, "*.example.com", "evilexample.com");
        no(Network, "*.example.com", "x.example.com.evil.io");
        no(Network, "ex*", "example.com");
        no(Network, "", "example.com");
    }

    #[test]
    fn paths_keep_whole_segment_semantics() {
        yes(FileRead, "/data", "/data");
        yes(FileRead, "/data", "/data/x");
        yes(FileRead, "/data/", "/data/x");
        yes(FileRead, "C:\\Work\\", "c:/work/a.txt");
        no(FileRead, "/data", "/data-secret/x");
        no(FileRead, "/data/", "/data/../etc/passwd");
        no(FileRead, "", "/data");
    }

    #[test]
    fn posix_paths_are_case_sensitive() {
        yes(FileRead, "/data/", "/data/x");
        no(FileRead, "/data/", "/DATA/x");
        no(FileRead, "/data", "/Data");
        no(FileRead, "/Data/", "/data/x");
        // Only the drive form, which names a case-insensitive file system,
        // folds case.
        yes(FileRead, "c:/Work/", "C:/WORK/a.txt");
    }

    #[test]
    fn windows_names_that_the_system_rewrites_never_match_by_accident() {
        let scope = "c:/work/";
        // Trailing dots and spaces are dropped by the system, so the name
        // below is the same directory.
        yes(FileRead, scope, "c:/work./a.txt");
        yes(FileRead, scope, "c:/work /a.txt");
        yes(FileRead, scope, "c:/work/a.txt.");
        // A segment made only of dots and spaces can be a parent reference.
        no(FileRead, scope, "c:/work/.. /secret");
        no(FileRead, scope, "c:/work/.../secret");
        no(FileRead, scope, "c:/work/. ./secret");
        // Alternate data streams, device names and 8.3 short names.
        no(FileRead, scope, "c:/work/a.txt:hidden");
        no(FileRead, scope, "c:/work/a.txt::$DATA");
        no(FileRead, scope, "c:/work/NUL");
        no(FileRead, scope, "c:/work/com1.txt");
        no(FileRead, scope, "c:/work/Lpt9");
        no(FileRead, scope, "c:/work/PROGRA~1/x");
        yes(FileRead, scope, "c:/work/console.txt");
        yes(FileRead, scope, "c:/work/nullable");
    }

    #[test]
    fn unc_and_device_namespaces_need_their_own_scope() {
        yes(FileRead, "//server/share/", "//server/share/a.txt");
        yes(FileRead, "//Server/Share/", "//server/share/A.TXT");
        no(FileRead, "//server/share/", "//server/other/a.txt");
        no(FileRead, "//server/share/", "/server/share/a.txt");
        no(FileRead, "/server/share/", "//server/share/a.txt");
        no(FileRead, "c:/", "//server/share/a.txt");
        no(FileRead, "//?/", "//?/c:/secret");
        no(FileRead, "//./", "//./pipe/x");
        no(FileRead, "//?/c:/work/", "//?/c:/work/a.txt");
    }
}
