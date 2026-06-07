use std::sync::Mutex;
use std::fs;
use std::path::Path;
use lsp_server::{Connection, Message, Request, Response};
use lsp_types::*;
use lsp_types::notification::*;
use lsp_types::request::*;
use lsp_types::request::Request as _;

use crate::lsp::store::DocumentStore;
use crate::lsp::bindings::Bindings;
use crate::lsp::handlers;

pub fn run(bindings_path: Option<&str>) -> Result<(), String> {
    eprintln!("Wolfram LSP starting...");
    let (connection, io_threads) = Connection::stdio();
    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".into(), ":".into()]),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".into(), ",".into()]),
            ..Default::default()
        }),
        definition_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    };

    let init_result = InitializeResult {
        capabilities: server_capabilities,
        server_info: Some(ServerInfo {
            name: "wolfram".into(),
            version: Some("0.1.0".into()),
        }),
        offset_encoding: None,
    };

    let initialize_id = connection.initialize(serde_json::to_value(&init_result).unwrap());
    if let Err(e) = initialize_id {
        eprintln!("Initialize failed: {:?}", e);
        return Err(format!("Initialize failed: {:?}", e));
    }

    let state = Mutex::new(ServerState {
        store: DocumentStore::new(),
        debounce_timer: std::time::Instant::now(),
        workspace_files: scan_workspace_files("."),
    });

    let mut bindings = Bindings::load(bindings_path);
    bindings.load_workspace_wolds(".");

    eprintln!("Wolfram LSP ready");

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req).unwrap_or(false) {
                    break;
                }
                let response = handle_request(&state, &bindings, &req);
                if let Some(resp) = response {
                    if let Err(e) = connection.sender.send(Message::Response(resp)) {
                        eprintln!("Failed to send response: {:?}", e);
                    }
                }
            }
            Message::Notification(not) => {
                handle_notification(&state, &connection, &not);
            }
            Message::Response(_) => {}
        }
    }

    io_threads.join().map_err(|e| format!("IO thread error: {:?}", e))?;
    Ok(())
}

struct ServerState {
    store: DocumentStore,
    debounce_timer: std::time::Instant,
    workspace_files: Vec<String>,
}

fn handle_request(
    state: &Mutex<ServerState>,
    bindings: &Bindings,
    req: &Request,
) -> Option<Response> {
    match req.method.as_str() {
        Completion::METHOD => {
            let params: CompletionParams = serde_json::from_value(req.params.clone()).ok()?;
            let mut s = state.lock().ok()?;
            let workspace_files = s.workspace_files.clone();
            let result = handlers::handle_completion(&mut s.store, bindings, &workspace_files, params)?;
            Some(Response {
                id: req.id.clone(),
                result: Some(serde_json::to_value(&result).unwrap()),
                error: None,
            })
        }
        HoverRequest::METHOD => {
            let params: HoverParams = serde_json::from_value(req.params.clone()).ok()?;
            let mut s = state.lock().ok()?;
            let result = handlers::handle_hover(&mut s.store, bindings, params)?;
            Some(Response {
                id: req.id.clone(),
                result: Some(serde_json::to_value(&result).unwrap()),
                error: None,
            })
        }
        GotoDefinition::METHOD => {
            let params: GotoDefinitionParams = serde_json::from_value(req.params.clone()).ok()?;
            let mut s = state.lock().ok()?;
            let result = handlers::handle_definition(&mut s.store, params)?;
            Some(Response {
                id: req.id.clone(),
                result: Some(serde_json::to_value(&result).unwrap()),
                error: None,
            })
        }
        SignatureHelpRequest::METHOD => {
            let params: SignatureHelpParams = serde_json::from_value(req.params.clone()).ok()?;
            let mut s = state.lock().ok()?;
            let result = handlers::handle_signature_help(&mut s.store, bindings, params)?;
            Some(Response {
                id: req.id.clone(),
                result: Some(serde_json::to_value(&result).unwrap()),
                error: None,
            })
        }
        DocumentSymbolRequest::METHOD => {
            let params: DocumentSymbolParams = serde_json::from_value(req.params.clone()).ok()?;
            let mut s = state.lock().ok()?;
            let result = handlers::handle_document_symbols(&mut s.store, params)?;
            Some(Response {
                id: req.id.clone(),
                result: Some(serde_json::to_value(&result).unwrap()),
                error: None,
            })
        }
        _ => None,
    }
}

fn handle_notification(
    state: &Mutex<ServerState>,
    connection: &Connection,
    not: &lsp_server::Notification,
) {
    match not.method.as_str() {
        DidOpenTextDocument::METHOD => {
            if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(not.params.clone()) {
                if let Ok(mut s) = state.lock() {
                    s.store.open(&params.text_document.uri, params.text_document.text);
                }
            }
        }
        DidChangeTextDocument::METHOD => {
            if let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(not.params.clone()) {
                if let Ok(mut s) = state.lock() {
                    for change in params.content_changes {
                        s.store.update(&params.text_document.uri, &change.text);
                    }
                    // Debounce diagnostics
                    let now = std::time::Instant::now();
                    if now.duration_since(s.debounce_timer) > std::time::Duration::from_millis(300) {
                        s.debounce_timer = now;
                        let uri = params.text_document.uri.clone();
                        let diags = handlers::handle_diagnostics(&mut s.store, &uri);
                        publish_diagnostics(connection, &uri, diags);
                    }
                }
            }
        }
        DidCloseTextDocument::METHOD => {
            if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(not.params.clone()) {
                if let Ok(mut s) = state.lock() {
                    s.store.close(&params.text_document.uri);
                }
            }
        }
        _ => {}
    }
}

fn publish_diagnostics(connection: &Connection, uri: &lsp_types::Url, diags: Vec<lsp_types::Diagnostic>) {
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics: diags,
        version: None,
    };
    let not = lsp_server::Notification {
        method: PublishDiagnostics::METHOD.to_string(),
        params: serde_json::to_value(&params).unwrap(),
    };
    let _ = connection.sender.send(Message::Notification(not));
}

fn scan_workspace_files(root: &str) -> Vec<String> {
    let mut files = Vec::new();
    let src_dir = Path::new(root).join("src");
    let dir = if src_dir.is_dir() { &src_dir } else { Path::new(root) };
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == ".git" || name == "node_modules" || name == "out" || name == "target" { continue; }
                    files.extend(scan_workspace_files(&path.display().to_string()));
                }
            } else if path.extension().and_then(|e| e.to_str()) == Some("wrm") {
                if let Ok(rel) = path.strip_prefix(dir) {
                    files.push(rel.display().to_string().replace('\\', "/").trim_end_matches(".wrm").to_string());
                }
            }
        }
    }
    files
}
