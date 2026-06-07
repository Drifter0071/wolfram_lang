pub mod analyze;
pub mod ast;
pub mod constants;
pub mod errors;
pub mod generator;
pub mod lexer;
pub mod lsp;
pub mod luau_checker;
pub mod parser;
pub mod roblox_api;
pub mod roblox_config;
pub mod roblox_context;
pub mod rojo_config;
pub mod scope;
pub mod typeck;
pub mod types;

#[cfg(test)]
mod tests;

use crate::ast::Stmt;
use generator::generate;
use lexer::Token;
use logos::Logos;
use parser::Parser;
use roblox_config::{DeploymentEntry, RobloxProjectConfig};
use rojo_config::RojoPathMapping;
use scope::ScopeAnalysis;

pub const DEFAULT_OUT_DIR: &str = "out";

pub fn tokenize_and_parse(source_code: &str) -> Result<Vec<Stmt>, String> {
    let mut tokens = Vec::new();
    let mut spans = Vec::new();
    for (res, span) in Token::lexer(source_code).spanned() {
        if let Ok(tok) = res {
            if matches!(tok, Token::Comment(_)) {
                continue;
            }
            tokens.push(tok);
            spans.push(span.start);
        }
    }
    let mut parser = Parser::new(tokens, spans, source_code);
    parser.parse_program()
}

pub fn check_scope(source_code: &str, file_path: &str) -> Vec<String> {
    match tokenize_and_parse(source_code) {
        Ok(ast) => ScopeAnalysis::analyze(&ast, source_code, file_path),
        Err(_) => vec![],
    }
}

pub fn check_types(source_code: &str) -> Vec<String> {
    match tokenize_and_parse(source_code) {
        Ok(ast) => {
            let tc = typeck::check_types(&ast);
            let mut msgs: Vec<String> = tc
                .errors
                .into_iter()
                .map(|e| format!("error: {}", e))
                .collect();
            msgs.extend(tc.warnings.into_iter().map(|w| format!("warning: {}", w)));
            msgs
        }
        Err(_) => vec![],
    }
}

/// Transpile Wolfram source code → Luau source code.
/// `source_code` — full Wolfram source string.
/// `file_path`   — display name used in error messages (e.g. "main.wrm").
pub fn transpile(source_code: &str, file_path: &str) -> Result<String, String> {
    transpile_inner(
        source_code,
        file_path,
        false,
        None,
        None,
        None,
        &[],
        DEFAULT_OUT_DIR,
    )
}

/// Transpile Wolfram → Luau in Roblox mode.
/// `config` — parsed `wolfram.toml` roblox section.
/// `importing_file` — path of the file being compiled (for import resolution).
/// `rojo_mappings` — parsed from `default.project.json` if present.
/// `deployments` — normalized deployment table from `wolfram.toml` [deployment].
/// `out_dir` — output directory (e.g. "out").
pub fn transpile_roblox(
    source_code: &str,
    file_path: &str,
    config: Option<&RobloxProjectConfig>,
    importing_file: &str,
    rojo_mappings: Option<&[RojoPathMapping]>,
    deployments: &[DeploymentEntry],
    out_dir: &str,
) -> Result<String, String> {
    transpile_inner(
        source_code,
        file_path,
        true,
        config,
        Some(importing_file),
        rojo_mappings,
        deployments,
        out_dir,
    )
}

fn transpile_inner(
    source_code: &str,
    file_path: &str,
    roblox_mode: bool,
    config: Option<&RobloxProjectConfig>,
    importing_file: Option<&str>,
    rojo_mappings: Option<&[RojoPathMapping]>,
    deployments: &[DeploymentEntry],
    out_dir: &str,
) -> Result<String, String> {
    let ast = match tokenize_and_parse(source_code) {
        Ok(ast) => ast,
        Err(e) => return Err(format!("Parse error in '{}': {}", file_path, e)),
    };

    // Run the Luau Validation Engine before generation
    let script_type = roblox_context::ScriptType::from_filename(file_path);
    let checker_config = luau_checker::CheckConfig {
        script_type,
        file_path: file_path.to_string(),
        source: source_code.to_string(),
        check_roblox_api: true,
        check_circular_deps: roblox_mode,
        check_nil_safety: true,
        check_patterns: true,
        dependency_graph: None,
    };
    let validation = luau_checker::validate(&ast, checker_config);

    // If errors exist, halt the pipeline
    if validation.has_errors() {
        let err_msgs: Vec<String> = validation
            .errors
            .iter()
            .map(|d| format!("  line {}:{} — {}", d.line, d.column, d.message))
            .collect();
        return Err(format!(
            "Validation failed in '{}' with {} error(s):\n{}",
            file_path,
            validation.errors.len(),
            err_msgs.join("\n")
        ));
    }

    // Generate Luau output
    let mut output = generate(
        &ast,
        roblox_mode,
        config,
        importing_file,
        rojo_mappings,
        deployments,
        out_dir,
    );

    // Prepend validation warnings as comments (for developer visibility)
    if !validation.warnings.is_empty() {
        let warn_header = format!(
            "-- Luau Checker: {} warning(s) in '{}'\n",
            validation.warnings.len(),
            file_path
        );
        let warn_lines: Vec<String> = validation
            .warnings
            .iter()
            .map(|d| format!("--   line {}:{} — {}\n", d.line, d.column, d.message))
            .collect();
        let warn_block = format!("{}{}", warn_header, warn_lines.concat());
        output.insert_str(0, &warn_block);
    }

    Ok(output)
}

#[derive(Debug, Clone)]
pub struct LineMapEntry {
    pub wrm_line: usize,
    pub wrm_col: usize,
    pub luau_line: usize,
    pub luau_col: usize,
}

impl LineMapEntry {
    pub fn new(wrm_line: usize, wrm_col: usize, luau_line: usize, luau_col: usize) -> Self {
        Self {
            wrm_line,
            wrm_col,
            luau_line,
            luau_col,
        }
    }
}

/// Transpile Result with line mapping for LSP diagnostics.
#[derive(Debug, Clone)]
pub struct TranspileResult {
    pub luau: String,
    pub line_map: Vec<LineMapEntry>,
    pub warnings: Vec<analyze::Diagnostic>,
}

/// Transpile and return line map for LSP integration.
/// Same validation pipeline as `transpile`, but returns structured result
/// with bidirectional line mapping for error translation.
pub fn transpile_with_cache(
    source_code: &str,
    file_path: &str,
    roblox_mode: bool,
    importing_file: Option<&str>,
) -> TranspileResult {
    let ast = match tokenize_and_parse(source_code) {
        Ok(ast) => ast,
        Err(e) => {
            return TranspileResult {
                luau: format!("-- Parse error: {}", e),
                line_map: Vec::new(),
                warnings: Vec::new(),
            };
        }
    };

    // Run validation
    let script_type = roblox_context::ScriptType::from_filename(file_path);
    let checker_config = luau_checker::CheckConfig {
        script_type,
        file_path: file_path.to_string(),
        source: source_code.to_string(),
        check_roblox_api: true,
        check_circular_deps: roblox_mode,
        check_nil_safety: true,
        check_patterns: true,
        dependency_graph: None,
    };
    let validation = luau_checker::validate(&ast, checker_config);

    if validation.has_errors() {
        return TranspileResult {
            luau: String::new(),
            line_map: Vec::new(),
            warnings: validation.errors,
        };
    }

    let luau = generate(
        &ast,
        roblox_mode,
        None,
        importing_file,
        None,
        &[],
        DEFAULT_OUT_DIR,
    );

    // Build approximate line map: each wolfram source line maps to the same Luau line
    // (accurate mapping requires tracking during generation)
    let mut line_map = Vec::new();
    let wrm_lines: Vec<&str> = source_code.lines().collect();
    let luau_lines: Vec<&str> = luau.lines().collect();
    let max_lines = wrm_lines.len().max(luau_lines.len());
    for i in 0..max_lines {
        line_map.push(LineMapEntry {
            wrm_line: i + 1,
            wrm_col: 1,
            luau_line: (i + 1).min(luau_lines.len()),
            luau_col: 1,
        });
    }

    TranspileResult {
        luau,
        line_map,
        warnings: validation.warnings,
    }
}
