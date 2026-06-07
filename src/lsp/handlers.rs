use lsp_types::*;
use crate::lsp::store::DocumentStore;
use crate::lsp::bindings::Bindings;
use crate::parser::Parser;

pub fn handle_completion(
    store: &mut DocumentStore,
    bindings: &Bindings,
    params: CompletionParams,
) -> Option<CompletionResponse> {
    let uri = &params.text_document_position.text_document.uri;
    let pos = &params.text_document_position.position;

    store.reparse_if_dirty(uri);
    let doc = store.get(uri)?;

    let items = build_completions(&doc.source, pos, bindings, &doc.scope);
    Some(CompletionResponse::Array(items))
}

fn build_completions(
    source: &str,
    pos: &Position,
    bindings: &Bindings,
    scope: &crate::lsp::store::ScopeMap,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(pos.line as usize).map(|l| *l).unwrap_or("");
    let col = pos.character as usize;
    let line_prefix: String = line.chars().take(col).collect();
    let offset = line_prefix.len();

    // Check dot/colon completion
    if offset > 0 {
        let ch = line_prefix.chars().last().unwrap_or(' ');
        if ch == ':' || ch == '.' {
            let expr = extract_expr_before_dot(source, offset, pos.line as usize);
            if let Some(type_name) = resolve_expression_type(&expr, bindings, scope, source) {
                if ch == ':' {
                    for method in bindings.get_all_methods(&type_name) {
                        let params: Vec<String> = method.params.iter()
                            .map(|p| format!("${{{}:{}}}", p.name, p.r#type))
                            .collect();
                        let label = method.name.clone();
                        let detail = format!("({}): {}",
                            method.params.iter().map(|p| format!("{}: {}", p.name, p.r#type)).collect::<Vec<_>>().join(", "),
                            method.returns);
                        let insert_text = if method.params.is_empty() {
                            format!("{}()", method.name)
                        } else {
                            format!("{}({})", method.name, params.join(", "))
                        };

                        items.push(CompletionItem {
                            label,
                            kind: Some(CompletionItemKind::METHOD),
                            detail: Some(detail),
                            insert_text: Some(insert_text),
                            insert_text_format: if method.params.is_empty() { Some(InsertTextFormat::PLAIN_TEXT) } else { Some(InsertTextFormat::SNIPPET) },
                        documentation: {
                            if method.description.is_empty() {
                                None
                            } else {
                                Some(Documentation::MarkupContent(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: method.description.clone(),
                                }))
                            }
                        },
                            ..Default::default()
                        });
                    }
                } else {
                    for prop in bindings.get_all_properties(&type_name) {
                        let detail = format!("{}: {}{}", prop.name, prop.r#type,
                            if prop.rw { " (read/write)" } else { " (read-only)" });
                        items.push(CompletionItem {
                            label: prop.name.clone(),
                            kind: Some(CompletionItemKind::PROPERTY),
                            detail: Some(detail),
                        documentation: {
                            if prop.description.is_empty() { None }
                            else { Some(Documentation::MarkupContent(MarkupContent { kind: MarkupKind::Markdown, value: prop.description.clone() })) }
                        },
                            ..Default::default()
                        });
                    }
                    for method in bindings.get_all_methods(&type_name) {
                        let params: Vec<String> = method.params.iter()
                            .map(|p| format!("${{{}:{}}}", p.name, p.r#type)).collect();
                        let label = method.name.clone();
                        let detail = format!("({}): {}",
                            method.params.iter().map(|p| format!("{}: {}", p.name, p.r#type)).collect::<Vec<_>>().join(", "),
                            method.returns);
                        let insert_text = if method.params.is_empty() {
                            format!("{}()", method.name)
                        } else {
                            format!("{}({})", method.name, params.join(", "))
                        };
                        items.push(CompletionItem {
                            label,
                            kind: Some(CompletionItemKind::METHOD),
                            detail: Some(detail),
                            insert_text: Some(insert_text),
                            insert_text_format: if method.params.is_empty() { Some(InsertTextFormat::PLAIN_TEXT) } else { Some(InsertTextFormat::SNIPPET) },
                            ..Default::default()
                        });
                    }
                }
                return items;
            }
        }
    }

    // Import path completion
    let trimmed = line_prefix.trim_start().to_lowercase();
    if let Some(_import_match) = parse_import_path(&trimmed) {
        // Return import completions later
        items.push(CompletionItem {
            label: "import path completion not yet implemented".into(),
            kind: Some(CompletionItemKind::TEXT),
            ..Default::default()
        });
        return items;
    }

    // Keywords
    let kw = ["if", "else", "while", "for", "in", "function", "local", "return",
        "class", "struct", "enum", "import", "as", "public", "private",
        "true", "false", "nil", "self", "break", "continue", "and", "or", "not"];
    let prefix = line_prefix.trim_start();
    for &k in &kw {
        if k.starts_with(prefix) {
            let mut item = CompletionItem {
                label: k.into(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            };
            match k {
                "if" => { item.insert_text = Some("if (${1:condition}) {\n\t${0}\n}".into()); item.insert_text_format = Some(InsertTextFormat::SNIPPET); }
                "while" => { item.insert_text = Some("while (${1:condition}) {\n\t${0}\n}".into()); item.insert_text_format = Some(InsertTextFormat::SNIPPET); }
                "for" => { item.insert_text = Some("for ${1:x} in ${2:items} {\n\t${0}\n}".into()); item.insert_text_format = Some(InsertTextFormat::SNIPPET); }
                "function" => { item.insert_text = Some("function ${1:name}(${2:params}) {\n\t${0}\n}".into()); item.insert_text_format = Some(InsertTextFormat::SNIPPET); }
                "class" => { item.insert_text = Some("class ${1:Name} {\n\t${0}\n}".into()); item.insert_text_format = Some(InsertTextFormat::SNIPPET); }
                "struct" => { item.insert_text = Some("struct ${1:Name} {\n\t${0}\n}".into()); item.insert_text_format = Some(InsertTextFormat::SNIPPET); }
                "enum" => { item.insert_text = Some("enum ${1:Name} {\n\t${0}\n}".into()); item.insert_text_format = Some(InsertTextFormat::SNIPPET); }
                "import" => { item.insert_text = Some("import \"${1:path}\" as ${2:alias}".into()); item.insert_text_format = Some(InsertTextFormat::SNIPPET); }
                "local" => { item.insert_text = Some("local ${1:name} = ".into()); item.insert_text_format = Some(InsertTextFormat::SNIPPET); }
                "return" => { item.insert_text = Some("return ${1:value}".into()); item.insert_text_format = Some(InsertTextFormat::SNIPPET); }
                _ => {}
            }
            items.push(item);
        }
    }

    // Roblox globals
    for g in &bindings.globals {
        if g.name.to_lowercase().starts_with(prefix) {
            items.push(CompletionItem {
                label: g.name.clone(),
                kind: Some(CompletionItemKind::VALUE),
                detail: Some(format!("{} — {}", g.r#type, g.description)),
                ..Default::default()
            });
        }
    }

    // Roblox functions
    for f in &bindings.functions {
        if f.name.to_lowercase().starts_with(prefix) {
            let params: Vec<String> = f.params.iter().map(|p| format!("${{{}:{}}}", p.name, p.r#type)).collect();
            items.push(CompletionItem {
                label: f.name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("({}): {}",
                    f.params.iter().map(|p| format!("{}: {}", p.name, p.r#type)).collect::<Vec<_>>().join(", "),
                    f.returns)),
                insert_text: Some(if f.params.is_empty() { f.name.clone() } else { format!("{}({})", f.name, params.join(", ")) }),
                insert_text_format: Some(if f.params.is_empty() { InsertTextFormat::PLAIN_TEXT } else { InsertTextFormat::SNIPPET }),
            documentation: {
                if f.description.is_empty() { None }
                else { Some(Documentation::MarkupContent(MarkupContent { kind: MarkupKind::Markdown, value: f.description.clone() })) }
            },
                ..Default::default()
            });
        }
    }

    // Enum completions
    if prefix.starts_with("enum.") {
        for en in &bindings.enums {
            for item_name in &en.items {
                let full = format!("Enum.{}.{}", en.name, item_name);
                if full.to_lowercase().starts_with(prefix) {
                    items.push(CompletionItem {
                        label: full,
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        detail: Some(en.name.clone()),
                        ..Default::default()
                    });
                }
            }
        }
    } else {
        for en in &bindings.enums {
            if en.name.to_lowercase().starts_with(prefix) {
                items.push(CompletionItem {
                    label: format!("Enum.{}", en.name),
                    kind: Some(CompletionItemKind::ENUM),
                    detail: Some(en.items.join(", ")),
                documentation: {
                    if en.description.is_empty() { None }
                    else { Some(Documentation::MarkupContent(MarkupContent { kind: MarkupKind::Markdown, value: en.description.clone() })) }
                },
                    ..Default::default()
                });
            }
        }
    }

    items
}

fn extract_expr_before_dot(source: &str, cursor_offset: usize, line: usize) -> String {
    if cursor_offset == 0 { return String::new(); }
    let lines: Vec<&str> = source.lines().collect();
    let mut offset = if line < lines.len() {
        lines[..line].iter().map(|l| l.len() + 1).sum::<usize>() + cursor_offset - 1
    } else {
        cursor_offset - 1
    };
    // Skip the . or :
    if offset > 0 {
        offset -= 1;
    }
    // Walk back through identifier chars and dots
    let bytes = source.as_bytes();
    let mut start = offset;
    loop {
        if start == 0 { break; }
        let c = bytes[start - 1] as char;
        if c.is_alphanumeric() || c == '_' || c == '.' {
            start -= 1;
        } else {
            break;
        }
    }
    source[start..offset].to_string()
}

fn resolve_expression_type(
    expr: &str,
    bindings: &Bindings,
    scope: &crate::lsp::store::ScopeMap,
    source: &str,
) -> Option<String> {
    if expr.is_empty() { return None; }

    // Check scope first
    let parts: Vec<&str> = expr.split('.').collect();
    let root = parts[0];

    // Check bindings globals
    if let Some(g) = bindings.get_global(root) {
        let mut current = g.r#type.clone();
        for part in &parts[1..] {
            if let Some(prop) = bindings.get_all_properties(&current).iter().find(|p| p.name.eq_ignore_ascii_case(part)) {
                current = prop.r#type.clone();
            } else if let Some(m) = bindings.get_all_methods(&current).iter().find(|m| m.name.eq_ignore_ascii_case(part)) {
                current = m.returns.clone();
            } else {
                return Some("Instance".into());
            }
        }
        return Some(current);
    }

    // Check scope
    if let Some(t) = scope.variables.get(root) {
        return Some(t.clone());
    }

    // Infer from source (local x = Instance.new("ClassName"))
    let new_pattern = format!("local {} = ", root);
    if let Some(after) = source.find(&new_pattern).map(|i| &source[i + new_pattern.len()..]) {
        if let Some(end) = after.find(' ').or_else(|| after.find('\n')).or_else(|| after.find(';')) {
            let rhs = &after[..end];
            if rhs.contains(".new(") {
                if let Some(class_name) = rhs.split('.').next() {
                    return Some(class_name.trim().to_string());
                }
            }
        }
    }

    // Infer from source (local x = expr:GetService("Name"))
    if let Some(after) = source.find(&format!("local {} = ", root)).map(|i| &source[i + format!("local {} = ", root).len()..]) {
        if after.contains(":GetService(") {
            if let Some(start) = after.find("\"") {
                if let Some(end) = after[start+1..].find("\"") {
                    return Some(after[start+1..start+1+end].to_string());
                }
            }
        }
    }

    // Infer from source (local x = obj:Method(...))
    if let Some(after) = source.find(&format!("local {} = ", root)).map(|i| &source[i + format!("local {} = ", root).len()..]) {
        if let Some(colon) = after.find(':') {
            let obj = after[..colon].trim().to_string();
            let after_colon = &after[colon+1..];
            if let Some(paren) = after_colon.find('(') {
                let method = &after_colon[..paren];
                let obj_type = resolve_expression_type(&obj, bindings, scope, source);
                if let Some(t) = obj_type {
                    return bindings.get_method_return(&t, method);
                }
            }
        }
    }

    // Default: is it a known class name?
    if bindings.get_type(root).is_some() {
        return Some(root.to_string());
    }

    None
}

fn parse_import_path(trimmed: &str) -> Option<String> {
    if trimmed.starts_with("import\"") || trimmed.starts_with("import \"") {
        let after = if trimmed.starts_with("import\"") { &trimmed[7..] } else { &trimmed[8..] };
        // Extract partial path up to the cursor
        let path = after.trim_start_matches('"').trim_start_matches('\'');
        Some(path.to_string())
    } else {
        None
    }
}

pub fn handle_hover(
    store: &mut DocumentStore,
    bindings: &Bindings,
    params: HoverParams,
) -> Option<Hover> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = &params.text_document_position_params.position;

    store.reparse_if_dirty(uri);
    let doc = store.get(uri)?;

    let word = extract_word_at(&doc.source, pos);

    // Check globals
    if let Some(g) = bindings.get_global(&word) {
        let value = format!("**{}**\n\nType: `{}`\n\n{}", g.name, g.r#type, g.description);
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value }),
            range: None,
        });
    }

    // Check functions
    if let Some(f) = bindings.functions.iter().find(|f| f.name.eq_ignore_ascii_case(&word)) {
        let params: Vec<String> = f.params.iter().map(|p| format!("`{}: {}`", p.name, p.r#type)).collect();
        let value = format!("**{}({})** → `{}`\n\n{}", f.name, params.join(", "), f.returns, f.description);
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value }),
            range: None,
        });
    }

    // Check symbols
    if let Some(sym) = doc.symbols.iter().find(|s| s.name == word) {
        let mut value = format!("**{} {}** `{}`", sym.access, sym.kind, sym.name);
        if !sym.params.is_empty() {
            value.push_str(&format!("\n\nParameters: `{}`", sym.params.join(", ")));
        }
        if !sym.fields.is_empty() {
            let label = if sym.kind == "enum" { "Variants" } else { "Fields" };
            value.push_str(&format!("\n\n{}: `{}`", label, sym.fields.join(", ")));
        }
        value.push_str(&format!("\n\n*Line {}, Column {}*", sym.location.line, sym.location.column));
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value }),
            range: None,
        });
    }

    None
}

fn extract_word_at(source: &str, pos: &Position) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(pos.line as usize).map(|l| *l).unwrap_or("");
    let col = pos.character as usize;
    if col >= line.len() { return String::new(); }

    let start = line[..col].rfind(|c: char| !c.is_alphanumeric() && c != '_').map(|i| i + 1).unwrap_or(0);
    let end = line[col..].find(|c: char| !c.is_alphanumeric() && c != '_').map(|i| col + i).unwrap_or(line.len());

    line[start..end].to_string()
}

pub fn handle_definition(
    store: &mut DocumentStore,
    params: GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = &params.text_document_position_params.position;

    store.reparse_if_dirty(uri);
    let doc = store.get(uri)?;

    let word = extract_word_at(&doc.source, pos);

    if let Some(sym) = doc.symbols.iter().find(|s| s.name == word) {
        let range = Range {
            start: Position { line: (sym.location.line as u32).saturating_sub(1), character: (sym.location.column as u32).saturating_sub(1) },
            end: Position { line: (sym.location.end_line as u32).saturating_sub(1), character: (sym.location.end_column as u32).saturating_sub(1) },
        };
        return Some(GotoDefinitionResponse::Scalar(Location {
            uri: uri.clone(),
            range,
        }));
    }

    None
}

pub fn handle_diagnostics(
    store: &mut DocumentStore,
    uri: &Url,
) -> Vec<Diagnostic> {
    store.reparse_if_dirty(uri);
    let doc = match store.get(uri) {
        Some(d) => d,
        None => return Vec::new(),
    };

    let mut diagnostics = Vec::new();

    if doc.ast.is_empty() {
        // Parse error — try to extract from source
        let mut tokens = Vec::new();
        let mut spans = Vec::new();
        for (res, span) in logos::Logos::lexer(doc.source.as_str()).spanned() {
            if let Ok(tok) = res {
                tokens.push(tok);
                spans.push(span.start);
            }
        }
        let mut parser = Parser::new(tokens, spans, &doc.source);
        if let Err(e) = parser.parse_program() {
            let (line, col) = extract_line_col(&e);
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position { line: (line as u32).saturating_sub(1), character: (col as u32).saturating_sub(1) },
                    end: Position { line: (line as u32).saturating_sub(1), character: (col as u32 + 20) },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: e,
                source: Some("wolfram".into()),
                ..Default::default()
            });
        }
    }

    diagnostics
}

fn extract_line_col(error: &str) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    if let Some(pos_part) = error.strip_prefix("line ") {
        if let Some(comma) = pos_part.find(", column ") {
            if let Ok(l) = pos_part[..comma].parse::<usize>() {
                line = l;
            }
            let after = &pos_part[comma + 9..];
            if let Some(colon) = after.find(':') {
                if let Ok(c) = after[..colon].parse::<usize>() {
                    col = c;
                }
            }
        }
    }
    (line, col)
}

pub fn handle_document_symbols(
    store: &mut DocumentStore,
    params: DocumentSymbolParams,
) -> Option<DocumentSymbolResponse> {
    let uri = &params.text_document.uri;
    store.reparse_if_dirty(uri);
    let doc = store.get(uri)?;

    let symbols: Vec<DocumentSymbol> = doc.symbols.iter().map(|s| {
        let kind = match s.kind.as_str() {
            "function" => SymbolKind::FUNCTION,
            "class" => SymbolKind::CLASS,
            "struct" => SymbolKind::STRUCT,
            "enum" => SymbolKind::ENUM,
            _ => SymbolKind::VARIABLE,
        };

        DocumentSymbol {
            name: s.name.clone(),
            detail: Some(format!("{} {}", s.access, s.kind)),
            kind,
            range: Range {
                start: Position { line: (s.location.line as u32).saturating_sub(1), character: (s.location.column as u32).saturating_sub(1) },
                end: Position { line: (s.location.end_line as u32).saturating_sub(1), character: (s.location.end_column as u32).saturating_sub(1) },
            },
            selection_range: Range {
                start: Position { line: (s.location.line as u32).saturating_sub(1), character: (s.location.column as u32).saturating_sub(1) },
                end: Position { line: (s.location.line as u32).saturating_sub(1), character: (s.location.column as u32).saturating_sub(1) + s.name.len() as u32 },
            },
            children: None,
            deprecated: None,
            tags: None,
        }
    }).collect();

    Some(DocumentSymbolResponse::Nested(symbols))
}
