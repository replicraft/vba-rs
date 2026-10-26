//! Protocol tests: the real `vba-lsp` binary over stdio JSON-RPC.
#![allow(missing_docs)]

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use serde_json::{Value, json};

const URI: &str = "file:///project/Module1.bas";
const TIMEOUT: Duration = Duration::from_secs(20);

struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<Value>,
    queue: Vec<Value>,
    next_id: i64,
}

impl Client {
    fn spawn() -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vba-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn vba-lsp");
        let stdin = child.stdin.take();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            loop {
                let mut len = None;
                loop {
                    let mut line = String::new();
                    match out.read_line(&mut line) {
                        Ok(0) | Err(_) => return,
                        Ok(_) => {}
                    }
                    let line = line.trim_end();
                    if line.is_empty() {
                        break;
                    }
                    if let Some(v) = line.strip_prefix("Content-Length: ") {
                        len = v.parse::<usize>().ok();
                    }
                }
                let mut body = vec![0; len.expect("Content-Length header")];
                if out.read_exact(&mut body).is_err() {
                    return;
                }
                if tx
                    .send(serde_json::from_slice(&body).expect("valid JSON"))
                    .is_err()
                {
                    return;
                }
            }
        });
        Client {
            child,
            stdin,
            rx,
            queue: Vec::new(),
            next_id: 0,
        }
    }

    fn send(&mut self, v: Value) {
        let body = v.to_string();
        let stdin = self.stdin.as_mut().expect("stdin open");
        write!(stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        stdin.flush().unwrap();
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    /// Next message satisfying `pred`; earlier others stay queued.
    fn wait(&mut self, pred: impl Fn(&Value) -> bool) -> Value {
        if let Some(i) = self.queue.iter().position(&pred) {
            return self.queue.remove(i);
        }
        loop {
            let m = self.rx.recv_timeout(TIMEOUT).expect("server message");
            if pred(&m) {
                return m;
            }
            self.queue.push(m);
        }
    }

    /// Sends a request and returns the whole response message.
    fn request_raw(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        self.wait(|m| m["id"] == json!(id) && m.get("method").is_none())
    }

    /// Sends a request that must succeed and returns its result.
    fn request(&mut self, method: &str, params: Value) -> Value {
        let r = self.request_raw(method, params);
        assert!(r.get("error").is_none(), "{method} failed: {r}");
        r["result"].clone()
    }

    fn diagnostics(&mut self, version: i64) -> Vec<Value> {
        let m = self.wait(|m| {
            m["method"] == "textDocument/publishDiagnostics"
                && m["params"]["uri"] == URI
                && m["params"]["version"] == version
        });
        m["params"]["diagnostics"].as_array().unwrap().clone()
    }

    fn exit_code(mut self) -> Option<i32> {
        drop(self.stdin.take());
        let deadline = std::time::Instant::now() + TIMEOUT;
        loop {
            if let Some(s) = self.child.try_wait().unwrap() {
                return s.code();
            }
            assert!(std::time::Instant::now() < deadline, "server did not exit");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn error_code(r: &Value) -> i64 {
    r["error"]["code"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected error: {r}"))
}

fn pos(line: u64, character: u64) -> Value {
    json!({"line": line, "character": character})
}

fn at(line: u64, character: u64) -> Value {
    json!({"textDocument": {"uri": URI}, "position": pos(line, character)})
}

const SOURCE: &str = "\
Declare PtrSafe Function GetTick Lib \"kernel32\" Alias \"GetTickCount\" () As Long
Option Explicit

Public Function Add(ByVal a As Long, ByVal b As Long) As Long
    Dim total As Long
    total = a + = b
    Add = total
End Function

Public Sub Run()
    Dim s As String
    s = \"😀\" & Left(\"abc\", 1) & CStr(Add(1, 2))
End Sub
";

fn initialize(c: &mut Client, with_config: bool) -> Value {
    let result = c.request(
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {
                "workspace": {"configuration": with_config},
                "textDocument": {"documentSymbol": {"hierarchicalDocumentSymbolSupport": true}}
            },
            "initializationOptions": {"vba": {"profile": "vba7.1-excel-win64"}},
        }),
    );
    c.notify("initialized", json!({}));
    result
}

#[test]
fn full_session() {
    let mut c = Client::spawn();
    let init = initialize(&mut c, true);

    // capabilities
    let caps = &init["capabilities"];
    assert_eq!(init["serverInfo"]["name"], "vba-lsp");
    assert_eq!(caps["positionEncoding"], "utf-16");
    assert_eq!(caps["textDocumentSync"]["change"], 2);
    assert_eq!(caps["renameProvider"]["prepareProvider"], true);
    assert_eq!(caps["hoverProvider"], true);
    assert_eq!(caps["definitionProvider"], true);
    assert_eq!(caps["referencesProvider"], true);
    assert_eq!(caps["documentFormattingProvider"], true);
    assert_eq!(caps["documentRangeFormattingProvider"], true);
    assert_eq!(caps["foldingRangeProvider"], true);
    assert_eq!(caps["selectionRangeProvider"], true);
    assert!(
        caps["completionProvider"]["triggerCharacters"]
            .as_array()
            .unwrap()
            .contains(&json!("."))
    );
    assert!(
        caps["signatureHelpProvider"]["triggerCharacters"]
            .as_array()
            .unwrap()
            .contains(&json!("("))
    );
    let legend = &caps["semanticTokensProvider"]["legend"];
    assert_eq!(legend["tokenTypes"][0], "keyword");
    assert_eq!(caps["semanticTokensProvider"]["range"], true);

    // the server pulls the `vba` configuration
    let req = c.wait(|m| m["method"] == "workspace/configuration");
    assert_eq!(req["params"]["items"][0]["section"], "vba");
    c.send(
        json!({"jsonrpc": "2.0", "id": req["id"], "result": [{"profile": "vba7.1-excel-win32"}]}),
    );

    // open: the syntax error is reported, the Declare is fine on 32 bit
    c.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": URI, "languageId": "vba", "version": 1, "text": SOURCE}}),
    );
    let d = c.diagnostics(1);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0]["source"], "vba");
    assert_eq!(d[0]["severity"], 1);
    assert!(d[0]["code"].as_str().unwrap().starts_with("SYN"));
    assert_eq!(d[0]["range"]["start"]["line"], 5);

    // documentSymbol is hierarchical
    let syms = c.request(
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": URI}}),
    );
    let names: Vec<&str> = syms
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["GetTick", "Add", "Run"]);
    assert_eq!(syms[1]["children"][0]["name"], "total");
    assert_eq!(syms[1]["kind"], 12); // Function

    // hover on Left, after an emoji (UTF-16 column 15)
    let h = c.request("textDocument/hover", at(11, 16));
    assert!(
        h["contents"]["value"].as_str().unwrap().contains("Left("),
        "{h}"
    );
    assert_eq!(h["range"]["start"]["character"], 15);

    // definition / references / rename
    let def = c.request("textDocument/definition", at(6, 12));
    assert_eq!(def[0]["range"]["start"], pos(4, 8));
    let refs = c.request(
        "textDocument/references",
        json!({"textDocument": {"uri": URI}, "position": pos(6, 12), "context": {"includeDeclaration": false}}),
    );
    assert_eq!(refs.as_array().unwrap().len(), 2);
    let prep = c.request("textDocument/prepareRename", at(6, 12));
    assert_eq!(prep["start"], pos(6, 10));
    let edit = c.request(
        "textDocument/rename",
        json!({"textDocument": {"uri": URI}, "position": pos(6, 12), "newName": "sum"}),
    );
    assert_eq!(edit["changes"][URI].as_array().unwrap().len(), 3);
    let bad = c.request_raw(
        "textDocument/rename",
        json!({"textDocument": {"uri": URI}, "position": pos(6, 12), "newName": "End"}),
    );
    assert_eq!(error_code(&bad), -32803, "{bad}");

    // completion and signature help
    let comp = c.request("textDocument/completion", at(10, 4));
    let labels: Vec<&str> = comp["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["label"].as_str().unwrap())
        .collect();
    assert!(labels.contains(&"Dim") && labels.contains(&"Add") && labels.contains(&"Left"));
    assert!(comp["items"][0]["textEdit"]["range"].is_object());
    let sig = c.request("textDocument/signatureHelp", at(11, 20));
    assert!(
        sig["signatures"][0]["label"]
            .as_str()
            .unwrap()
            .starts_with("Left(")
    );
    assert_eq!(sig["activeParameter"], 0);
    assert!(sig["signatures"][0]["parameters"][0]["label"].is_array());

    // folding, selection, semantic tokens, formatting
    let folds = c.request(
        "textDocument/foldingRange",
        json!({"textDocument": {"uri": URI}}),
    );
    assert!(
        folds
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["startLine"] == 3 && f["endLine"] == 7)
    );
    let sel = c.request(
        "textDocument/selectionRange",
        json!({"textDocument": {"uri": URI}, "positions": [pos(6, 12)]}),
    );
    assert!(sel[0]["parent"].is_object());
    let toks = c.request(
        "textDocument/semanticTokens/full",
        json!({"textDocument": {"uri": URI}}),
    );
    let data = toks["data"].as_array().unwrap();
    assert!(!data.is_empty() && data.len().is_multiple_of(5));
    let part = c.request(
        "textDocument/semanticTokens/range",
        json!({"textDocument": {"uri": URI}, "range": {"start": pos(10, 0), "end": pos(12, 0)}}),
    );
    assert!(part["data"].as_array().unwrap().len() < data.len());
    let fmt = c.request(
        "textDocument/formatting",
        json!({"textDocument": {"uri": URI}, "options": {"tabSize": 2, "insertSpaces": true}}),
    );
    assert!(fmt.as_array().unwrap().iter().any(|e| e["newText"] == "  "));
    let rf = c.request(
        "textDocument/rangeFormatting",
        json!({"textDocument": {"uri": URI}, "range": {"start": pos(4, 0), "end": pos(5, 0)},
               "options": {"tabSize": 4, "insertSpaces": true}}),
    );
    assert!(rf.as_array().unwrap().len() <= 1);

    // incremental change fixes the error
    c.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": URI, "version": 2},
            "contentChanges": [{"range": {"start": pos(5, 16), "end": pos(5, 18)}, "text": ""}],
        }),
    );
    assert_eq!(c.diagnostics(2), Vec::<Value>::new());

    // a settings change re-publishes: PtrSafe is an error before VBA 7
    c.notify(
        "workspace/didChangeConfiguration",
        json!({"settings": {"vba": {"profile": "vba6-excel-win32"}}}),
    );
    let d = c.diagnostics(2);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0]["code"], "SYN010");

    // explicit errors
    let r = c.request_raw("textDocument/codeAction", json!({}));
    assert_eq!(error_code(&r), -32601);
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap()
            .contains("codeAction")
    );
    let r = c.request_raw(
        "textDocument/hover",
        json!({"textDocument": {"uri": "file:///other.bas"}, "position": pos(0, 0)}),
    );
    assert_eq!(error_code(&r), -32602);
    let r = c.request_raw("textDocument/hover", json!({"nonsense": true}));
    assert_eq!(error_code(&r), -32602);
    let r = c.request_raw(
        "textDocument/rename",
        json!({"textDocument": {"uri": URI}, "position": pos(11, 16), "newName": "L"}),
    );
    assert_eq!(error_code(&r), -32803); // library function: unsupported
    assert!(r["error"]["message"].as_str().unwrap().contains("library"));

    // close clears diagnostics
    c.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": URI}}),
    );
    let m = c.wait(|m| {
        m["method"] == "textDocument/publishDiagnostics"
            && m["params"]["diagnostics"]
                .as_array()
                .is_some_and(Vec::is_empty)
            && m["params"]["version"].is_null()
    });
    assert_eq!(m["params"]["uri"], URI);

    // shutdown / exit
    assert_eq!(c.request("shutdown", Value::Null), Value::Null);
    let r = c.request_raw("textDocument/hover", at(0, 0));
    assert_eq!(error_code(&r), -32600);
    c.notify("exit", Value::Null);
    assert_eq!(c.exit_code(), Some(0));
}

#[test]
fn flat_symbols_without_hierarchical_support_and_no_config_pull() {
    let mut c = Client::spawn();
    let init = c.request(
        "initialize",
        json!({"processId": null, "rootUri": null, "capabilities": {},
               "initializationOptions": {"profile": "vba6-excel-win32", "bogus": 1}}),
    );
    assert_eq!(init["capabilities"]["hoverProvider"], true);
    c.notify("initialized", json!({}));
    // bad initialization option is reported, not fatal
    let log = c.wait(|m| m["method"] == "window/logMessage");
    assert!(log["params"]["message"].as_str().unwrap().contains("bogus"));
    c.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": URI, "languageId": "vba", "version": 7, "text": SOURCE}}),
    );
    // vba6 profile from the initialization options: PtrSafe is an error
    let d = c.diagnostics(7);
    assert!(d.iter().any(|d| d["code"] == "SYN010"), "{d:?}");
    let syms = c.request(
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": URI}}),
    );
    let first = &syms[0];
    assert_eq!(first["name"], "GetTick");
    assert_eq!(first["location"]["uri"], URI);
    assert!(
        syms.as_array()
            .unwrap()
            .iter()
            .any(|s| s["name"] == "total" && s["containerName"] == "Add")
    );
    assert_eq!(c.request("shutdown", Value::Null), Value::Null);
    c.notify("exit", Value::Null);
    assert_eq!(c.exit_code(), Some(0));
}

#[test]
fn exit_without_shutdown_is_an_error_exit() {
    let mut c = Client::spawn();
    initialize(&mut c, false);
    c.notify("exit", Value::Null);
    assert_eq!(c.exit_code(), Some(1));
}

#[test]
fn closing_stdin_ends_the_server() {
    let mut c = Client::spawn();
    initialize(&mut c, false);
    assert_eq!(c.exit_code(), Some(1));
}

#[test]
fn utf16_incremental_changes_over_the_wire() {
    let mut c = Client::spawn();
    initialize(&mut c, false);
    c.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": URI, "languageId": "vba", "version": 1,
                                "text": "Sub A()\n    s = \"😀\" & x\nEnd Sub\n"}}),
    );
    assert!(c.diagnostics(1).is_empty());
    // columns count UTF-16 units: replace the `x` at column 16
    c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 2},
               "contentChanges": [{"range": {"start": pos(1, 16), "end": pos(1, 17)}, "text": "= ="}]}),
    );
    assert!(!c.diagnostics(2).is_empty());
    c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 3},
               "contentChanges": [{"text": "Sub B()\nEnd Sub\n"}]}),
    );
    assert!(c.diagnostics(3).is_empty());
    let syms = c.request(
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": URI}}),
    );
    assert_eq!(syms[0]["name"], "B");
}
