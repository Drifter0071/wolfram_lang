#![allow(deprecated)]
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod generator;
pub mod roblox_config;
pub mod analyze;
pub mod lsp;

use logos::Logos;
use lexer::Token;
use parser::Parser;
use generator::generate;
use roblox_config::RobloxProjectConfig;

/// Transpile Wolfram source code → Luau source code.
/// `source_code` — full Wolfram source string.
/// `file_path`   — display name used in error messages (e.g. "main.wrm").
pub fn transpile(source_code: &str, file_path: &str) -> Result<String, String> {
    transpile_inner(source_code, file_path, false, None, None)
}

/// Transpile Wolfram → Luau in Roblox mode.
/// `config` — parsed `wolfram.toml` roblox section.
/// `importing_file` — path of the file being compiled (for import resolution).
pub fn transpile_roblox(
    source_code: &str,
    file_path: &str,
    config: &RobloxProjectConfig,
    importing_file: &str,
) -> Result<String, String> {
    transpile_inner(source_code, file_path, true, Some(config), Some(importing_file))
}

fn transpile_inner(
    source_code: &str,
    file_path: &str,
    roblox_mode: bool,
    config: Option<&RobloxProjectConfig>,
    importing_file: Option<&str>,
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
        Ok(ast) => Ok(generate(&ast, roblox_mode, config, importing_file)),
        Err(e) => Err(format!("Parse error in '{}': {}", file_path, e)),
    }
}
