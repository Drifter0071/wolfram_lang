use crate::ast::Stmt;
use crate::LineMapEntry;
use lsp_types::Url;
use std::collections::HashMap;

pub struct DocumentStore {
    documents: HashMap<Url, DocumentState>,
}

#[derive(Debug, Clone)]
pub struct DocumentState {
    pub uri: Url,
    pub source: String,
    pub ast: Vec<Stmt>,
    pub symbols: Vec<crate::analyze::Symbol>,
    pub imports: Vec<crate::analyze::ImportInfo>,
    pub scope: ScopeMap,
    pub dirty: bool,
    pub cached_luau: Option<String>,
    pub line_map: Vec<LineMapEntry>,
    pub last_change_start: Option<usize>,
    pub last_change_end: Option<usize>,
}

#[derive(Default, Debug, Clone)]
pub struct ScopeMap {
    pub variables: HashMap<String, String>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub fn open(&mut self, uri: &Url, source: String) {
        let state = parse_document(uri, source);
        self.documents.insert(uri.clone(), state);
    }

    pub fn update(&mut self, uri: &Url, changes: &str) {
        if let Some(state) = self.documents.get_mut(uri) {
            let old_len = state.source.len();
            state.source = changes.to_string();
            state.dirty = true;
            state.last_change_start = Some(0);
            state.last_change_end = Some(changes.len().max(old_len));
        }
    }

    pub fn update_with_range(
        &mut self,
        uri: &Url,
        text: &str,
        range_start: usize,
        range_end: usize,
    ) {
        if let Some(state) = self.documents.get_mut(uri) {
            state.source = text.to_string();
            state.dirty = true;
            state.last_change_start = Some(range_start);
            state.last_change_end = Some(range_end);
        }
    }

    pub fn set_cached_luau(&mut self, uri: &Url, luau: String, line_map: Vec<LineMapEntry>) {
        if let Some(state) = self.documents.get_mut(uri) {
            state.cached_luau = Some(luau);
            state.line_map = line_map;
        }
    }

    pub fn mark_dirty(&mut self, uri: &Url) {
        if let Some(state) = self.documents.get_mut(uri) {
            state.dirty = true;
        }
    }

    pub fn reparse_if_dirty(&mut self, uri: &Url) {
        let state = self.documents.get(uri).map(|s| s.dirty).unwrap_or(false);
        if state {
            let source = self
                .documents
                .get(uri)
                .map(|s| s.source.clone())
                .unwrap_or_default();
            let new_state = parse_document(uri, source);
            self.documents.insert(uri.clone(), new_state);
        }
    }

    pub fn get(&self, uri: &Url) -> Option<&DocumentState> {
        self.documents.get(uri)
    }

    pub fn get_mut(&mut self, uri: &Url) -> Option<&mut DocumentState> {
        self.documents.get_mut(uri)
    }

    pub fn close(&mut self, uri: &Url) {
        self.documents.remove(uri);
    }

    pub fn iter(&self) -> impl Iterator<Item = &DocumentState> {
        self.documents.values()
    }

    pub fn get_all(&self) -> Vec<&DocumentState> {
        self.documents.values().collect()
    }

    pub fn find_by_file_name(&self, file_name: &str) -> Option<&DocumentState> {
        self.documents.values().find(|d| {
            d.uri.path().ends_with(file_name) || d.uri.path().ends_with(&format!("/{}", file_name))
        })
    }

    pub fn find_by_uri_str(&self, uri_str: &str) -> Option<&DocumentState> {
        self.documents.values().find(|d| d.uri.as_str() == uri_str)
    }
}

pub fn parse_document(uri: &Url, source: String) -> DocumentState {
    let ast = crate::tokenize_and_parse(&source).unwrap_or_default();
    let symbols = crate::analyze::extract_symbols(&ast, &source);
    let imports = crate::analyze::extract_imports(&ast);
    let scope = extract_scope(&ast, &source);

    DocumentState {
        uri: uri.clone(),
        source,
        ast,
        symbols,
        imports,
        scope,
        dirty: false,
        cached_luau: None,
        line_map: Vec::new(),
        last_change_start: None,
        last_change_end: None,
    }
}

fn extract_scope(ast: &[Stmt], source: &str) -> ScopeMap {
    let mut scope = ScopeMap::default();

    for stmt in ast {
        if let Stmt::Local { name, value, .. } = stmt {
            let var_type = value
                .as_ref()
                .map(|v| infer_expr_type(v, source))
                .unwrap_or_else(|| "any".into());
            scope.variables.insert(name.clone(), var_type);
        }

        if let Stmt::FuncDef { name, params, .. } = stmt {
            let param_types: Vec<String> = params.iter().map(|_| "any".into()).collect();
            scope.variables.insert(
                name.clone(),
                format!("function({})", param_types.join(", ")),
            );
        }

        if let Stmt::For { var, .. } = stmt {
            scope.variables.insert(var.clone(), "any".into());
        }
    }

    scope
}

fn infer_expr_type(expr: &crate::ast::Expr, _source: &str) -> String {
    match expr {
        crate::ast::Expr::Number(_) => "number".into(),
        crate::ast::Expr::Str(_) => "string".into(),
        crate::ast::Expr::FString(_) => "string".into(),
        crate::ast::Expr::Bool(_) => "bool".into(),
        crate::ast::Expr::Nil => "nil".into(),
        crate::ast::Expr::Ident(name) => name.clone(),
        crate::ast::Expr::Call { func, .. } => match func.as_str() {
            "Vector3" => "Vector3".into(),
            "Vector2" => "Vector2".into(),
            "CFrame" => "CFrame".into(),
            "Color3" => "Color3".into(),
            "UDim2" => "UDim2".into(),
            "Ray" => "Ray".into(),
            "Region3" => "Region3".into(),
            "DateTime" => "DateTime".into(),
            _ => "any".into(),
        },
        _ => "any".into(),
    }
}
