#![allow(deprecated)]
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod generator;
pub mod roblox_config;
pub mod rojo_config;
pub mod roblox_context;
pub mod analyze;
pub mod scope;
pub mod typeck;
pub mod lsp;

use logos::Logos;
use lexer::Token;
use parser::Parser;
use generator::generate;
use roblox_config::RobloxProjectConfig;
use rojo_config::RojoPathMapping;
use scope::ScopeAnalysis;

pub fn check_scope(source_code: &str, file_path: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut spans = Vec::new();
    for (res, span) in Token::lexer(source_code).spanned() {
        if let Ok(tok) = res {
            tokens.push(tok);
            spans.push(span.start);
        }
    }
    let mut parser = Parser::new(tokens, spans, source_code);
    match parser.parse_program() {
        Ok(ast) => ScopeAnalysis::analyze(&ast, source_code, file_path),
        Err(_) => vec![],
    }
}

pub fn check_types(source_code: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut spans = Vec::new();
    for (res, span) in Token::lexer(source_code).spanned() {
        if let Ok(tok) = res {
            tokens.push(tok);
            spans.push(span.start);
        }
    }
    let mut parser = Parser::new(tokens, spans, source_code);
    match parser.parse_program() {
        Ok(ast) => {
            let tc = typeck::check_types(&ast);
            let mut msgs: Vec<String> = tc.errors.into_iter()
                .map(|e| format!("error: {}", e)).collect();
            msgs.extend(tc.warnings.into_iter()
                .map(|w| format!("warning: {}", w)));
            msgs
        }
        Err(_) => vec![],
    }
}

/// Transpile Wolfram source code → Luau source code.
/// `source_code` — full Wolfram source string.
/// `file_path`   — display name used in error messages (e.g. "main.wrm").
pub fn transpile(source_code: &str, file_path: &str) -> Result<String, String> {
    transpile_inner(source_code, file_path, false, None, None, None, "out")
}

/// Transpile Wolfram → Luau in Roblox mode.
/// `config` — parsed `wolfram.toml` roblox section.
/// `importing_file` — path of the file being compiled (for import resolution).
/// `rojo_mappings` — parsed from `default.project.json` if present.
/// `out_dir` — output directory (e.g. "out").
pub fn transpile_roblox(
    source_code: &str,
    file_path: &str,
    config: Option<&RobloxProjectConfig>,
    importing_file: &str,
    rojo_mappings: Option<&[RojoPathMapping]>,
    out_dir: &str,
) -> Result<String, String> {
    transpile_inner(source_code, file_path, true, config, Some(importing_file), rojo_mappings, out_dir)
}

fn transpile_inner(
    source_code: &str,
    file_path: &str,
    roblox_mode: bool,
    config: Option<&RobloxProjectConfig>,
    importing_file: Option<&str>,
    rojo_mappings: Option<&[RojoPathMapping]>,
    out_dir: &str,
) -> Result<String, String> {
    let mut tokens = Vec::new();
    let mut spans = Vec::new();
    for (res, span) in Token::lexer(source_code).spanned() {
        if let Ok(tok) = res {
            tokens.push(tok);
            spans.push(span.start);
        }
    }

    let mut parser = Parser::new(tokens, spans, source_code);
    match parser.parse_program() {
        Ok(ast) => Ok(generate(&ast, roblox_mode, config, importing_file, rojo_mappings, out_dir)),
        Err(e) => Err(format!("Parse error in '{}': {}", file_path, e)),
    }
}
