use serde::Serialize;

use crate::ast::{Span, Stmt};
use crate::lexer::Token;
use crate::parser::Parser;
use logos::Logos;

#[derive(Debug, Serialize)]
pub struct AnalyzeResult {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<ImportInfo>,
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Serialize)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub access: String,
    pub location: SourceLocation,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Serialize)]
pub struct ImportInfo {
    pub path: String,
    pub alias: String,
}

fn span_to_location(span: &Span, source: &str) -> SourceLocation {
    let start = span.start.min(source.len());
    let end = span.end.min(source.len());
    let prefix = &source[..start];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() + 1;
    let col = start.saturating_sub(prefix.rfind('\n').map(|i| i + 1).unwrap_or(0)) + 1;

    let end_prefix = &source[..end];
    let end_line = end_prefix.bytes().filter(|&b| b == b'\n').count() + 1;
    let end_col = end.saturating_sub(end_prefix.rfind('\n').map(|i| i + 1).unwrap_or(0)) + 1;

    SourceLocation {
        line,
        column: col,
        end_line,
        end_column: end_col,
    }
}

fn extract_symbols(stmts: &[Stmt], source: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::Local {
                name,
                access,
                span,
                ..
            } => {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: "variable".into(),
                    access: access.clone(),
                    location: span_to_location(span, source),
                    params: vec![],
                    fields: vec![],
                });
            }
            Stmt::FuncDef {
                name,
                params,
                access,
                span,
                ..
            } => {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: "function".into(),
                    access: access.clone(),
                    location: span_to_location(span, source),
                    params: params.clone(),
                    fields: vec![],
                });
            }
            Stmt::ClassDef {
                name,
                access,
                span,
                ..
            } => {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: "class".into(),
                    access: access.clone(),
                    location: span_to_location(span, source),
                    params: vec![],
                    fields: vec![],
                });
            }
            Stmt::EnumDef {
                name,
                variants,
                access,
                span,
                ..
            } => {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: "enum".into(),
                    access: access.clone(),
                    location: span_to_location(span, source),
                    params: vec![],
                    fields: variants.clone(),
                });
            }
            Stmt::StructDef {
                name,
                fields,
                access,
                span,
                ..
            } => {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: "struct".into(),
                    access: access.clone(),
                    location: span_to_location(span, source),
                    params: vec![],
                    fields: fields.clone(),
                });
            }
            _ => {}
        }
    }
    symbols
}

fn extract_imports(stmts: &[Stmt]) -> Vec<ImportInfo> {
    stmts
        .iter()
        .filter_map(|stmt| {
            if let Stmt::Import { path, alias, .. } = stmt {
                Some(ImportInfo {
                    path: path.clone(),
                    alias: alias.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn analyze(source: &str, _file_path: &str) -> AnalyzeResult {
    let mut tokens = Vec::new();
    let mut spans = Vec::new();
    for (res, span) in Token::lexer(source).spanned() {
        if let Ok(tok) = res {
            tokens.push(tok);
            spans.push(span.start);
        }
    }

    let mut parser = Parser::new(tokens, spans, source);
    match parser.parse_program() {
        Ok(stmts) => AnalyzeResult {
            ok: true,
            diagnostics: vec![],
            symbols: extract_symbols(&stmts, source),
            imports: extract_imports(&stmts),
        },
        Err(e) => AnalyzeResult {
            ok: false,
            diagnostics: vec![parse_error_to_diagnostic(&e, source)],
            symbols: vec![],
            imports: vec![],
        },
    }
}

fn parse_error_to_diagnostic(error: &str, _source: &str) -> Diagnostic {
    let mut line = 1;
    let mut column = 1;
    if let Some(pos_part) = error.strip_prefix("line ") {
        if let Some(comma) = pos_part.find(", column ") {
            if let Ok(l) = pos_part[..comma].parse::<usize>() {
                line = l;
            }
            let after_col = &pos_part[comma + 9..];
            if let Some(colon) = after_col.find(':') {
                if let Ok(c) = after_col[..colon].parse::<usize>() {
                    column = c;
                }
            }
        }
    }
    Diagnostic {
        line,
        column,
        message: error.to_string(),
        severity: "error".into(),
    }
}
