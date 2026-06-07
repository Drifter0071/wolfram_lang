use lsp_types::*;
use crate::lsp::store::DocumentStore;

pub fn handle_inlay_hint(
    store: &mut DocumentStore,
    params: InlayHintParams,
) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    let uri = &params.text_document.uri;
    store.reparse_if_dirty(uri);
    let doc = match store.get(uri) {
        Some(d) => d,
        None => return hints,
    };

    let lines: Vec<&str> = doc.source.lines().collect();

    for stmt in &doc.ast {
        hints.extend(extract_type_hints(stmt, &lines));
        hints.extend(extract_param_hints(stmt, &lines));
    }

    hints
}

fn extract_type_hints(stmt: &crate::ast::Stmt, lines: &[&str]) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    if let crate::ast::Stmt::Local { name, value, span, .. } = stmt {
        if let Some(val) = value {
            let type_str = infer_var_type(val);
            if type_str != "any" && type_str != *name {
                let start = span.start;
                let prefix = &lines.join("\n")[..start.min(lines.join("\n").len())];
                let line = prefix.bytes().filter(|&b| b == b'\n').count() as u32;
                let col_start = start.saturating_sub(prefix.rfind('\n').map(|i| i + 1).unwrap_or(0));

                // Find position after the variable name
                let var_line = lines.get(line as usize).copied().unwrap_or("");
                let col = col_start + name.len();
                let hint_col = if col < var_line.len() { col } else { col_start + 1 };

                hints.push(InlayHint {
                    position: Position { line, character: hint_col as u32 },
                    label: InlayHintLabel::String(format!(": {}", type_str)),
                    kind: Some(InlayHintKind::TYPE),
                    padding_left: Some(false),
                    padding_right: Some(true),
                    text_edits: None,
                    tooltip: None,
                    data: None,
                });
            }
        }
    }
    hints
}

fn extract_param_hints(_stmt: &crate::ast::Stmt, _lines: &[&str]) -> Vec<InlayHint> {
    // Parameter name hints for function calls with >= 3 args
    // This requires walking the AST for all Call/MethodCall nodes
    // For now, return empty — will be enhanced with expression tree walk
    Vec::new()
}

fn infer_var_type(expr: &crate::ast::Expr) -> String {
    match expr {
        crate::ast::Expr::Number(_) => "number".into(),
        crate::ast::Expr::Str(_) => "string".into(),
        crate::ast::Expr::FString(_) => "string".into(),
        crate::ast::Expr::Bool(_) => "boolean".into(),
        crate::ast::Expr::Nil => "nil".into(),
        crate::ast::Expr::Array(_) => "array".into(),
        crate::ast::Expr::Table(_) => "table".into(),
        crate::ast::Expr::Call { func, .. } => {
            match func.as_str() {
                "Vector3" => "Vector3".into(),
                "Vector2" => "Vector2".into(),
                "CFrame" => "CFrame".into(),
                "Color3" => "Color3".into(),
                "UDim2" => "UDim2".into(),
                "UDim" => "UDim".into(),
                "BrickColor" => "BrickColor".into(),
                "TweenInfo" => "TweenInfo".into(),
                "Ray" => "Ray".into(),
                "Region3" => "Region3".into(),
                "DateTime" => "DateTime".into(),
                "Instance" => "Instance".into(),
                _ => "any".into(),
            }
        }
        crate::ast::Expr::MethodCall { field, .. } => {
            if field == "GetService" { "Instance".into() }
            else { "any".into() }
        }
        crate::ast::Expr::ListComp { .. } => "array".into(),
        crate::ast::Expr::Function { .. } => "function".into(),
        _ => "any".into(),
    }
}
