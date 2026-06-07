use crate::lsp::bindings::Bindings;
use crate::lsp::store::DocumentStore;
use lsp_types::*;

pub fn handle_completion(
    store: &mut DocumentStore,
    bindings: &Bindings,
    workspace_files: &[String],
    params: CompletionParams,
) -> Option<CompletionResponse> {
    let uri = &params.text_document_position.text_document.uri;
    let pos = &params.text_document_position.position;

    store.reparse_if_dirty(uri);
    let doc = store.get(uri)?;

    let items = build_completions(&doc.source, pos, bindings, &doc.scope, workspace_files);
    Some(CompletionResponse::Array(items))
}

fn build_completions(
    source: &str,
    pos: &Position,
    bindings: &Bindings,
    scope: &crate::lsp::store::ScopeMap,
    workspace_files: &[String],
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(pos.line as usize).copied().unwrap_or("");
    let col = pos.character as usize;
    let line_prefix: String = line.chars().take(col).collect();
    let offset = line_prefix.len();
    let trimmed = line_prefix.trim_start().to_lowercase();
    let ctx = detect_context(&line_prefix);

    // Suppress completions in comment/string/definition-name contexts
    if matches!(ctx, Ctx::Comment | Ctx::String | Ctx::DefinitionName) {
        return items;
    }

    // Import path completion
    if ctx == Ctx::ImportPath {
        let partial = extract_import_partial(&line_prefix)
            .unwrap_or_default()
            .to_lowercase();
        for f in workspace_files {
            let lower = f.to_lowercase();
            if lower.starts_with(&partial) || lower.contains(&partial) {
                items.push(CompletionItem {
                    label: f.clone(),
                    kind: Some(CompletionItemKind::FILE),
                    sort_text: Some(format!("0{}", f)),
                    ..Default::default()
                });
            }
        }
        return items;
    }

    // Dot/colon member access
    if ctx == Ctx::DotColon {
        let ch = line_prefix.chars().last().unwrap_or(' ');
        let expr = extract_expr_before_dot(source, offset, pos.line as usize);
        if let Some(type_name) = resolve_expression_type(&expr, bindings, scope, source) {
            if ch == ':' {
                for method in bindings.get_all_methods(&type_name) {
                    let params: Vec<String> = method
                        .params
                        .iter()
                        .map(|p| format!("${{{}:{}}}", p.name, p.r#type))
                        .collect();
                    let label = method.name.clone();
                    let detail = format!(
                        "({}): {}",
                        method
                            .params
                            .iter()
                            .map(|p| format!("{}: {}", p.name, p.r#type))
                            .collect::<Vec<_>>()
                            .join(", "),
                        method.returns
                    );
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
                        insert_text_format: if method.params.is_empty() {
                            Some(InsertTextFormat::PLAIN_TEXT)
                        } else {
                            Some(InsertTextFormat::SNIPPET)
                        },
                        sort_text: Some(format!("3{}", method.name)),
                        documentation: if method.description.is_empty() {
                            None
                        } else {
                            Some(Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: method.description.clone(),
                            }))
                        },
                        ..Default::default()
                    });
                }
            } else {
                for prop in bindings.get_all_properties(&type_name) {
                    let detail = format!(
                        "{}: {}{}",
                        prop.name,
                        prop.r#type,
                        if prop.rw {
                            " (read/write)"
                        } else {
                            " (read-only)"
                        }
                    );
                    items.push(CompletionItem {
                        label: prop.name.clone(),
                        kind: Some(CompletionItemKind::PROPERTY),
                        detail: Some(detail),
                        sort_text: Some(format!("3{}", prop.name)),
                        documentation: if prop.description.is_empty() {
                            None
                        } else {
                            Some(Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: prop.description.clone(),
                            }))
                        },
                        ..Default::default()
                    });
                }
                for method in bindings.get_all_methods(&type_name) {
                    let params: Vec<String> = method
                        .params
                        .iter()
                        .map(|p| format!("${{{}:{}}}", p.name, p.r#type))
                        .collect();
                    let label = method.name.clone();
                    let detail = format!(
                        "({}): {}",
                        method
                            .params
                            .iter()
                            .map(|p| format!("{}: {}", p.name, p.r#type))
                            .collect::<Vec<_>>()
                            .join(", "),
                        method.returns
                    );
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
                        insert_text_format: if method.params.is_empty() {
                            Some(InsertTextFormat::PLAIN_TEXT)
                        } else {
                            Some(InsertTextFormat::SNIPPET)
                        },
                        sort_text: Some(format!("3{}", method.name)),
                        ..Default::default()
                    });
                }
            }
        }
        return items;
    }

    // ── General completions with sortText ──
    let word_prefix = extract_last_word(trimmed.trim_start()).to_lowercase();
    let wp = &word_prefix;

    let add_keywords = matches!(ctx, Ctx::StatementStart | Ctx::Expression);
    let add_value_kw = matches!(ctx, Ctx::ValueExpr | Ctx::Expression);
    let add_locals = !matches!(ctx, Ctx::StatementStart);
    let add_api = matches!(ctx, Ctx::ValueExpr | Ctx::Expression);
    let add_enums = matches!(ctx, Ctx::ValueExpr | Ctx::EnumValue | Ctx::Expression);

    // Locals (priority 1)
    if add_locals {
        for (name, var_type) in &scope.variables {
            if name.to_lowercase().starts_with(wp) {
                let detail = if var_type != "any" && var_type != "local" && var_type != name {
                    var_type.clone()
                } else {
                    String::new()
                };
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: if detail.is_empty() {
                        None
                    } else {
                        Some(detail)
                    },
                    sort_text: Some(format!("1{}", name)),
                    ..Default::default()
                });
            }
        }
    }

    // Structural keywords (priority 0)
    if add_keywords {
        let struct_kw = [
            "if", "else", "elif", "while", "for", "function", "class", "struct", "enum", "import",
            "local", "return", "break", "continue", "public", "private",
        ];
        for &k in &struct_kw {
            if k.starts_with(wp) {
                let mut item = CompletionItem {
                    label: k.into(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    sort_text: Some(format!("0{}", k)),
                    ..Default::default()
                };
                match k {
                    "if" => {
                        item.insert_text = Some("if (${1:condition}) {\n\t${0}\n}".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    "else" => {
                        item.insert_text = Some("else {\n\t${0}\n}".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    "elif" => {
                        item.insert_text = Some("elif (${1:condition}) {\n\t${0}\n}".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    "while" => {
                        item.insert_text = Some("while (${1:condition}) {\n\t${0}\n}".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    "for" => {
                        item.insert_text = Some("for ${1:x} in ${2:items} {\n\t${0}\n}".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    "function" => {
                        item.insert_text =
                            Some("function ${1:name}(${2:params}) {\n\t${0}\n}".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    "class" => {
                        item.insert_text = Some("class ${1:Name} {\n\t${0}\n}".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    "struct" => {
                        item.insert_text = Some("struct ${1:Name} {\n\t${0}\n}".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    "enum" => {
                        item.insert_text = Some("enum ${1:Name} {\n\t${0}\n}".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    "import" => {
                        item.insert_text = Some("import \"${1:path}\" as ${2:alias}".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    "local" => {
                        item.insert_text = Some("local ${1:name} = ".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    "return" => {
                        item.insert_text = Some("return ${1:value}".into());
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                    _ => {}
                }
                items.push(item);
            }
        }
    }

    // Value keywords: true, false, nil, self, and, or, not (priority 0)
    if add_value_kw {
        let val_kw = ["true", "false", "nil", "self"];
        for &k in &val_kw {
            if k.starts_with(wp) {
                items.push(CompletionItem {
                    label: k.into(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    sort_text: Some(format!("0{}", k)),
                    ..Default::default()
                });
            }
        }
    }

    // API globals (priority 2)
    if add_api {
        for g in &bindings.globals {
            if g.name.to_lowercase().starts_with(wp) {
                items.push(CompletionItem {
                    label: g.name.clone(),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: Some(format!("{} — {}", g.r#type, g.description)),
                    sort_text: Some(format!("2{}", g.name)),
                    ..Default::default()
                });
            }
        }
        for f in &bindings.functions {
            if f.name.to_lowercase().starts_with(wp) {
                let params: Vec<String> = f
                    .params
                    .iter()
                    .map(|p| format!("${{{}:{}}}", p.name, p.r#type))
                    .collect();
                items.push(CompletionItem {
                    label: f.name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(format!(
                        "({}): {}",
                        f.params
                            .iter()
                            .map(|p| format!("{}: {}", p.name, p.r#type))
                            .collect::<Vec<_>>()
                            .join(", "),
                        f.returns
                    )),
                    insert_text: Some(if f.params.is_empty() {
                        f.name.clone()
                    } else {
                        format!("{}({})", f.name, params.join(", "))
                    }),
                    insert_text_format: Some(if f.params.is_empty() {
                        InsertTextFormat::PLAIN_TEXT
                    } else {
                        InsertTextFormat::SNIPPET
                    }),
                    sort_text: Some(format!("2{}", f.name)),
                    documentation: if f.description.is_empty() {
                        None
                    } else {
                        Some(Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: f.description.clone(),
                        }))
                    },
                    ..Default::default()
                });
            }
        }
    }

    // Enum completions (priority 3)
    if add_enums {
        if word_prefix.starts_with("enum.") {
            for en in &bindings.enums {
                for item_name in &en.items {
                    let full = format!("Enum.{}.{}", en.name, item_name);
                    if full.to_lowercase().starts_with(wp) {
                        items.push(CompletionItem {
                            label: full.clone(),
                            kind: Some(CompletionItemKind::ENUM_MEMBER),
                            detail: Some(en.name.clone()),
                            sort_text: Some(format!("3{}", full)),
                            ..Default::default()
                        });
                    }
                }
            }
        } else {
            for en in &bindings.enums {
                if en.name.to_lowercase().starts_with(wp) {
                    items.push(CompletionItem {
                        label: format!("Enum.{}", en.name),
                        kind: Some(CompletionItemKind::ENUM),
                        detail: Some(en.items.join(", ")),
                        sort_text: Some(format!("3Enum.{}", en.name)),
                        documentation: if en.description.is_empty() {
                            None
                        } else {
                            Some(Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: en.description.clone(),
                            }))
                        },
                        ..Default::default()
                    });
                }
            }
        }
    }

    items
}

#[derive(PartialEq)]
enum Ctx {
    ImportPath,
    DefinitionName,
    DotColon,
    EnumValue,
    ValueExpr,
    Comment,
    String,
    StatementStart,
    Expression,
}

fn detect_context(line_prefix: &str) -> Ctx {
    let trimmed = line_prefix.trim_start();
    if trimmed.is_empty() || line_prefix.len() - trimmed.len() > 0 && trimmed.is_empty() {
        return Ctx::StatementStart;
    }

    // Comment
    if trimmed.starts_with("//") || trimmed.starts_with("--") {
        return Ctx::Comment;
    }

    // String detection
    let mut in_string = false;
    let mut quote_char = ' ';
    for ch in line_prefix.chars() {
        if !in_string && (ch == '"' || ch == '\'') {
            in_string = true;
            quote_char = ch;
        } else if in_string && ch == quote_char {
            in_string = false;
        }
    }
    if in_string {
        // Check if it's inside an import string
        if trimmed.starts_with("import") && (trimmed.contains('"') || trimmed.contains('\'')) {
            return Ctx::ImportPath;
        }
        return Ctx::String;
    }

    // Import path
    let re = regex_lite::Regex::new(r#"^import\s+["'][^"']*$"#).unwrap();
    if re.is_match(trimmed) {
        return Ctx::ImportPath;
    }

    // Definition name: function/class/struct/enum keyword followed by (optional) name
    let def_re = regex_lite::Regex::new(
        r"^(?:local\s+)?(?:public\s+|private\s+)?(function|class|struct|enum)\s+\w*$",
    )
    .unwrap();
    if def_re.is_match(trimmed) {
        return Ctx::DefinitionName;
    }

    // Dot/colon
    let last_char = line_prefix.chars().last().unwrap_or(' ');
    if last_char == '.' || last_char == ':' {
        if trimmed.ends_with("Enum.") {
            return Ctx::EnumValue;
        }
        return Ctx::DotColon;
    }

    // Enum value context
    if let Some(word) = word_before_cursor(trimmed) {
        if word.to_lowercase().starts_with("enum.") {
            let before = &trimmed[..trimmed.len().saturating_sub(word.len())];
            if is_value_position(before) {
                return Ctx::EnumValue;
            }
        }
    }

    // Value expression
    if is_value_position(trimmed) {
        return Ctx::ValueExpr;
    }

    Ctx::Expression
}

fn is_value_position(text: &str) -> bool {
    let vp_re = regex_lite::Regex::new(r#"[=(,\[\-+*\/<>!]|\b(?:return|and|or)\b\s*$"#).unwrap();
    vp_re.is_match(text)
}

fn word_before_cursor(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut end = text.len();
    while end > 0
        && !(bytes[end - 1] as char).is_alphanumeric()
        && bytes[end - 1] != b'_'
        && bytes[end - 1] != b'.'
    {
        end -= 1;
    }
    if end == 0 {
        return None;
    }
    let mut start = end;
    while start > 0
        && ((bytes[start - 1] as char).is_alphanumeric()
            || bytes[start - 1] == b'_'
            || bytes[start - 1] == b'.')
    {
        start -= 1;
    }
    Some(&text[start..end])
}

fn extract_import_partial(line_prefix: &str) -> Option<&str> {
    let trimmed = line_prefix.trim_start();
    if let Some(after) = trimmed.strip_prefix("import") {
        let inner = after.trim_start();
        if let Some(rest) = inner.strip_prefix('"').or_else(|| inner.strip_prefix('\'')) {
            return Some(rest);
        }
    }
    None
}

fn extract_last_word(text: &str) -> &str {
    let bytes = text.as_bytes();
    let end = text.len();
    let mut start = end;
    while start > 0
        && ((bytes[start - 1] as char).is_alphanumeric()
            || bytes[start - 1] == b'_'
            || bytes[start - 1] == b'.')
    {
        start -= 1;
    }
    &text[start..end]
}

fn extract_expr_before_dot(source: &str, cursor_offset: usize, line: usize) -> String {
    if cursor_offset == 0 {
        return String::new();
    }
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
        if start == 0 {
            break;
        }
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
    if expr.is_empty() {
        return None;
    }

    // Check scope first
    let parts: Vec<&str> = expr.split('.').collect();
    let root = parts[0];

    // Check bindings globals
    if let Some(g) = bindings.get_global(root) {
        let mut current = g.r#type.clone();
        for part in &parts[1..] {
            if let Some(prop) = bindings
                .get_all_properties(&current)
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case(part))
            {
                current = prop.r#type.clone();
            } else if let Some(m) = bindings
                .get_all_methods(&current)
                .iter()
                .find(|m| m.name.eq_ignore_ascii_case(part))
            {
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
    if let Some(after) = source
        .find(&new_pattern)
        .map(|i| &source[i + new_pattern.len()..])
    {
        if let Some(end) = after
            .find(' ')
            .or_else(|| after.find('\n'))
            .or_else(|| after.find(';'))
        {
            let rhs = &after[..end];
            if rhs.contains(".new(") {
                if let Some(class_name) = rhs.split('.').next() {
                    return Some(class_name.trim().to_string());
                }
            }
        }
    }

    // Infer from source (local x = expr:GetService("Name"))
    if let Some(after) = source
        .find(&format!("local {} = ", root))
        .map(|i| &source[i + format!("local {} = ", root).len()..])
    {
        if after.contains(":GetService(") {
            if let Some(start) = after.find("\"") {
                if let Some(end) = after[start + 1..].find("\"") {
                    return Some(after[start + 1..start + 1 + end].to_string());
                }
            }
        }
    }

    // Infer from source (local x = obj:Method(...))
    if let Some(after) = source
        .find(&format!("local {} = ", root))
        .map(|i| &source[i + format!("local {} = ", root).len()..])
    {
        if let Some(colon) = after.find(':') {
            let obj = after[..colon].trim().to_string();
            let after_colon = &after[colon + 1..];
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
        let value = format!(
            "**{}**\n\nType: `{}`\n\n{}",
            g.name, g.r#type, g.description
        );
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: None,
        });
    }

    // Check functions
    if let Some(f) = bindings
        .functions
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case(&word))
    {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| format!("`{}: {}`", p.name, p.r#type))
            .collect();
        let value = format!(
            "**{}({})** → `{}`\n\n{}",
            f.name,
            params.join(", "),
            f.returns,
            f.description
        );
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
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
            let label = if sym.kind == "enum" {
                "Variants"
            } else {
                "Fields"
            };
            value.push_str(&format!("\n\n{}: `{}`", label, sym.fields.join(", ")));
        }
        value.push_str(&format!(
            "\n\n*Line {}, Column {}*",
            sym.location.line, sym.location.column
        ));
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: None,
        });
    }

    None
}

fn extract_word_at(source: &str, pos: &Position) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(pos.line as usize).map(|l| *l).unwrap_or("");
    let col = pos.character as usize;
    if col >= line.len() {
        return String::new();
    }

    let start = line[..col]
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| i + 1)
        .unwrap_or(0);
    let end = line[col..]
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| col + i)
        .unwrap_or(line.len());

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
    let extended = extract_extended_expr_at(&doc.source, pos);

    // Check for alias.Member pattern (cross-file)
    if let Some(dot) = extended.find('.') {
        let alias = &extended[..dot];
        let member = &extended[dot + 1..];

        // Find import matching this alias
        if let Some(import) = doc.imports.iter().find(|i| i.alias == alias) {
            if let Some(loc) = resolve_import_location(&import.path, member, store, uri) {
                return Some(GotoDefinitionResponse::Scalar(loc));
            }
        }
    }

    // Check local symbols
    if let Some(sym) = doc.symbols.iter().find(|s| s.name == word) {
        let range = Range {
            start: Position {
                line: (sym.location.line as u32).saturating_sub(1),
                character: (sym.location.column as u32).saturating_sub(1),
            },
            end: Position {
                line: (sym.location.end_line as u32).saturating_sub(1),
                character: (sym.location.end_column as u32).saturating_sub(1),
            },
        };
        return Some(GotoDefinitionResponse::Scalar(Location {
            uri: uri.clone(),
            range,
        }));
    }

    // Search all workspace documents for the symbol
    for other_doc in store.get_all() {
        if let Some(sym) = other_doc
            .symbols
            .iter()
            .find(|s| s.name == word && s.access == "public")
        {
            let range = Range {
                start: Position {
                    line: (sym.location.line as u32).saturating_sub(1),
                    character: (sym.location.column as u32).saturating_sub(1),
                },
                end: Position {
                    line: (sym.location.end_line as u32).saturating_sub(1),
                    character: (sym.location.end_column as u32).saturating_sub(1),
                },
            };
            return Some(GotoDefinitionResponse::Scalar(Location {
                uri: other_doc.uri.clone(),
                range,
            }));
        }
    }

    None
}

fn resolve_import_location(
    import_path: &str,
    member: &str,
    store: &DocumentStore,
    _current_uri: &Url,
) -> Option<Location> {
    let file_name = if import_path.ends_with(".wrm") {
        import_path.to_string()
    } else {
        format!("{}.wrm", import_path)
    };

    // Extract just the filename for lookup
    let just_name = std::path::Path::new(&file_name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&file_name)
        .to_string();

    // Try to find the target document in the store
    if let Some(target_doc) = store.find_by_file_name(&just_name) {
        if let Some(sym) = target_doc.symbols.iter().find(|s| s.name == member) {
            let range = Range {
                start: Position {
                    line: (sym.location.line as u32).saturating_sub(1),
                    character: (sym.location.column as u32).saturating_sub(1),
                },
                end: Position {
                    line: (sym.location.end_line as u32).saturating_sub(1),
                    character: (sym.location.end_column as u32).saturating_sub(1),
                },
            };
            return Some(Location {
                uri: target_doc.uri.clone(),
                range,
            });
        }
    }

    None
}

fn extract_extended_expr_at(source: &str, pos: &Position) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(pos.line as usize).copied().unwrap_or("");
    let col = pos.character as usize;
    if col >= line.len() {
        return String::new();
    }

    let start = line[..col]
        .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
        .map(|i| i + 1)
        .unwrap_or(0);
    let end = line[col..]
        .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
        .map(|i| col + i)
        .unwrap_or(line.len());

    line[start..end].to_string()
}

pub fn handle_diagnostics(store: &mut DocumentStore, uri: &Url) -> Vec<Diagnostic> {
    store.reparse_if_dirty(uri);
    let doc = match store.get(uri) {
        Some(d) => d,
        None => return Vec::new(),
    };

    let mut diagnostics = Vec::new();

    if doc.ast.is_empty() {
        // Parse error — try to extract from source
        match crate::tokenize_and_parse(&doc.source) {
            Err(e) => {
                let (line, col) = extract_line_col(&e);
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: (line as u32).saturating_sub(1),
                            character: (col as u32).saturating_sub(1),
                        },
                        end: Position {
                            line: (line as u32).saturating_sub(1),
                            character: (col as u32 + 20),
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: e,
                    source: Some("wolfram".into()),
                    ..Default::default()
                });
            }
            _ => {}
        }
        return diagnostics;
    }

    // Scope analysis warnings
    let file_path = uri.path().to_string();
    let scope_warnings = crate::scope::ScopeAnalysis::analyze(&doc.ast, &doc.source, &file_path);
    for warning in &scope_warnings {
        let (line, col) = extract_line_col_from_str(warning);
        diagnostics.push(Diagnostic {
            range: Range {
                start: Position {
                    line: (line as u32).saturating_sub(1),
                    character: (col as u32).saturating_sub(1),
                },
                end: Position {
                    line: (line as u32).saturating_sub(1),
                    character: col as u32 + 30,
                },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            message: warning.clone(),
            source: Some("wolfram-scope".into()),
            ..Default::default()
        });
    }

    // Type check warnings
    let type_result = crate::typeck::check_types(&doc.ast);
    for err in &type_result.errors {
        diagnostics.push(Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 1,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            message: err.clone(),
            source: Some("wolfram-typeck".into()),
            ..Default::default()
        });
    }
    for warn in &type_result.warnings {
        diagnostics.push(Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 1,
                },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            message: warn.clone(),
            source: Some("wolfram-typeck".into()),
            ..Default::default()
        });
    }

    diagnostics
}

fn extract_line_col_from_str(msg: &str) -> (usize, usize) {
    if let Some(pos_start) = msg.find("line ") {
        if let Some(comma) = msg[pos_start..].find(", column ") {
            let line_str = &msg[pos_start + 5..pos_start + comma];
            if let Ok(line) = line_str.parse::<usize>() {
                let rest = &msg[pos_start + comma + 9..];
                if let Some(colon) = rest.find(':') {
                    if let Ok(col) = rest[..colon].parse::<usize>() {
                        return (line, col);
                    }
                }
            }
        }
    }
    (1, 1)
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

pub fn handle_signature_help(
    store: &mut DocumentStore,
    bindings: &Bindings,
    params: SignatureHelpParams,
) -> Option<SignatureHelp> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = &params.text_document_position_params.position;

    store.reparse_if_dirty(uri);
    let doc = store.get(uri)?;

    let lines: Vec<&str> = doc.source.lines().collect();
    let line = lines.get(pos.line as usize).copied().unwrap_or("");
    let col = pos.character as usize;

    if col > line.len() {
        return None;
    }

    // Find matching opening paren by walking backwards
    let full_source = &doc.source;
    let offset = lines[..pos.line as usize]
        .iter()
        .map(|l| l.len() + 1)
        .sum::<usize>()
        + col;

    // Find the opening paren of the current call
    let source_bytes = full_source.as_bytes();
    let (open_offset, _preceding_expr) = find_opening_paren(source_bytes, offset)?;

    // Find function name before the opening paren
    let func_name = extract_callable_before(source_bytes, open_offset);
    if func_name.is_empty() {
        return None;
    }

    // Count commas between open paren and cursor to determine active parameter
    let active_param = count_commas_between(source_bytes, open_offset, offset);

    // Look up function signature
    let (label, params, ret, doc_text, has_overload) =
        find_signature(bindings, &func_name, &doc.scope, full_source);

    let param_info: Vec<ParameterInformation> = params
        .iter()
        .enumerate()
        .map(|(i, (pname, ptype))| {
            let label = if ptype.is_empty() {
                pname.clone()
            } else {
                format!("{}: {}", pname, ptype)
            };
            ParameterInformation {
                label: ParameterLabel::Simple(label),
                documentation: Some(Documentation::String(format!("Parameter {}", i + 1))),
            }
        })
        .collect();

    let active = active_param.min(param_info.len().saturating_sub(1));
    let label_str = if ret.is_empty() {
        format!(
            "{}({})",
            label,
            params
                .iter()
                .map(|(n, t)| if t.is_empty() {
                    n.clone()
                } else {
                    format!("{}: {}", n, t)
                })
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        format!(
            "{}({}) -> {}",
            label,
            params
                .iter()
                .map(|(n, t)| if t.is_empty() {
                    n.clone()
                } else {
                    format!("{}: {}", n, t)
                })
                .collect::<Vec<_>>()
                .join(", "),
            ret
        )
    };

    let doc = if doc_text.is_empty() {
        None
    } else {
        Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc_text,
        }))
    };

    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label: label_str,
            documentation: doc,
            parameters: Some(param_info),
            active_parameter: if has_overload {
                Some(active as u32)
            } else {
                None
            },
        }],
        active_signature: Some(0),
        active_parameter: Some(active as u32),
    })
}

fn find_opening_paren(bytes: &[u8], cursor: usize) -> Option<(usize, String)> {
    let mut depth: i32 = 0;
    let mut i = cursor.saturating_sub(1);
    loop {
        if i >= bytes.len() {
            break;
        }
        let c = bytes[i] as char;
        if c == ')' || c == '}' || c == ']' {
            depth += 1;
        } else if c == '(' || c == '{' || c == '[' {
            if depth == 0 && c == '(' {
                // Found the opening paren for this call
                // Extract text between the paren's predecessor position
                let expr = extract_callable_before(bytes, i);
                return Some((i, expr));
            }
            depth = depth.saturating_sub(1);
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    None
}

fn extract_callable_before(bytes: &[u8], paren_pos: usize) -> String {
    if paren_pos == 0 {
        return String::new();
    }
    let mut end = paren_pos;
    // Skip whitespace between function name and (
    while end > 0 && (bytes[end - 1] as char).is_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return String::new();
    }

    let mut start = end;
    // Walk back through alphanumeric, underscore, dot, colon
    loop {
        if start == 0 {
            break;
        }
        let c = bytes[start - 1] as char;
        if c.is_alphanumeric() || c == '_' || c == '.' || c == ':' {
            start -= 1;
        } else {
            break;
        }
    }

    std::str::from_utf8(&bytes[start..end])
        .unwrap_or("")
        .to_string()
}

fn count_commas_between(bytes: &[u8], open: usize, cursor: usize) -> usize {
    let mut depth: i32 = 0;
    let mut count = 0;
    for i in open + 1..cursor.min(bytes.len()) {
        let c = bytes[i] as char;
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    count
}

fn find_signature(
    bindings: &Bindings,
    func_name: &str,
    scope: &crate::lsp::store::ScopeMap,
    source: &str,
) -> (String, Vec<(String, String)>, String, String, bool) {
    let parts: Vec<&str> = func_name.split('.').collect();

    // Handle method calls: obj:method(...)
    if let Some(colon_pos) = func_name.rfind(':') {
        let obj_part = &func_name[..colon_pos];
        let method = &func_name[colon_pos + 1..];
        if let Some(type_name) = resolve_expression_type(obj_part, bindings, scope, source) {
            if let Some(m) = bindings
                .get_all_methods(&type_name)
                .iter()
                .find(|m| m.name.eq_ignore_ascii_case(method))
            {
                let params: Vec<(String, String)> = m
                    .params
                    .iter()
                    .map(|p| (format!("self: {}", p.name), p.r#type.clone()))
                    .collect();
                let has_over = bindings
                    .get_all_methods(&type_name)
                    .iter()
                    .filter(|m2| m2.name.eq_ignore_ascii_case(method))
                    .count()
                    > 1;
                return (
                    format!("{}.{}", type_name, method),
                    params,
                    m.returns.clone(),
                    m.description.clone(),
                    has_over,
                );
            }
        }
        let params = vec![
            ("self".into(), String::new()),
            ("...".into(), String::new()),
        ];
        return (
            func_name.to_string(),
            params,
            String::new(),
            String::new(),
            false,
        );
    }

    // Handle dotted calls: Vector3.new(...)
    if parts.len() > 1 {
        let class = parts[0];
        let method = parts[1];
        if let Some(m) = bindings
            .get_all_methods(class)
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(method))
        {
            let params: Vec<(String, String)> = m
                .params
                .iter()
                .map(|p| (p.name.clone(), p.r#type.clone()))
                .collect();
            let has_over = bindings
                .get_all_methods(class)
                .iter()
                .filter(|m2| m2.name.eq_ignore_ascii_case(method))
                .count()
                > 1;
            return (
                func_name.to_string(),
                params,
                m.returns.clone(),
                m.description.clone(),
                has_over,
            );
        }
    }

    // Simple function call
    if let Some(f) = bindings
        .functions
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case(func_name))
    {
        let params: Vec<(String, String)> = f
            .params
            .iter()
            .map(|p| (p.name.clone(), p.r#type.clone()))
            .collect();
        let has_over = bindings
            .functions
            .iter()
            .filter(|f2| f2.name.eq_ignore_ascii_case(func_name))
            .count()
            > 1;
        return (
            f.name.clone(),
            params,
            f.returns.clone(),
            f.description.clone(),
            has_over,
        );
    }

    // Fallback with no args
    (
        func_name.to_string(),
        vec![],
        String::new(),
        String::new(),
        false,
    )
}

pub fn handle_document_symbols(
    store: &mut DocumentStore,
    params: DocumentSymbolParams,
) -> Option<DocumentSymbolResponse> {
    let uri = &params.text_document.uri;
    store.reparse_if_dirty(uri);
    let doc = store.get(uri)?;

    let symbols: Vec<DocumentSymbol> = doc
        .symbols
        .iter()
        .map(|s| {
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
                    start: Position {
                        line: (s.location.line as u32).saturating_sub(1),
                        character: (s.location.column as u32).saturating_sub(1),
                    },
                    end: Position {
                        line: (s.location.end_line as u32).saturating_sub(1),
                        character: (s.location.end_column as u32).saturating_sub(1),
                    },
                },
                selection_range: Range {
                    start: Position {
                        line: (s.location.line as u32).saturating_sub(1),
                        character: (s.location.column as u32).saturating_sub(1),
                    },
                    end: Position {
                        line: (s.location.line as u32).saturating_sub(1),
                        character: (s.location.column as u32).saturating_sub(1)
                            + s.name.len() as u32,
                    },
                },
                children: None,
                deprecated: None,
                tags: None,
            }
        })
        .collect();

    Some(DocumentSymbolResponse::Nested(symbols))
}
