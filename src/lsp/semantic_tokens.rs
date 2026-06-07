use crate::ast::{Expr, Stmt};
use crate::lsp::store::DocumentStore;
use lsp_types::*;

// Legend: token type index
// 0 namespace, 1 type(enum/struct), 2 class, 3 function, 4 property, 5 method, 6 variable,
// 7 parameter, 8 keyword, 9 string, 10 number, 11 comment, 12 operator, 13 decorator
const TYPE_KEYWORD: u32 = 8;
const TYPE_VARIABLE: u32 = 6;
const TYPE_FUNCTION: u32 = 3;
const TYPE_METHOD: u32 = 5;
const TYPE_CLASS: u32 = 2;
const TYPE_PARAMETER: u32 = 7;
const TYPE_STRING: u32 = 9;
const TYPE_NUMBER: u32 = 10;
const TYPE_COMMENT: u32 = 11;
const TYPE_DECORATOR: u32 = 13;
const TYPE_ENUM: u32 = 1;
const TYPE_STRUCT: u32 = 1;

pub fn handle_semantic_tokens(
    store: &mut DocumentStore,
    params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
    store.reparse_if_dirty(&params.text_document.uri);
    let doc = store.get(&params.text_document.uri)?;
    let tokens = extract_semantic_tokens(&doc.ast, &doc.source);

    // Sort by position for proper delta encoding
    let mut tokens = tokens;
    tokens.sort_by_key(|t| (t.line, t.character));

    let data = encode_semantic_tokens(&tokens);
    Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data,
    }))
}

#[derive(Debug, Clone)]
struct InternalToken {
    line: u32,
    character: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
}

fn encode_semantic_tokens(tokens: &[InternalToken]) -> Vec<SemanticToken> {
    let mut encoded = Vec::with_capacity(tokens.len());
    let mut prev_line: u32 = 0;
    let mut prev_char: u32 = 0;

    for t in tokens {
        let delta_line = t.line - prev_line;
        let delta_char = if delta_line == 0 {
            t.character - prev_char
        } else {
            t.character
        };
        encoded.push(SemanticToken {
            delta_line,
            delta_start: delta_char,
            length: t.length,
            token_type: t.token_type,
            token_modifiers_bitset: t.modifiers,
        });
        prev_line = t.line;
        prev_char = t.character;
    }
    encoded
}

fn extract_semantic_tokens(ast: &[Stmt], source: &str) -> Vec<InternalToken> {
    let mut tokens = Vec::new();
    walk_stmts(ast, source, &mut tokens, false);
    // Also extract comment tokens
    tokens.extend(extract_comments(source));
    tokens
}

fn extract_comments(source: &str) -> Vec<InternalToken> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#") {
            let col = line.find('#').unwrap_or(0);
            tokens.push(InternalToken {
                line: i as u32,
                character: col as u32,
                length: trimmed.len() as u32,
                token_type: TYPE_COMMENT,
                modifiers: 0,
            });
        }
    }
    tokens
}

fn walk_stmts(stmts: &[Stmt], source: &str, tokens: &mut Vec<InternalToken>, inside_class: bool) {
    for stmt in stmts {
        walk_stmt(stmt, source, tokens, inside_class);
    }
}

fn walk_stmt(stmt: &Stmt, source: &str, tokens: &mut Vec<InternalToken>, inside_class: bool) {
    match stmt {
        Stmt::Local { name, value, span, .. } => {
            // Variable name
            if let Some(pos) = find_token_in_source(source, name, span.start) {
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: name.len() as u32,
                    token_type: TYPE_VARIABLE,
                    modifiers: 0,
                });
            }
            if let Some(val) = value {
                walk_expr(val, source, span.start, tokens);
            }
        }
        Stmt::Assign { target, value, span, .. } => {
            walk_expr(target, source, span.start, tokens);
            walk_expr(value, source, span.start, tokens);
        }
        Stmt::Return { value, span, .. } => {
            tokens.push(make_keyword_token(source, span, "return"));
            if let Some(val) = value {
                walk_expr(val, source, span.start, tokens);
            }
        }
        Stmt::If { cond, then_block, else_if_blocks, else_block, span } => {
            tokens.push(make_keyword_token(source, span, "if"));
            walk_expr(cond, source, span.start, tokens);
            walk_stmts(then_block, source, tokens, inside_class);
            for (_cond, block) in else_if_blocks {
                walk_stmts(block, source, tokens, inside_class);
            }
            if let Some(block) = else_block {
                walk_stmts(block, source, tokens, inside_class);
            }
        }
        Stmt::While { cond, block, span } => {
            tokens.push(make_keyword_token(source, span, "while"));
            walk_expr(cond, source, span.start, tokens);
            walk_stmts(block, source, tokens, inside_class);
        }
        Stmt::For { var, iter, block, span } => {
            tokens.push(make_keyword_token(source, span, "for"));
            if let Some(pos) = find_token_in_source(source, var, span.start) {
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: var.len() as u32,
                    token_type: TYPE_VARIABLE,
                    modifiers: 0,
                });
            }
            walk_expr(iter, source, span.start, tokens);
            walk_stmts(block, source, tokens, inside_class);
        }
        Stmt::FuncDef { name, params, param_types: _, param_defaults, block, access, is_async, span } => {
            // "public/private" keyword
            if access == "public" || access == "private" {
                tokens.push(make_keyword_token(source, span, access));
            }
            // "def" keyword
            tokens.push(make_keyword_token(source, span, "def"));
            if *is_async {
                tokens.push(make_keyword_token(source, span, "async"));
            }
            // Function name
            if let Some(pos) = find_token_in_source(source, name, span.start) {
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: name.len() as u32,
                    token_type: if inside_class { TYPE_METHOD } else { TYPE_FUNCTION },
                    modifiers: 0,
                });
            }
            // Parameters
            for (i, param) in params.iter().enumerate() {
                if let Some(pos) = find_token_in_source(source, param, span.start) {
                    tokens.push(InternalToken {
                        line: pos.line,
                        character: pos.col,
                        length: param.len() as u32,
                        token_type: TYPE_PARAMETER,
                        modifiers: 0,
                    });
                }
                if let Some(default) = param_defaults.get(i).and_then(|d| d.as_ref()) {
                    walk_expr(default, source, span.start, tokens);
                }
            }
            walk_stmts(block, source, tokens, false);
        }
        Stmt::ClassDef { name, body, access, span } => {
            if access == "public" || access == "private" {
                tokens.push(make_keyword_token(source, span, access));
            }
            tokens.push(make_keyword_token(source, span, "class"));
            if let Some(pos) = find_token_in_source(source, name, span.start) {
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: name.len() as u32,
                    token_type: TYPE_CLASS,
                    modifiers: 0,
                });
            }
            walk_stmts(body, source, tokens, true);
        }
        Stmt::ExprStmt { expr, span } => {
            walk_expr(expr, source, span.start, tokens);
        }
        Stmt::EnumDef { name, variants, access, span } => {
            if access == "public" || access == "private" {
                tokens.push(make_keyword_token(source, span, access));
            }
            tokens.push(make_keyword_token(source, span, "enum"));
            if let Some(pos) = find_token_in_source(source, name, span.start) {
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: name.len() as u32,
                    token_type: TYPE_ENUM,
                    modifiers: 0,
                });
            }
            for variant in variants {
                if let Some(pos) = find_token_in_source(source, variant, span.start) {
                    tokens.push(InternalToken {
                        line: pos.line,
                        character: pos.col,
                        length: variant.len() as u32,
                        token_type: TYPE_VARIABLE,
                        modifiers: 0,
                    });
                }
            }
        }
        Stmt::StructDef { name, fields, access, span } => {
            if access == "public" || access == "private" {
                tokens.push(make_keyword_token(source, span, access));
            }
            tokens.push(make_keyword_token(source, span, "struct"));
            if let Some(pos) = find_token_in_source(source, name, span.start) {
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: name.len() as u32,
                    token_type: TYPE_STRUCT,
                    modifiers: 0,
                });
            }
            for field in fields {
                if let Some(pos) = find_token_in_source(source, &field.name, span.start) {
                    tokens.push(InternalToken {
                        line: pos.line,
                        character: pos.col,
                        length: field.name.len() as u32,
                        token_type: TYPE_VARIABLE,
                        modifiers: 0,
                    });
                }
            }
        }
        Stmt::Import { path, alias, span } => {
            tokens.push(make_keyword_token(source, span, "import"));
            let token_name = if alias.is_empty() { path } else { alias };
            if let Some(pos) = find_token_in_source(source, token_name, span.start) {
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: token_name.len() as u32,
                    token_type: TYPE_VARIABLE,
                    modifiers: 0,
                });
            }
        }
        Stmt::Break { span } => {
            tokens.push(make_keyword_token(source, span, "break"));
        }
        Stmt::Continue { span } => {
            tokens.push(make_keyword_token(source, span, "continue"));
        }
        Stmt::TryCatch { try_block, catch_clauses, finally_block, span } => {
            tokens.push(make_keyword_token(source, span, "try"));
            walk_stmts(try_block, source, tokens, inside_class);
            for (_var, _ty, block) in catch_clauses {
                tokens.push(make_keyword_token(source, span, "except"));
                walk_stmts(block, source, tokens, inside_class);
            }
            if let Some(block) = finally_block {
                tokens.push(make_keyword_token(source, span, "finally"));
                walk_stmts(block, source, tokens, inside_class);
            }
        }
        Stmt::DecoratedStmt { decorators, stmt, span } => {
            for decorator in decorators {
                if let Some(pos) = find_token_in_source(source, &format!("@{}", decorator), span.start) {
                    tokens.push(InternalToken {
                        line: pos.line,
                        character: pos.col,
                        length: (decorator.len() + 1) as u32,
                        token_type: TYPE_DECORATOR,
                        modifiers: 0,
                    });
                }
            }
            walk_stmt(stmt, source, tokens, inside_class);
        }
    }
}

fn walk_expr(expr: &Expr, source: &str, search_start: usize, tokens: &mut Vec<InternalToken>) {
    match expr {
        Expr::Number(_) => {
            if let Some(pos) = find_numeric_in_source(source, search_start) {
                let num_len = source[search_start..]
                    .find(|c: char| !c.is_ascii_digit() && c != '.')
                    .unwrap_or(source.len() - search_start);
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: num_len as u32,
                    token_type: TYPE_NUMBER,
                    modifiers: 0,
                });
            }
        }
        Expr::Str(s) => {
            let search_str = format!("{:?}", s);
            if let Some(pos) = find_token_in_source(source, &search_str, search_start) {
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: search_str.len() as u32,
                    token_type: TYPE_STRING,
                    modifiers: 0,
                });
            }
        }
        Expr::FString(_) => {
            if let Some(pos) = find_fstring_in_source(source, search_start) {
                let end = source[search_start..]
                    .find('"')
                    .map(|e| e + 1)
                    .unwrap_or(10);
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: end as u32,
                    token_type: TYPE_STRING,
                    modifiers: 0,
                });
            }
        }
        Expr::Ident(name) => {
            if let Some(pos) = find_token_in_source(source, name, search_start) {
                // Skip keywords
                if !is_keyword(name) {
                    tokens.push(InternalToken {
                        line: pos.line,
                        character: pos.col,
                        length: name.len() as u32,
                        token_type: TYPE_VARIABLE,
                        modifiers: 0,
                    });
                }
            }
        }
        Expr::Call { func, args } => {
            if let Some(pos) = find_token_in_source(source, func, search_start) {
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: func.len() as u32,
                    token_type: TYPE_FUNCTION,
                    modifiers: 0,
                });
            }
            for arg in args {
                walk_expr(arg, source, search_start, tokens);
            }
        }
        Expr::MethodCall { obj, field, args, .. } => {
            walk_expr(obj, source, search_start, tokens);
            if let Some(pos) = find_token_in_source(source, field, search_start) {
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: field.len() as u32,
                    token_type: TYPE_METHOD,
                    modifiers: 0,
                });
            }
            for arg in args {
                walk_expr(arg, source, search_start, tokens);
            }
        }
        Expr::Member { obj, field, .. } => {
            walk_expr(obj, source, search_start, tokens);
            if let Some(pos) = find_token_in_source(source, field, search_start) {
                tokens.push(InternalToken {
                    line: pos.line,
                    character: pos.col,
                    length: field.len() as u32,
                    token_type: TYPE_METHOD,
                    modifiers: 0,
                });
            }
        }
        Expr::Index { obj, index } => {
            walk_expr(obj, source, search_start, tokens);
            walk_expr(index, source, search_start, tokens);
        }
        Expr::Binary { left, right, .. } => {
            walk_expr(left, source, search_start, tokens);
            walk_expr(right, source, search_start, tokens);
        }
        Expr::Logical { left, right, .. } => {
            walk_expr(left, source, search_start, tokens);
            walk_expr(right, source, search_start, tokens);
        }
        Expr::UnaryMinus(inner) | Expr::Not(inner) | Expr::Grouping(inner) => {
            walk_expr(inner, source, search_start, tokens);
        }
        Expr::Ternary { cond, then_expr, else_expr } => {
            walk_expr(cond, source, search_start, tokens);
            walk_expr(then_expr, source, search_start, tokens);
            walk_expr(else_expr, source, search_start, tokens);
        }
        Expr::Array(elems) => {
            for elem in elems {
                walk_expr(elem, source, search_start, tokens);
            }
        }
        Expr::Table(fields) => {
            for field in fields {
                match field {
                    crate::ast::TableField::Pair { key, value } => {
                        walk_expr(key, source, search_start, tokens);
                        walk_expr(value, source, search_start, tokens);
                    }
                    crate::ast::TableField::Value(v) => {
                        walk_expr(v, source, search_start, tokens);
                    }
                }
            }
        }
        Expr::Function { params, block } => {
            for param in params {
                if let Some(pos) = find_token_in_source(source, param, search_start) {
                    tokens.push(InternalToken {
                        line: pos.line,
                        character: pos.col,
                        length: param.len() as u32,
                        token_type: TYPE_PARAMETER,
                        modifiers: 0,
                    });
                }
            }
            walk_stmts(block, source, tokens, false);
        }
        Expr::ListComp { elt, generators } => {
            walk_expr(elt, source, search_start, tokens);
            for gen in generators {
                if let Some(pos) = find_token_in_source(source, &gen.var, search_start) {
                    tokens.push(InternalToken {
                        line: pos.line,
                        character: pos.col,
                        length: gen.var.len() as u32,
                        token_type: TYPE_VARIABLE,
                        modifiers: 0,
                    });
                }
                walk_expr(&gen.iter, source, search_start, tokens);
                if let Some(cond) = &gen.condition {
                    walk_expr(cond, source, search_start, tokens);
                }
            }
        }
        _ => {} // Bool, Nil, SelfExpr, AwaitExpr, etc. — no tokens needed
    }
}

#[derive(Debug, Clone, Copy)]
struct TokenPosition {
    line: u32,
    col: u32,
}

fn find_token_in_source(source: &str, token: &str, search_start: usize) -> Option<TokenPosition> {
    // Search within a window around span start
    let window = &source[search_start.min(source.len())..];
    let pos_in_window = window.find(token)?;
    let abs_pos = search_start + pos_in_window;
    byte_pos_to_line_col(source, abs_pos)
}

fn find_numeric_in_source(source: &str, search_start: usize) -> Option<TokenPosition> {
    let window = &source[search_start.min(source.len())..];
    let pos_in_window = window.find(|c: char| c.is_ascii_digit())?;
    let abs_pos = search_start + pos_in_window;
    byte_pos_to_line_col(source, abs_pos)
}

fn find_fstring_in_source(source: &str, search_start: usize) -> Option<TokenPosition> {
    let window = &source[search_start.min(source.len())..];
    let pos_in_window = window.find("f\"")?;
    let abs_pos = search_start + pos_in_window;
    byte_pos_to_line_col(source, abs_pos)
}

fn make_keyword_token(source: &str, span: &crate::ast::Span, keyword: &str) -> InternalToken {
    let pos = find_token_in_source(source, keyword, span.start)
        .unwrap_or(TokenPosition { line: 0, col: 0 });
    InternalToken {
        line: pos.line,
        character: pos.col,
        length: keyword.len() as u32,
        token_type: TYPE_KEYWORD,
        modifiers: 0,
    }
}

fn byte_pos_to_line_col(source: &str, byte_pos: usize) -> Option<TokenPosition> {
    let byte_pos = byte_pos.min(source.len());
    let prefix = &source[..byte_pos];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() as u32;
    let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = prefix[last_newline..].chars().count() as u32;
    Some(TokenPosition { line, col })
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "local"
            | "def"
            | "class"
            | "if"
            | "else"
            | "elif"
            | "while"
            | "for"
            | "in"
            | "return"
            | "break"
            | "continue"
            | "import"
            | "from"
            | "true"
            | "false"
            | "nil"
            | "try"
            | "except"
            | "finally"
            | "public"
            | "private"
            | "async"
            | "await"
            | "enum"
            | "struct"
            | "self"
            | "not"
            | "and"
            | "or"
            | "pass"
            | "as"
    )
}
