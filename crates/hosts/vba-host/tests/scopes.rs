//! Scoped grants through the public `Policy` and `Host` API: every spelling
//! that used to slip through a text prefix is refused.

use vba_host::{Capability, HeadlessHost, Host, Policy, scope};

fn allowed(p: &Policy, cap: Capability, r: &str) -> bool {
    p.check(cap, r).is_ok()
}

#[test]
fn a_process_grant_names_the_program() {
    let p = Policy::deny_all().grant_prefixes(Capability::ProcessSpawn, ["notepad"]);
    let c = Capability::ProcessSpawn;
    assert!(allowed(&p, c, "notepad"));
    assert!(allowed(&p, c, "notepad.exe"));
    assert!(allowed(&p, c, "NotePad.EXE C:\\file.txt"));
    assert!(allowed(&p, c, "\"notepad\" file.txt"));
    for bad in [
        "notepad-x.exe",
        "notepadx",
        "notepad2.exe",
        "notepad.exe.exe",
        "evil.exe notepad",
        "evil.exe \"notepad\"",
        "\"notepad evil\"",
        "./notepad",
        "C:\\evil\\notepad.exe",
        "notepad\u{0}.exe",
        "",
        "   ",
        "\"\"",
    ] {
        assert!(!allowed(&p, c, bad), "{bad:?}");
    }
    // A directory grant covers programs below it, by whole segments.
    let p = Policy::deny_all().grant_prefixes(c, ["/opt/tools/", "C:\\Tools"]);
    assert!(allowed(&p, c, "/opt/tools/a --x"));
    assert!(allowed(&p, c, "c:\\tools\\a.exe /x"));
    for bad in [
        "/opt/tools-evil/a",
        "/opt/toolsx",
        "/opt/tools/../bin/sh",
        "C:\\Toolsmith\\a.exe",
        "C:\\Tools\\..\\Windows\\cmd.exe",
        "/opt/other",
    ] {
        assert!(!allowed(&p, c, bad), "{bad:?}");
    }
}

#[test]
fn a_progid_grant_is_exact_or_a_namespace() {
    let c = Capability::ComCreate;
    let exact = Policy::deny_all().grant_prefixes(c, ["Scripting.Dictionary"]);
    assert!(allowed(&exact, c, "Scripting.Dictionary"));
    assert!(allowed(&exact, c, "scripting.dictionary"));
    for bad in [
        "Scripting.Dictionary.1",
        "Scripting.DictionaryEvil",
        "Scripting.Evil",
        "Scripting",
        "Scripting.Dictionary ",
        " Scripting.Dictionary",
        "Scripting.Dictionary\0",
        "xScripting.Dictionary",
    ] {
        assert!(!allowed(&exact, c, bad), "{bad:?}");
    }
    let ns = Policy::deny_all().grant_prefixes(c, ["Scripting.*"]);
    assert!(allowed(&ns, c, "Scripting.FileSystemObject"));
    assert!(allowed(&ns, c, "SCRIPTING.Dictionary"));
    for bad in [
        "Scripting",
        "Scripting.",
        "Scripting..X",
        "ScriptingEvil.X",
        "Scripting2.Dictionary",
        "WScript.Shell",
        "X.Scripting.Dictionary",
    ] {
        assert!(!allowed(&ns, c, bad), "{bad:?}");
    }
    // A bare namespace name is exact, not a prefix.
    let bare = Policy::deny_all().grant_prefixes(c, ["Scripting"]);
    assert!(!allowed(&bare, c, "Scripting.Evil"));
    assert!(!allowed(&bare, c, "ScriptingX.Evil"));
    // Wildcards in the middle or at the front match nothing.
    for odd in ["Scripting.D*", "*.Dictionary", "Scr*", "*Scripting.*"] {
        let p = Policy::deny_all().grant_prefixes(c, [odd]);
        assert!(!allowed(&p, c, "Scripting.Dictionary"), "{odd}");
    }
    assert!(allowed(
        &Policy::deny_all().grant_prefixes(c, ["*"]),
        c,
        "Any.Thing"
    ));
}

#[test]
fn a_declare_grant_is_a_procedure_or_a_whole_library() {
    let c = Capability::NativeDeclare;
    let one = Policy::deny_all().grant_prefixes(c, ["kernel32!GetTickCount"]);
    assert!(allowed(&one, c, "kernel32!GetTickCount"));
    assert!(allowed(&one, c, "KERNEL32.DLL!gettickcount"));
    assert!(allowed(
        &one,
        c,
        "C:\\Windows\\System32\\kernel32.dll!GetTickCount"
    ));
    for bad in [
        "kernel32!GetTickCount64",
        "kernel32!GetTick",
        "kernel32!Sleep",
        "kernel32",
        "kernel32x!GetTickCount",
        "xkernel32!GetTickCount",
        "user32!GetTickCount",
        "kernel32!GetTickCount!",
        "kernel32.dll.dll!GetTickCount",
    ] {
        assert!(!allowed(&one, c, bad), "{bad:?}");
    }
    let lib = Policy::deny_all().grant_prefixes(c, ["Kernel32.DLL!*"]);
    assert!(allowed(&lib, c, "kernel32!Sleep"));
    assert!(allowed(&lib, c, "d:\\x\\KERNEL32!Sleep"));
    // Smuggling another library through the name: the resource is judged by
    // the library the provider will resolve.
    assert!(!allowed(&lib, c, "kernel32!/user32!GetDC"));
    let user = Policy::deny_all().grant_prefixes(c, ["user32!*"]);
    assert!(!allowed(&user, c, "user32!/kernel32!Sleep"));
    assert!(!allowed(&user, c, "kernel32!user32!Sleep"));
    assert!(allowed(&user, c, "kernel32!/user32!GetDC"));
    // Spellings that are not `library!procedure` match nothing.
    for odd in [
        "kernel32",
        "kernel32!",
        "!Sleep",
        "*!*",
        "kernel32!Get*",
        "*",
    ] {
        let p = Policy::deny_all().grant_prefixes(c, [odd]);
        assert!(!allowed(&p, c, "kernel32!GetTickCount"), "{odd}");
    }
}

#[test]
fn ui_automation_grants_name_actions() {
    let c = Capability::UiAutomation;
    let keys = Policy::deny_all().grant_prefixes(c, ["SendKeys"]);
    assert!(allowed(&keys, c, "SendKeys"));
    assert!(!allowed(&keys, c, "AppActivate:Notepad"));
    assert!(!allowed(&keys, c, "SendKeysEvil"));
    assert!(!allowed(&keys, c, "Send"));
    let one = Policy::deny_all().grant_prefixes(c, ["AppActivate:Notepad"]);
    assert!(allowed(&one, c, "AppActivate:notepad"));
    assert!(!allowed(&one, c, "AppActivate:Notepad++"));
    assert!(!allowed(&one, c, "AppActivate:Calculator"));
    assert!(!allowed(&one, c, "AppActivate"));
    let any = Policy::deny_all().grant_prefixes(c, ["AppActivate"]);
    assert!(allowed(&any, c, "AppActivate:Calculator"));
    assert!(!allowed(&any, c, "SendKeys"));
}

#[test]
fn app_activate_is_checked_as_action_and_title() {
    let host = HeadlessHost::new()
        .with_policy(Policy::deny_all().grant_prefixes(Capability::UiAutomation, ["SendKeys"]));
    let err = host.app_activate("Notepad", false).unwrap_err();
    assert_eq!(err.number, 70);
    let host = HeadlessHost::new().with_policy(
        Policy::deny_all().grant_prefixes(Capability::UiAutomation, ["AppActivate:Notepad"]),
    );
    // The policy lets the title through; the headless host has no UI to drive.
    let err = host.app_activate("Notepad", false).unwrap_err();
    assert!(
        format!("{err:?}").contains("no UI automation provider"),
        "{err:?}"
    );
    let err = host.app_activate("Calculator", false).unwrap_err();
    assert!(format!("{err:?}").contains("denied"), "{err:?}");
}

#[test]
fn network_grants_name_hosts() {
    let c = Capability::Network;
    let p = Policy::deny_all().grant_prefixes(c, ["example.com", "*.cdn.example.net"]);
    assert!(allowed(&p, c, "example.com"));
    assert!(allowed(&p, c, "example.com:8080"));
    assert!(allowed(&p, c, "a.cdn.example.net"));
    for bad in [
        "example.com.evil.io",
        "evilexample.com",
        "cdn.example.net",
        "xcdn.example.net",
        "a.cdn.example.net.evil.io",
    ] {
        assert!(!allowed(&p, c, bad), "{bad:?}");
    }
}

#[test]
fn malformed_scopes_are_reported_by_validation() {
    use Capability::*;
    for (cap, ok) in [
        (NativeDeclare, "kernel32!GetTickCount"),
        (NativeDeclare, "kernel32.dll!*"),
        (ComCreate, "Scripting.Dictionary"),
        (ComCreate, "Scripting.*"),
        (ComCreate, "Scripting."),
        (ComCreate, "*"),
        (ProcessSpawn, "notepad"),
        (ProcessSpawn, "C:\\Program Files\\x\\a.exe"),
        (UiAutomation, "SendKeys"),
        (Network, "*.example.com"),
        (FileRead, "/data"),
    ] {
        assert_eq!(scope::validate_scope(cap, ok), Ok(()), "{ok}");
    }
    for (cap, bad) in [
        (NativeDeclare, "kernel32"),
        (NativeDeclare, "kernel32!"),
        (NativeDeclare, "kernel32!Get*"),
        (NativeDeclare, "*!Sleep"),
        (ComCreate, "Scripting.D*"),
        (ComCreate, "*.Dictionary"),
        (Network, "ex*.com"),
        (FileRead, ""),
    ] {
        assert!(scope::validate_scope(cap, bad).is_err(), "{bad}");
    }
}
