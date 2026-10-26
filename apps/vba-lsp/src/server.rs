//! The request loop.

use std::collections::HashMap;
use std::error::Error as StdError;

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types as lsp;
use lsp_types::notification::Notification as _;
use lsp_types::request::Request as _;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use vba_lsp_core::{self as core, Workspace};

use crate::{config, convert};

/// Upper bound on workspace symbols returned for one query.
const MAX_WORKSPACE_SYMBOLS: usize = 500;

type Result<T> = std::result::Result<T, Box<dyn StdError + Sync + Send>>;

/// A failed request: an LSP error code and message.
struct Failure(ErrorCode, String);

impl From<core::Error> for Failure {
    fn from(e: core::Error) -> Self {
        match e {
            core::Error::NotOpen(_) => Failure(ErrorCode::InvalidParams, e.to_string()),
            core::Error::Invalid(_) | core::Error::Unsupported(_) => {
                Failure(ErrorCode::RequestFailed, e.to_string())
            }
        }
    }
}

fn params<T: DeserializeOwned>(v: Value) -> std::result::Result<T, Failure> {
    serde_json::from_value(v)
        .map_err(|e| Failure(ErrorCode::InvalidParams, format!("invalid parameters: {e}")))
}

fn reply<T: serde::Serialize>(v: T) -> std::result::Result<Value, Failure> {
    serde_json::to_value(v).map_err(|e| Failure(ErrorCode::InternalError, e.to_string()))
}

/// The capabilities announced to the client.
pub fn capabilities() -> lsp::ServerCapabilities {
    lsp::ServerCapabilities {
        position_encoding: Some(lsp::PositionEncodingKind::UTF16),
        text_document_sync: Some(lsp::TextDocumentSyncCapability::Options(
            lsp::TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(lsp::TextDocumentSyncKind::INCREMENTAL),
                ..Default::default()
            },
        )),
        hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
        completion_provider: Some(lsp::CompletionOptions {
            trigger_characters: Some(vec![".".to_string()]),
            ..Default::default()
        }),
        signature_help_provider: Some(lsp::SignatureHelpOptions {
            trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
            retrigger_characters: None,
            work_done_progress_options: Default::default(),
        }),
        definition_provider: Some(lsp::OneOf::Left(true)),
        references_provider: Some(lsp::OneOf::Left(true)),
        rename_provider: Some(lsp::OneOf::Right(lsp::RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: Default::default(),
        })),
        document_symbol_provider: Some(lsp::OneOf::Left(true)),
        workspace_symbol_provider: Some(lsp::OneOf::Left(true)),
        folding_range_provider: Some(lsp::FoldingRangeProviderCapability::Simple(true)),
        selection_range_provider: Some(lsp::SelectionRangeProviderCapability::Simple(true)),
        document_formatting_provider: Some(lsp::OneOf::Left(true)),
        document_range_formatting_provider: Some(lsp::OneOf::Left(true)),
        semantic_tokens_provider: Some(
            lsp::SemanticTokensServerCapabilities::SemanticTokensOptions(
                lsp::SemanticTokensOptions {
                    work_done_progress_options: Default::default(),
                    legend: convert::legend(),
                    range: Some(true),
                    full: Some(lsp::SemanticTokensFullOptions::Bool(true)),
                },
            ),
        ),
        ..Default::default()
    }
}

pub struct Server {
    conn: Connection,
    ws: Workspace,
    shutdown: bool,
    pull_config: bool,
    hierarchical_symbols: bool,
    next_id: i32,
    pending_config: Vec<RequestId>,
    /// Open documents whose diagnostics may be stale. Publishing waits
    /// until the client has no further message queued, so a burst of edits
    /// costs one analysis of the project.
    dirty: bool,
    published: HashMap<String, (i32, Vec<lsp::Diagnostic>)>,
}

impl Server {
    /// Performs the initialize handshake on `conn` and returns the server.
    pub fn initialize(conn: Connection) -> Result<Server> {
        let (id, init) = conn.initialize_start()?;
        let init: lsp::InitializeParams = serde_json::from_value(init)?;
        let workspace_caps = init.capabilities.workspace.as_ref();
        let pull_config = workspace_caps.and_then(|w| w.configuration) == Some(true);
        let hierarchical_symbols = init
            .capabilities
            .text_document
            .as_ref()
            .and_then(|t| t.document_symbol.as_ref())
            .and_then(|d| d.hierarchical_document_symbol_support)
            == Some(true);
        let mut ws = Workspace::new();
        let mut warnings = Vec::new();
        if let Some(opts) = &init.initialization_options {
            let (cfg, w) = config::from_settings(opts);
            ws.set_config(cfg);
            warnings = w;
        }
        let result = json!({
            "capabilities": capabilities(),
            "serverInfo": {"name": "vba-lsp", "version": env!("CARGO_PKG_VERSION")},
        });
        conn.initialize_finish(id, result)?;
        let mut s = Server {
            conn,
            ws,
            shutdown: false,
            pull_config,
            hierarchical_symbols,
            next_id: 0,
            pending_config: Vec::new(),
            dirty: false,
            published: HashMap::new(),
        };
        for w in warnings {
            s.log(lsp::MessageType::WARNING, w)?;
        }
        // `initialize_finish` consumed the `initialized` notification, so
        // this is the moment the client accepts requests.
        if s.pull_config {
            s.request_configuration()?;
        }
        Ok(s)
    }

    /// Runs until `exit`. Returns the process exit code (0 after a clean
    /// shutdown, 1 otherwise, as the protocol specifies).
    pub fn run(mut self) -> Result<i32> {
        while let Ok(msg) = self.conn.receiver.recv() {
            match msg {
                Message::Request(req) => self.on_request(req)?,
                Message::Notification(n) => {
                    if n.method == lsp::notification::Exit::METHOD {
                        return Ok(if self.shutdown { 0 } else { 1 });
                    }
                    self.on_notification(n)?;
                }
                Message::Response(r) => self.on_response(r)?,
            }
            if self.dirty && self.conn.receiver.is_empty() {
                self.publish_all()?;
            }
        }
        // The client closed the pipe without `exit`.
        Ok(if self.shutdown { 0 } else { 1 })
    }

    fn send(&self, msg: impl Into<Message>) -> Result<()> {
        self.conn.sender.send(msg.into())?;
        Ok(())
    }

    fn notify<N: lsp::notification::Notification>(&self, p: N::Params) -> Result<()> {
        self.send(Notification::new(N::METHOD.to_string(), p))
    }

    fn log(&self, ty: lsp::MessageType, message: String) -> Result<()> {
        self.notify::<lsp::notification::LogMessage>(lsp::LogMessageParams { typ: ty, message })
    }

    fn on_request(&mut self, req: Request) -> Result<()> {
        if self.shutdown {
            return self.send(Response::new_err(
                req.id,
                ErrorCode::InvalidRequest as i32,
                "the server is shutting down".to_string(),
            ));
        }
        if req.method == lsp::request::Shutdown::METHOD {
            self.shutdown = true;
            return self.send(Response::new_ok(req.id, Value::Null));
        }
        let id = req.id.clone();
        let response = match self.dispatch(&req.method, req.params) {
            Ok(v) => Response::new_ok(id, v),
            Err(Failure(code, msg)) => Response::new_err(id, code as i32, msg),
        };
        self.send(response)
    }

    fn view(&self, uri: &lsp::Uri) -> std::result::Result<core::View<'_>, Failure> {
        Ok(self.ws.view(uri.as_str())?)
    }

    fn dispatch(&self, method: &str, p: Value) -> std::result::Result<Value, Failure> {
        use lsp::request as r;
        match method {
            r::HoverRequest::METHOD => {
                let p: lsp::HoverParams = params(p)?;
                let tdp = p.text_document_position_params;
                let v = self.view(&tdp.text_document.uri)?;
                reply(v.hover(convert::position(tdp.position)).map(convert::hover))
            }
            r::Completion::METHOD => {
                let p: lsp::CompletionParams = params(p)?;
                let tdp = p.text_document_position;
                let v = self.view(&tdp.text_document.uri)?;
                reply(convert::completions(
                    v.completions(convert::position(tdp.position)),
                ))
            }
            r::SignatureHelpRequest::METHOD => {
                let p: lsp::SignatureHelpParams = params(p)?;
                let tdp = p.text_document_position_params;
                let v = self.view(&tdp.text_document.uri)?;
                reply(
                    v.signature_help(convert::position(tdp.position))
                        .map(convert::signature_help),
                )
            }
            r::GotoDefinition::METHOD => {
                let p: lsp::GotoDefinitionParams = params(p)?;
                let tdp = p.text_document_position_params;
                let locs: Vec<lsp::Location> = self
                    .ws
                    .definition(
                        tdp.text_document.uri.as_str(),
                        convert::position(tdp.position),
                    )?
                    .into_iter()
                    .filter_map(convert::location)
                    .collect();
                if locs.is_empty() {
                    Ok(Value::Null)
                } else {
                    reply(lsp::GotoDefinitionResponse::Array(locs))
                }
            }
            r::References::METHOD => {
                let p: lsp::ReferenceParams = params(p)?;
                let tdp = p.text_document_position;
                let locs: Vec<lsp::Location> = self
                    .ws
                    .references(
                        tdp.text_document.uri.as_str(),
                        convert::position(tdp.position),
                        p.context.include_declaration,
                    )?
                    .into_iter()
                    .filter_map(convert::location)
                    .collect();
                reply(locs)
            }
            r::PrepareRenameRequest::METHOD => {
                let p: lsp::TextDocumentPositionParams = params(p)?;
                let range = self
                    .ws
                    .prepare_rename(p.text_document.uri.as_str(), convert::position(p.position))?;
                reply(lsp::PrepareRenameResponse::Range(convert::to_range(range)))
            }
            r::Rename::METHOD => {
                let p: lsp::RenameParams = params(p)?;
                let tdp = p.text_document_position;
                let files = self.ws.rename(
                    tdp.text_document.uri.as_str(),
                    convert::position(tdp.position),
                    &p.new_name,
                )?;
                reply(convert::workspace_edit(files))
            }
            r::WorkspaceSymbolRequest::METHOD => {
                let p: lsp::WorkspaceSymbolParams = params(p)?;
                let syms: Vec<lsp::SymbolInformation> = self
                    .ws
                    .workspace_symbols(&p.query, MAX_WORKSPACE_SYMBOLS)
                    .into_iter()
                    .filter_map(convert::workspace_symbol)
                    .collect();
                reply(lsp::WorkspaceSymbolResponse::Flat(syms))
            }
            r::DocumentSymbolRequest::METHOD => {
                let p: lsp::DocumentSymbolParams = params(p)?;
                let uri = p.text_document.uri;
                let syms = self.view(&uri)?.document_symbols();
                if self.hierarchical_symbols {
                    reply(
                        syms.into_iter()
                            .map(convert::document_symbol)
                            .collect::<Vec<_>>(),
                    )
                } else {
                    let mut flat = Vec::new();
                    convert::flat_symbols(&uri, syms, None, &mut flat);
                    reply(flat)
                }
            }
            r::FoldingRangeRequest::METHOD => {
                let p: lsp::FoldingRangeParams = params(p)?;
                let v = self.view(&p.text_document.uri)?;
                reply(
                    v.folding_ranges()
                        .into_iter()
                        .map(convert::folding)
                        .collect::<Vec<_>>(),
                )
            }
            r::SelectionRangeRequest::METHOD => {
                let p: lsp::SelectionRangeParams = params(p)?;
                let v = self.view(&p.text_document.uri)?;
                let ps: Vec<core::Position> =
                    p.positions.into_iter().map(convert::position).collect();
                reply(
                    v.selection_ranges(&ps)
                        .into_iter()
                        .map(convert::selection_range)
                        .collect::<Vec<_>>(),
                )
            }
            r::SemanticTokensFullRequest::METHOD => {
                let p: lsp::SemanticTokensParams = params(p)?;
                let v = self.view(&p.text_document.uri)?;
                reply(convert::semantic_tokens(&v.semantic_tokens()))
            }
            r::SemanticTokensRangeRequest::METHOD => {
                let p: lsp::SemanticTokensRangeParams = params(p)?;
                let v = self.view(&p.text_document.uri)?;
                reply(convert::semantic_tokens(
                    &v.semantic_tokens_range(convert::range(p.range)),
                ))
            }
            r::Formatting::METHOD => {
                let p: lsp::DocumentFormattingParams = params(p)?;
                let v = self.view(&p.text_document.uri)?;
                reply(
                    v.format(format_options(&p.options))
                        .into_iter()
                        .map(convert::text_edit)
                        .collect::<Vec<_>>(),
                )
            }
            r::RangeFormatting::METHOD => {
                let p: lsp::DocumentRangeFormattingParams = params(p)?;
                let v = self.view(&p.text_document.uri)?;
                reply(
                    v.format_range(convert::range(p.range), format_options(&p.options))
                        .into_iter()
                        .map(convert::text_edit)
                        .collect::<Vec<_>>(),
                )
            }
            other => Err(Failure(
                ErrorCode::MethodNotFound,
                format!("vba-lsp does not support `{other}`"),
            )),
        }
    }

    fn on_notification(&mut self, n: Notification) -> Result<()> {
        use lsp::notification as nt;
        match n.method.as_str() {
            nt::DidOpenTextDocument::METHOD => {
                let p: lsp::DidOpenTextDocumentParams = serde_json::from_value(n.params)?;
                let d = p.text_document;
                self.ws.open(d.uri.as_str(), &d.text, d.version);
                self.dirty = true;
            }
            nt::DidChangeTextDocument::METHOD => {
                let p: lsp::DidChangeTextDocumentParams = serde_json::from_value(n.params)?;
                let changes: Vec<core::TextChange> = p
                    .content_changes
                    .into_iter()
                    .map(|c| core::TextChange {
                        range: c.range.map(convert::range),
                        text: c.text,
                    })
                    .collect();
                let uri = p.text_document.uri;
                match self
                    .ws
                    .change(uri.as_str(), p.text_document.version, &changes)
                {
                    Ok(()) => self.dirty = true,
                    Err(e) => self.log(lsp::MessageType::WARNING, e.to_string())?,
                }
            }
            nt::DidCloseTextDocument::METHOD => {
                let p: lsp::DidCloseTextDocumentParams = serde_json::from_value(n.params)?;
                self.ws.close(p.text_document.uri.as_str());
                self.published.remove(p.text_document.uri.as_str());
                self.dirty = true;
                // Clear the diagnostics of a closed document.
                self.notify::<nt::PublishDiagnostics>(lsp::PublishDiagnosticsParams::new(
                    p.text_document.uri,
                    Vec::new(),
                    None,
                ))?;
            }
            nt::DidChangeConfiguration::METHOD => {
                let p: lsp::DidChangeConfigurationParams = serde_json::from_value(n.params)?;
                if p.settings.get("vba").is_some() {
                    self.apply_settings(&p.settings)?;
                } else if self.pull_config {
                    self.request_configuration()?;
                }
            }
            // Cancellation and tracing hints, saves and everything else a
            // client may send are not needed; notifications cannot be
            // answered with an error.
            _ => {}
        }
        Ok(())
    }

    fn request_configuration(&mut self) -> Result<()> {
        self.next_id += 1;
        let params = lsp::ConfigurationParams {
            items: vec![lsp::ConfigurationItem {
                scope_uri: None,
                section: Some("vba".to_string()),
            }],
        };
        let id = RequestId::from(format!("vba-lsp-config-{}", self.next_id));
        self.pending_config.push(id.clone());
        self.send(Request::new(
            id,
            lsp::request::WorkspaceConfiguration::METHOD.to_string(),
            params,
        ))
    }

    fn on_response(&mut self, r: Response) -> Result<()> {
        let Some(i) = self.pending_config.iter().position(|id| *id == r.id) else {
            return Ok(());
        };
        self.pending_config.swap_remove(i);
        match r.response_result {
            Ok(Value::Array(items)) if !items.is_empty() => self.apply_settings(&items[0])?,
            Ok(_) => self.apply_settings(&Value::Null)?,
            Err(e) => self.log(
                lsp::MessageType::WARNING,
                format!("could not read the `vba` configuration: {}", e.message),
            )?,
        }
        Ok(())
    }

    fn apply_settings(&mut self, settings: &Value) -> Result<()> {
        let (cfg, warnings) = config::from_settings(settings);
        self.ws.set_config(cfg);
        for w in warnings {
            self.log(lsp::MessageType::WARNING, w)?;
        }
        // Every document is re-analysed with the new settings.
        self.published.clear();
        self.dirty = true;
        Ok(())
    }

    /// Publishes the diagnostics of every open document that changed since
    /// they were last sent: a semantic error in one module can appear or
    /// disappear in another when a module is edited.
    fn publish_all(&mut self) -> Result<()> {
        self.dirty = false;
        let mut uris: Vec<String> = self.ws.open_uris().into_iter().map(String::from).collect();
        uris.sort();
        for u in uris {
            let Ok(uri) = u.parse::<lsp::Uri>() else {
                continue;
            };
            let Ok(view) = self.ws.view(&u) else { continue };
            let diags: Vec<lsp::Diagnostic> = view
                .diagnostics()
                .into_iter()
                .map(|d| convert::diagnostic(&uri, d))
                .collect();
            let version = view.document().version();
            if self
                .published
                .get(&u)
                .is_some_and(|(v, d)| *v == version && *d == diags)
            {
                continue;
            }
            self.notify::<lsp::notification::PublishDiagnostics>(
                lsp::PublishDiagnosticsParams::new(uri, diags.clone(), Some(version)),
            )?;
            self.published.insert(u, (version, diags));
        }
        Ok(())
    }
}

fn format_options(o: &lsp::FormattingOptions) -> core::FormatOptions {
    core::FormatOptions {
        tab_size: o.tab_size.clamp(1, 16),
        insert_spaces: o.insert_spaces,
        keyword_case: matches!(
            o.properties.get("keywordCase"),
            Some(lsp::FormattingProperty::Bool(true))
        ),
    }
}
