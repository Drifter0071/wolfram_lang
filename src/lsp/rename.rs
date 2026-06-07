use lsp_types::*;
use crate::lsp::store::DocumentStore;

pub fn handle_prepare_rename(
    store: &mut DocumentStore,
    params: TextDocumentPositionParams,
) -> Option<PrepareRenameResponse> {
    let uri = &params.text_document.uri;
    store.reparse_if_dirty(uri);
    let doc = store.get(uri)?;
    let word = extract_word_at(&doc.source, &params.position);

    if word.is_empty() || is_keyword(&word) {
        return None;
    }

    // Check if word is a declared symbol
    let symbol_exists = doc.symbols.iter().any(|s| s.name == word)
        || doc.scope.variables.contains_key(&word);

    if !symbol_exists {
        return None;
    }

    // Return the word range for default placeholder behavior
    let range = word_range(&doc.source, &params.position);
    Some(PrepareRenameResponse::Range(range))
}

pub fn handle_rename(
    store: &mut DocumentStore,
    params: RenameParams,
) -> Option<WorkspaceEdit> {
    let uri = &params.text_document_position.text_document.uri;
    let pos = &params.text_document_position.position;
    let new_name = &params.new_name;

    store.reparse_if_dirty(uri);
    let doc = store.get(uri)?;
    let old_name = extract_word_at(&doc.source, pos);

    if old_name.is_empty() || is_keyword(&old_name) || old_name == *new_name {
        return None;
    }

    let mut changes: Vec<(Url, Vec<TextEdit>)> = Vec::new();

    // Rename in current file
    let local_edits = find_occurrences(&doc.source, &old_name);
    if !local_edits.is_empty() {
        let new_edits: Vec<TextEdit> = local_edits.into_iter().map(|e| TextEdit {
            range: e,
            new_text: new_name.clone(),
        }).collect();
        changes.push((uri.clone(), new_edits));
    }

    // Rename across workspace files (for public symbols)
    let is_public = doc.symbols.iter().any(|s| s.name == old_name && s.access == "public");
    if is_public {
        for other_doc in store.get_all() {
            if other_doc.uri == *uri { continue; }
            let edits = find_occurrences(&other_doc.source, &old_name);
            if !edits.is_empty() {
                let new_edits: Vec<TextEdit> = edits.into_iter().map(|e| TextEdit {
                    range: e,
                    new_text: new_name.clone(),
                }).collect();
                changes.push((other_doc.uri.clone(), new_edits));
            }
        }
    }

    if changes.is_empty() {
        return None;
    }

    let change_map: std::collections::HashMap<Url, Vec<TextEdit>> = changes.into_iter().collect();
    Some(WorkspaceEdit {
        changes: Some(change_map),
        ..Default::default()
    })
}

fn find_occurrences(source: &str, word: &str) -> Vec<Range> {
    let mut ranges = Vec::new();
    for (line_idx, line) in source.lines().enumerate() {
        let mut start = 0;
        while let Some(found) = line[start..].find(word) {
            let abs = start + found;
            let before = line[..abs].chars().last();
            let after = line[abs + word.len()..].chars().next();
            if is_word_boundary(before) && is_word_boundary(after) {
                ranges.push(Range {
                    start: Position { line: line_idx as u32, character: abs as u32 },
                    end: Position { line: line_idx as u32, character: (abs + word.len()) as u32 },
                });
            }
            start = abs + word.len();
        }
    }
    ranges
}

fn extract_word_at(source: &str, pos: &Position) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(pos.line as usize).copied().unwrap_or("");
    let col = pos.character as usize;
    if col >= line.len() { return String::new(); }

    let bytes = line.as_bytes();
    let start = (0..=col).rev()
        .find(|&i| i == 0 || !is_id_char(bytes[i - 1]))
        .unwrap_or(0);
    let end = (col..line.len())
        .find(|&i| !is_id_char(bytes[i]))
        .unwrap_or(line.len());

    line[start..end].to_string()
}

fn word_range(source: &str, pos: &Position) -> Range {
    let word = extract_word_at(source, pos);
    let lines: Vec<&str> = source.lines().collect();
    let _line = lines.get(pos.line as usize).copied().unwrap_or("");
    let col = pos.character as usize;
    let start = col.saturating_sub(word.len());
    Range {
        start: Position { line: pos.line, character: start as u32 },
        end: Position { line: pos.line, character: col as u32 },
    }
}

fn is_id_char(b: u8) -> bool {
    (b as char).is_alphanumeric() || b == b'_'
}

fn is_word_boundary(c: Option<char>) -> bool {
    match c {
        None => true,
        Some(ch) => !ch.is_alphanumeric() && ch != '_',
    }
}

fn is_keyword(word: &str) -> bool {
    matches!(word, "if" | "else" | "elif" | "while" | "for" | "in" | "function"
        | "class" | "struct" | "enum" | "import" | "as" | "local" | "return"
        | "true" | "false" | "nil" | "self" | "break" | "continue"
        | "and" | "or" | "not" | "public" | "private" | "try" | "catch"
        | "finally" | "async" | "await")
}
