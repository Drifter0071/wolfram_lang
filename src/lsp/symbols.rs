use crate::ast::Stmt;
use crate::lsp::store::DocumentStore;
use lsp_types::*;

// ===== Document Symbols =====

pub fn handle_document_symbols(
    store: &mut DocumentStore,
    params: DocumentSymbolParams,
) -> Option<Vec<DocumentSymbol>> {
    store.reparse_if_dirty(&params.text_document.uri);
    let doc = store.get(&params.text_document.uri)?;
    Some(extract_document_symbols(&doc.ast, &doc.source))
}

fn extract_document_symbols(ast: &[Stmt], source: &str) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    for stmt in ast {
        if let Some(sym) = stmt_to_document_symbol(stmt, source) {
            symbols.push(sym);
        }
    }
    symbols
}

fn stmt_to_document_symbol(stmt: &Stmt, source: &str) -> Option<DocumentSymbol> {
    match stmt {
        Stmt::FuncDef {
            name, params, block, span, ..
        } => {
            let range = byte_span_to_range(source, span);
            let selection_range = name_range(source, name, span.start)?;
            let detail = format!("def ({})", params.join(", "));
            let children = extract_document_symbols(block, source);
            Some(DocumentSymbol {
                name: name.clone(),
                detail: Some(detail),
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: None,
                range,
                selection_range,
                children: if children.is_empty() {
                    None
                } else {
                    Some(children)
                },
            })
        }
        Stmt::ClassDef {
            name, body, span, ..
        } => {
            let range = byte_span_to_range(source, span);
            let selection_range = name_range(source, name, span.start)?;
            let children = extract_document_symbols(body, source);
            Some(DocumentSymbol {
                name: name.clone(),
                detail: None,
                kind: SymbolKind::CLASS,
                tags: None,
                deprecated: None,
                range,
                selection_range,
                children: if children.is_empty() {
                    None
                } else {
                    Some(children)
                },
            })
        }
        Stmt::EnumDef {
            name, span, ..
        } => {
            let range = byte_span_to_range(source, span);
            let selection_range = name_range(source, name, span.start)?;
            Some(DocumentSymbol {
                name: name.clone(),
                detail: None,
                kind: SymbolKind::ENUM,
                tags: None,
                deprecated: None,
                range,
                selection_range,
                children: None,
            })
        }
        Stmt::StructDef {
            name, span, ..
        } => {
            let range = byte_span_to_range(source, span);
            let selection_range = name_range(source, name, span.start)?;
            Some(DocumentSymbol {
                name: name.clone(),
                detail: None,
                kind: SymbolKind::STRUCT,
                tags: None,
                deprecated: None,
                range,
                selection_range,
                children: None,
            })
        }
        Stmt::Local { name, span, .. } => {
            let range = byte_span_to_range(source, span);
            let selection_range = name_range(source, name, span.start)?;
            Some(DocumentSymbol {
                name: name.clone(),
                detail: None,
                kind: SymbolKind::VARIABLE,
                tags: None,
                deprecated: None,
                range,
                selection_range,
                children: None,
            })
        }
        Stmt::DecoratedStmt { stmt, span, .. } => {
            // Use the inner stmt's info but the outer decorator's span
            let mut sym = stmt_to_document_symbol(stmt, source)?;
            sym.range = byte_span_to_range(source, span);
            Some(sym)
        }
        _ => None,
    }
}

// Workspace symbols — search across all open documents

pub fn handle_workspace_symbols(
    store: &mut DocumentStore,
    params: WorkspaceSymbolParams,
) -> Option<Vec<SymbolInformation>> {
    let query = params.query.to_lowercase();
    if query.is_empty() {
        return Some(Vec::new());
    }

    let mut results = Vec::new();

    for doc in store.iter() {
        for stmt in &doc.ast {
            if let Some(info) = stmt_to_symbol_information(stmt, &doc.source, &doc.uri, &query) {
                results.push(info);
            }
        }
    }

    // Limit results
    results.truncate(50);
    Some(results)
}

fn stmt_to_symbol_information(
    stmt: &Stmt,
    source: &str,
    uri: &Url,
    query: &str,
) -> Option<SymbolInformation> {
    match stmt {
        Stmt::FuncDef { name, span, .. } => {
            if !name.to_lowercase().contains(query) {
                return None;
            }
            let range = byte_span_to_range(source, span);
            Some(SymbolInformation {
                name: name.clone(),
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range,
                },
                container_name: None,
            })
        }
        Stmt::ClassDef { name, span, .. } => {
            if !name.to_lowercase().contains(query) {
                return None;
            }
            let range = byte_span_to_range(source, span);
            Some(SymbolInformation {
                name: name.clone(),
                kind: SymbolKind::CLASS,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range,
                },
                container_name: None,
            })
        }
        Stmt::EnumDef { name, span, .. } => {
            if !name.to_lowercase().contains(query) {
                return None;
            }
            let range = byte_span_to_range(source, span);
            Some(SymbolInformation {
                name: name.clone(),
                kind: SymbolKind::ENUM,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range,
                },
                container_name: None,
            })
        }
        Stmt::StructDef { name, span, .. } => {
            if !name.to_lowercase().contains(query) {
                return None;
            }
            let range = byte_span_to_range(source, span);
            Some(SymbolInformation {
                name: name.clone(),
                kind: SymbolKind::STRUCT,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range,
                },
                container_name: None,
            })
        }
        Stmt::Local { name, span, .. } => {
            if !name.to_lowercase().contains(query) {
                return None;
            }
            let range = byte_span_to_range(source, span);
            Some(SymbolInformation {
                name: name.clone(),
                kind: SymbolKind::VARIABLE,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range,
                },
                container_name: None,
            })
        }
        _ => None,
    }
}

// Helpers

fn byte_span_to_range(source: &str, span: &crate::ast::Span) -> Range {
    let start = byte_pos_to_position(source, span.start);
    let end = byte_pos_to_position(source, span.end);
    Range { start, end }
}

fn name_range(source: &str, name: &str, search_start: usize) -> Option<Range> {
    let window = &source[search_start.min(source.len())..];
    let pos = window.find(name)?;
    let abs = search_start + pos;
    let start = byte_pos_to_position(source, abs);
    let end = byte_pos_to_position(source, abs + name.len());
    Some(Range { start, end })
}

fn byte_pos_to_position(source: &str, byte_pos: usize) -> Position {
    let byte_pos = byte_pos.min(source.len());
    let prefix = &source[..byte_pos];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() as u32;
    let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = prefix[last_newline..].chars().count() as u32;
    Position { line, character }
}
