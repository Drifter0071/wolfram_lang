use std::collections::HashMap;
use lsp_types::Url;
use crate::ast::Stmt;
use crate::parser::Parser;
use crate::lexer::Token;
use logos::Logos;

pub struct DocumentStore {
    documents: HashMap<Url, DocumentState>,
}

pub struct DocumentState {
    pub uri: Url,
    pub source: String,
    pub ast: Vec<Stmt>,
    pub symbols: Vec<crate::analyze::Symbol>,
    pub imports: Vec<crate::analyze::ImportInfo>,
    pub scope: ScopeMap,
    pub dirty: bool,
}

#[derive(Default)]
pub struct ScopeMap {
    pub variables: HashMap<String, String>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self { documents: HashMap::new() }
    }

    pub fn open(&mut self, uri: &Url, source: String) {
        let state = parse_document(uri, source);
        self.documents.insert(uri.clone(), state);
    }

    pub fn update(&mut self, uri: &Url, changes: &str) {
        if let Some(state) = self.documents.get_mut(uri) {
            state.source = changes.to_string();
            state.dirty = true;
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
            let source = self.documents.get(uri).map(|s| s.source.clone()).unwrap_or_default();
            let new_state = parse_document(uri, source);
            self.documents.insert(uri.clone(), new_state);
        }
    }

    pub fn get(&self, uri: &Url) -> Option<&DocumentState> {
        self.documents.get(uri)
    }

    pub fn close(&mut self, uri: &Url) {
        self.documents.remove(uri);
    }
}

fn parse_document(uri: &Url, source: String) -> DocumentState {
    let mut tokens = Vec::new();
    let mut spans = Vec::new();
    for (res, span) in Token::lexer(&source).spanned() {
        if let Ok(tok) = res {
            tokens.push(tok);
            spans.push(span.start);
        }
    }

    let mut parser = Parser::new(tokens, spans, &source);
    let ast = parser.parse_program().unwrap_or_default();
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
    }
}

fn extract_scope(ast: &[Stmt], source: &str) -> ScopeMap {
    let mut scope = ScopeMap::default();

    for stmt in ast {
        if let Stmt::Local { name, value, .. } = stmt {
            let var_type = value.as_ref().map(|v| infer_expr_type(v, source)).unwrap_or_else(|| "any".into());
            scope.variables.insert(name.clone(), var_type);
        }

        if let Stmt::FuncDef { name, params, .. } = stmt {
            let param_types: Vec<String> = params.iter().map(|_| "any".into()).collect();
            scope.variables.insert(name.clone(), format!("function({})", param_types.join(", ")));
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
        crate::ast::Expr::Call { func, .. } => {
            match func.as_str() {
                "Vector3" => "Vector3".into(),
                "Vector2" => "Vector2".into(),
                "CFrame" => "CFrame".into(),
                "Color3" => "Color3".into(),
                "UDim2" => "UDim2".into(),
                "Ray" => "Ray".into(),
                "Region3" => "Region3".into(),
                "DateTime" => "DateTime".into(),
                _ => "any".into(),
            }
        }
        _ => "any".into(),
    }
}
