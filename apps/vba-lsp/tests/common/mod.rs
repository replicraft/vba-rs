//! Shared protocol-test client: spawns `vba-lsp` and speaks JSON-RPC over stdio.
#![allow(dead_code, missing_docs)]

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use serde_json::{Value, json};

pub const URI: &str = "file:///project/Module1.bas";
pub const TIMEOUT: Duration = Duration::from_secs(20);

pub struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<Value>,
    queue: Vec<Value>,
    next_id: i64,
}

impl Client {
    pub fn spawn() -> Client {
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

    pub fn send(&mut self, v: Value) {
        let body = v.to_string();
        let stdin = self.stdin.as_mut().expect("stdin open");
        write!(stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        stdin.flush().unwrap();
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    /// Next message satisfying `pred`; earlier others stay queued.
    pub fn wait(&mut self, pred: impl Fn(&Value) -> bool) -> Value {
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
    pub fn request_raw(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        self.wait(|m| m["id"] == json!(id) && m.get("method").is_none())
    }

    /// Sends a request that must succeed and returns its result.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let r = self.request_raw(method, params);
        assert!(r.get("error").is_none(), "{method} failed: {r}");
        r["result"].clone()
    }

    pub fn diagnostics(&mut self, version: i64) -> Vec<Value> {
        let m = self.wait(|m| {
            m["method"] == "textDocument/publishDiagnostics"
                && m["params"]["uri"] == URI
                && m["params"]["version"] == version
        });
        m["params"]["diagnostics"].as_array().unwrap().clone()
    }

    pub fn exit_code(mut self) -> Option<i32> {
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

pub fn error_code(r: &Value) -> i64 {
    r["error"]["code"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected error: {r}"))
}
