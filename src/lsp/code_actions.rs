use crate::lsp::store::DocumentStore;
use lsp_types::*;

pub fn handle_code_action(
    store: &mut DocumentStore,
    params: CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();
    let uri = &params.text_document.uri;

    store.reparse_if_dirty(uri);
    let doc = match store.get(uri) {
        Some(d) => d,
        None => return actions,
    };

    for diag in &params.context.diagnostics {
        if let Some(action) = match_diagnostic(&doc.source, diag) {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: action.title,
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(vec![(uri.clone(), vec![action.edit])].into_iter().collect()),
                    ..Default::default()
                }),
                is_preferred: Some(action.preferred),
                ..Default::default()
            }));
        }
    }

    actions
}

struct QuickFix {
    title: String,
    edit: TextEdit,
    preferred: bool,
}

fn match_diagnostic(source: &str, diag: &Diagnostic) -> Option<QuickFix> {
    let msg = &diag.message.to_lowercase();

    // 1. Undefined variable / potential global
    if msg.contains("undefined variable") || msg.contains("potential global") {
        return fix_undeclared(source, diag);
    }

    // 2. Deprecated API (wait, spawn, delay)
    if msg.contains("deprecated") {
        return fix_deprecated(source, diag);
    }

    // 3. Server-only service in client
    if msg.contains("server-only") {
        return fix_remote_wrapper(source, diag);
    }

    // 4. ModuleScript missing return
    if msg.contains("should return a value") {
        return fix_missing_return(source, diag);
    }

    // 5. Class missing init constructor
    if msg.contains("no 'init' constructor") {
        return fix_missing_init(source, diag);
    }

    None
}

fn fix_undeclared(source: &str, diag: &Diagnostic) -> Option<QuickFix> {
    let var_name = extract_quoted_word(&diag.message)?;
    let target_line = diag.range.start.line as usize;

    // Find the first occurrence of the variable name at or above target_line
    let lines: Vec<&str> = source.lines().collect();
    let mut insert_line = target_line;

    for (i, line) in lines.iter().enumerate() {
        if i <= target_line {
            if let Some(col) = line.find(&var_name) {
                let before = line[..col].chars().last();
                let after = line[col + var_name.len()..].chars().next();
                if is_word_boundary(before) && is_word_boundary(after) {
                    insert_line = i;
                    break;
                }
            }
        }
    }

    // Insert `local var_name = nil` before the first use
    let indent = get_line_indent(&lines, insert_line);
    let insert_text = format!("{}local {} = nil\n{}", indent, var_name, indent);
    let edit = TextEdit {
        range: Range {
            start: Position {
                line: insert_line as u32,
                character: 0,
            },
            end: Position {
                line: insert_line as u32,
                character: 0,
            },
        },
        new_text: insert_text,
    };

    Some(QuickFix {
        title: format!("Declare '{}' as local", var_name),
        edit,
        preferred: true,
    })
}

fn fix_deprecated(source: &str, diag: &Diagnostic) -> Option<QuickFix> {
    // Extract "use 'task.wait' instead" → replacement
    let msg = &diag.message;
    let replacement = if msg.contains("task.wait") {
        "task.wait"
    } else if msg.contains("task.spawn") {
        "task.spawn"
    } else if msg.contains("task.delay") {
        "task.delay"
    } else {
        return None;
    };

    let line = diag.range.start.line as usize;
    let lines: Vec<&str> = source.lines().collect();
    let target_line = lines.get(line).copied().unwrap_or("");

    // Find the deprecated word and its range
    let old_func = if msg.contains("wait(") {
        "wait"
    } else if msg.contains("spawn(") {
        "spawn"
    } else if msg.contains("delay(") {
        "delay"
    } else {
        return None;
    };

    let pos = target_line.find(old_func)?;
    let edit = TextEdit {
        range: Range {
            start: Position {
                line: line as u32,
                character: pos as u32,
            },
            end: Position {
                line: line as u32,
                character: (pos + old_func.len()) as u32,
            },
        },
        new_text: replacement.to_string(),
    };

    Some(QuickFix {
        title: format!("Replace '{}' with '{}'", old_func, replacement),
        edit,
        preferred: true,
    })
}

fn fix_remote_wrapper(source: &str, diag: &Diagnostic) -> Option<QuickFix> {
    let line = diag.range.start.line as usize;
    let indent = source
        .lines()
        .nth(line)
        .map(|l| l.chars().take_while(|c| c.is_whitespace()).count())
        .unwrap_or(0);
    let _pad = " ".repeat(indent.max(4));

    let insert_text = format!(
        "{}-- TODO: Access server-only services via RemoteEvents/RemoteFunctions\n",
        " ".repeat(indent)
    );

    let edit = TextEdit {
        range: Range {
            start: Position {
                line: line as u32,
                character: 0,
            },
            end: Position {
                line: line as u32,
                character: 0,
            },
        },
        new_text: insert_text,
    };

    Some(QuickFix {
        title: "Comment with RemoteFunction pattern note".into(),
        edit,
        preferred: false,
    })
}

fn fix_missing_return(source: &str, _diag: &Diagnostic) -> Option<QuickFix> {
    let lines: Vec<&str> = source.lines().collect();
    let _last_line = lines.len().saturating_sub(1);
    let last_text = *lines.last().unwrap_or(&"");

    let insert_text = if last_text.is_empty() {
        "\nreturn {}".to_string()
    } else {
        "\nreturn {}".to_string()
    };

    let edit = TextEdit {
        range: Range {
            start: Position {
                line: lines.len() as u32,
                character: 0,
            },
            end: Position {
                line: lines.len() as u32,
                character: 0,
            },
        },
        new_text: insert_text,
    };

    Some(QuickFix {
        title: "Add return statement".into(),
        edit,
        preferred: true,
    })
}

fn fix_missing_init(source: &str, diag: &Diagnostic) -> Option<QuickFix> {
    let line = diag.range.start.line as usize;

    // Find the class body and insert init() before the closing brace
    let class_name = extract_class_name(source, line).unwrap_or("MyClass".to_string());

    let insert_text =
        format!("    public function init(self)\n        -- Initialize instance here\n    end\n\n");

    let edit = TextEdit {
        range: Range {
            start: Position {
                line: line as u32 + 1,
                character: 0,
            },
            end: Position {
                line: line as u32 + 1,
                character: 0,
            },
        },
        new_text: insert_text,
    };

    Some(QuickFix {
        title: format!("Add 'init()' constructor to {}", class_name),
        edit,
        preferred: false,
    })
}

fn extract_quoted_word(msg: &str) -> Option<String> {
    let start = msg.find('\'')?;
    let end = msg[start + 1..].find('\'')?;
    Some(msg[start + 1..start + 1 + end].to_string())
}

fn extract_class_name(source: &str, around_line: usize) -> Option<String> {
    for line in source.lines().skip(around_line) {
        if line.contains("class") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(pos) = parts.iter().position(|w| *w == "class") {
                return parts.get(pos + 1).map(|s| s.to_string());
            }
        }
    }
    None
}

fn is_word_boundary(c: Option<char>) -> bool {
    match c {
        None => true,
        Some(ch) => !ch.is_alphanumeric() && ch != '_',
    }
}

fn get_line_indent(lines: &[&str], line_idx: usize) -> String {
    let prefix: String = lines
        .get(line_idx)
        .map(|l| l.chars().take_while(|c| c.is_whitespace()).collect())
        .unwrap_or_default();
    prefix
}
