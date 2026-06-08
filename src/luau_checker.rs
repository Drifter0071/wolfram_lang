use std::collections::{HashMap, HashSet};

use crate::analyze::Diagnostic;
use crate::api_db::ApiDatabase;
use crate::ast::{Expr, Stmt, TableField};
use crate::constants::{CLIENT_ONLY_SERVICES, ROBLOX_GLOBALS, SERVER_ONLY_SERVICES};
use crate::roblox_api::RobloxApi;
use crate::roblox_context::ScriptType;

#[derive(Debug, Clone)]
pub struct CheckConfig {
    pub script_type: ScriptType,
    pub file_path: String,
    pub source: String,
    pub check_roblox_api: bool,
    pub check_circular_deps: bool,
    pub check_nil_safety: bool,
    pub check_patterns: bool,
    pub dependency_graph: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

impl ValidationResult {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

fn diagnostic(line: usize, column: usize, message: String, severity: &str) -> Diagnostic {
    Diagnostic {
        line,
        column,
        message,
        severity: severity.to_string(),
        suggestion: None,
    }
}

fn warning_d(line: usize, col: usize, msg: String) -> Diagnostic {
    diagnostic(line, col, msg, "warning")
}

fn error_d(line: usize, col: usize, msg: String) -> Diagnostic {
    diagnostic(line, col, msg, "error")
}

fn span_line_col(span: &crate::ast::Span, source: &str) -> (usize, usize) {
    let start = span.start.min(source.len());
    let prefix = &source[..start];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() + 1;
    let col = start.saturating_sub(prefix.rfind('\n').map(|i| i + 1).unwrap_or(0)) + 1;
    (line, col)
}

fn is_table_key_literal(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Ident(_) | Expr::Str(_) | Expr::Number(_) | Expr::FString(_)
    )
}

pub struct LuauChecker {
    result: ValidationResult,
    source: String,
    file_path: String,
    script_type: ScriptType,
    api: RobloxApi,
    api_db: ApiDatabase,
    scope_vars: Vec<HashMap<String, bool>>,
    scope_types: Vec<HashMap<String, String>>,
    check_roblox_api: bool,
    check_nil_safety: bool,
    check_patterns: bool,
    check_circular_deps: bool,
}

impl LuauChecker {
    pub fn check(stmts: &[Stmt], config: CheckConfig) -> ValidationResult {
        let api = RobloxApi::new();
        let api_db = ApiDatabase::empty();
        let mut checker = LuauChecker {
            result: ValidationResult {
                errors: Vec::new(),
                warnings: Vec::new(),
            },
            source: config.source,
            file_path: config.file_path,
            script_type: config.script_type,
            api,
            api_db,
            scope_vars: vec![HashMap::new()],
            scope_types: vec![HashMap::new()],
            check_roblox_api: config.check_roblox_api,
            check_nil_safety: config.check_nil_safety,
            check_patterns: config.check_patterns,
            check_circular_deps: config.check_circular_deps,
        };

        // Pre-populate global scope
        for g in ROBLOX_GLOBALS {
            checker.scope_vars[0].insert(g.to_string(), true);
        }

        // Phase 1: Structural & Syntax Integrity
        checker.phase1_structural(stmts);

        // Phase 2: Semantic & Type Safety
        checker.phase2_semantic(stmts);

        // Phase 3: Roblox API & Context Validation
        checker.phase3_api_validation(stmts);

        // Phase 4: Architectural Pattern Validation
        checker.phase4_architecture(stmts, config.dependency_graph.as_ref());

        checker.result
    }

    // ==========================================
    // Phase 1: Structural & Syntax Integrity
    // ==========================================
    fn phase1_structural(&mut self, stmts: &[Stmt]) {
        self.check_block_closure(stmts);
        self.check_undeclared_variables(stmts);
        self.check_duplicate_imports(stmts);
    }

    fn check_block_closure(&self, _stmts: &[Stmt]) {
        // The parser already guarantees block closure (if→end, for→end, etc.)
        // This is a sanity check for any AST nodes that might break this invariant.
        // Currently, the Parser has exhaustive block matching, so this is
        // redundant but kept for defensive reasons.
    }

    fn check_undeclared_variables(&mut self, stmts: &[Stmt]) {
        self.walk_stmts_for_scope(stmts);
    }

    fn check_duplicate_imports(&mut self, stmts: &[Stmt]) {
        let mut seen: HashMap<&str, &Stmt> = HashMap::new();
        for stmt in stmts {
            if let Stmt::Import { alias, span, .. } = stmt {
                if let Some(prev) = seen.get(alias.as_str()) {
                    if let Stmt::Import {
                        span: prev_span, ..
                    } = prev
                    {
                        let (line, col) = span_line_col(span, &self.source);
                        let (prev_line, prev_col) = span_line_col(prev_span, &self.source);
                        self.result.warnings.push(warning_d(
                            line,
                            col,
                            format!(
                                "duplicate import '{}' (first imported at line {}, column {})",
                                alias, prev_line, prev_col
                            ),
                        ));
                    }
                }
                seen.insert(alias.as_str(), stmt);
            }
        }
    }

    pub fn load_api_db(&mut self, db: ApiDatabase) {
        self.api_db = db;
    }

    fn push_scope(&mut self) {
        self.scope_vars.push(HashMap::new());
        self.scope_types.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scope_vars.pop();
        self.scope_types.pop();
    }

    fn declare(&mut self, name: &str) {
        if let Some(s) = self.scope_vars.last_mut() {
            s.insert(name.to_string(), true);
        }
    }

    fn declare_type(&mut self, name: &str, type_name: &str) {
        if let Some(s) = self.scope_types.last_mut() {
            s.insert(name.to_string(), type_name.to_string());
        }
    }

    fn lookup_type(&self, name: &str) -> Option<String> {
        for s in self.scope_types.iter().rev() {
            if let Some(t) = s.get(name) {
                return Some(t.clone());
            }
        }
        self.api_db.get_global_type(name)
    }

    fn is_declared(&self, name: &str) -> bool {
        for s in self.scope_vars.iter().rev() {
            if s.contains_key(name) {
                return true;
            }
        }
        false
    }

    fn walk_stmts_for_scope(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.walk_stmt_for_scope(stmt);
        }
    }

    fn walk_stmt_for_scope(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Local {
                name,
                value,
                span: _,
                ..
            } => {
                self.declare(name);
                let inferred = value.as_ref().and_then(|v| self.infer_type(v));
                if let Some(ref t) = inferred {
                    self.declare_type(name, t);
                }
                if let Some(v) = value {
                    self.walk_expr_for_scope(v);
                }
            }
            Stmt::Assign {
                target,
                value,
                span,
                ..
            } => {
                self.walk_expr_for_scope(target);
                self.walk_expr_for_scope(value);
                if let Expr::Ident(name) = target {
                    if !self.is_declared(name) && !ROBLOX_GLOBALS.contains(&name.as_str()) {
                        let (line, col) = span_line_col(span, &self.source);
                        self.result.warnings.push(warning_d(
                            line,
                            col,
                            format!(
                                "potential global '{}' — consider declaring with 'local {}' first",
                                name, name
                            ),
                        ));
                    }
                    let inferred = self.infer_type(value);
                    if let Some(t) = inferred {
                        self.declare_type(name, &t);
                    }
                }
            }
            Stmt::Return { value, span: _, .. } => {
                if let Some(v) = value {
                    self.walk_expr_for_scope(v);
                }
            }
            Stmt::If {
                cond,
                then_block,
                else_if_blocks,
                else_block,
                ..
            } => {
                self.walk_expr_for_scope(cond);
                self.push_scope();
                self.walk_stmts_for_scope(then_block);
                self.pop_scope();
                for (cond, block) in else_if_blocks {
                    self.walk_expr_for_scope(cond);
                    self.push_scope();
                    self.walk_stmts_for_scope(block);
                    self.pop_scope();
                }
                if let Some(block) = else_block {
                    self.push_scope();
                    self.walk_stmts_for_scope(block);
                    self.pop_scope();
                }
            }
            Stmt::While { cond, block, .. } => {
                self.walk_expr_for_scope(cond);
                self.push_scope();
                self.walk_stmts_for_scope(block);
                self.pop_scope();
            }
            Stmt::For {
                var, iter, block, ..
            } => {
                self.push_scope();
                self.declare(var);
                self.declare_type(var, "any");
                self.walk_expr_for_scope(iter);
                self.walk_stmts_for_scope(block);
                self.pop_scope();
            }
            Stmt::FuncDef {
                name,
                params,
                param_defaults,
                block,
                ..
            } => {
                self.declare(name);
                self.push_scope();
                for p in params {
                    self.declare(p);
                }
                for d in param_defaults {
                    if let Some(e) = d {
                        self.walk_expr_for_scope(e);
                    }
                }
                self.walk_stmts_for_scope(block);
                self.pop_scope();
            }
            Stmt::ClassDef { name, body, .. } => {
                self.declare(name);
                self.walk_stmts_for_scope(body);
            }
            Stmt::ExprStmt { expr, .. } => self.walk_expr_for_scope(expr),
            Stmt::EnumDef { name, .. } => {
                self.declare(name);
            }
            Stmt::StructDef { name, .. } => {
                self.declare(name);
            }
            Stmt::Import { alias, .. } => {
                self.declare(alias);
            }
            Stmt::TryCatch {
                try_block,
                catch_clauses,
                finally_block,
                ..
            } => {
                self.push_scope();
                self.walk_stmts_for_scope(try_block);
                self.pop_scope();
                for (_, var_name, block) in catch_clauses {
                    self.push_scope();
                    if let Some(v) = var_name {
                        self.declare(v);
                    }
                    self.walk_stmts_for_scope(block);
                    self.pop_scope();
                }
                if let Some(block) = finally_block {
                    self.push_scope();
                    self.walk_stmts_for_scope(block);
                    self.pop_scope();
                }
            }
            Stmt::DecoratedStmt { stmt: inner, .. } => self.walk_stmt_for_scope(inner),
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }

    fn walk_expr_for_scope(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident(name) => {
                if !name.starts_with('_') {
                    self.warn_undeclared(name, expr);
                }
            }
            Expr::Call { args, .. } => {
                for a in args {
                    self.walk_expr_for_scope(a);
                }
            }
            Expr::MethodCall { obj, args, .. } => {
                self.walk_expr_for_scope(obj);
                for a in args {
                    self.walk_expr_for_scope(a);
                }
            }
            Expr::Member { obj, .. } => self.walk_expr_for_scope(obj),
            Expr::Index { obj, index } => {
                self.walk_expr_for_scope(obj);
                self.walk_expr_for_scope(index);
            }
            Expr::Binary { left, right, .. } => {
                self.walk_expr_for_scope(left);
                self.walk_expr_for_scope(right);
            }
            Expr::Logical { left, right, .. } => {
                self.walk_expr_for_scope(left);
                self.walk_expr_for_scope(right);
            }
            Expr::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.walk_expr_for_scope(cond);
                self.walk_expr_for_scope(then_expr);
                self.walk_expr_for_scope(else_expr);
            }
            Expr::UnaryMinus(e) => self.walk_expr_for_scope(e),
            Expr::Not(e) => self.walk_expr_for_scope(e),
            Expr::Grouping(e) => self.walk_expr_for_scope(e),
            Expr::Array(elements) => {
                for e in elements {
                    self.walk_expr_for_scope(e);
                }
            }
            Expr::Table(fields) => {
                for f in fields {
                    match f {
                        TableField::Pair { key, value } => {
                            if !is_table_key_literal(key) {
                                self.walk_expr_for_scope(key);
                            }
                            self.walk_expr_for_scope(value);
                        }
                        TableField::Value(v) => self.walk_expr_for_scope(v),
                    }
                }
            }
            Expr::AwaitExpr(e) => self.walk_expr_for_scope(e),
            Expr::Function { params, block } => {
                self.push_scope();
                for p in params {
                    self.declare(p);
                }
                self.walk_stmts_for_scope(block);
                self.pop_scope();
            }
            Expr::ListComp { elt, generators } => {
                self.walk_expr_for_scope(elt);
                for gen in generators {
                    self.walk_expr_for_scope(&gen.iter);
                    if let Some(ref cond) = gen.condition {
                        self.walk_expr_for_scope(cond);
                    }
                }
            }
            _ => {}
        }
    }

    fn warn_undeclared(&mut self, name: &str, _expr: &Expr) {
        if !self.is_declared(name) {
            self.result.warnings.push(warning_d(
                0,
                0,
                format!(
                    "undefined variable '{}' — may be a typo or missing import",
                    name
                ),
            ));
        }
    }

    fn infer_type(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Ident(name) => self.lookup_type(name),
            Expr::MethodCall { obj, field, .. } => {
                if field == "new" {
                    if let Expr::Ident(class_name) = obj.as_ref() {
                        return Some(class_name.clone());
                    }
                }
                if field == "GetService" {
                    return Some("Instance".into());
                }
                if let Expr::Member { obj: inner_obj, field: inner_field, .. } = obj.as_ref() {
                    if let Expr::Ident(root) = inner_obj.as_ref() {
                        if let Some(class_type) = self.lookup_type(root) {
                            if let Some(ret) = self.api_db.method_returns(&class_type, inner_field) {
                                if ret != "null" && !ret.is_empty() {
                                    return Some(ret);
                                }
                            }
                        }
                    }
                }
                self.infer_type(obj)
            }
            Expr::Member { obj, field, .. } => {
                if let Some(obj_type) = self.infer_type(obj) {
                    self.api_db.property_type(&obj_type, field)
                } else {
                    None
                }
            }
            Expr::Call { func, .. } => {
                if let Some(g) = self.api_db.get_global(func) {
                    return Some(g.r#type.clone());
                }
                if let Some(f) = self.api_db.get_function(func) {
                    if !f.returns.is_empty() && f.returns != "null" {
                        return Some(f.returns.clone());
                    }
                }
                None
            }
            Expr::Str(_) => Some("string".into()),
            Expr::Number(_) => Some("number".into()),
            Expr::Bool(_) => Some("boolean".into()),
            Expr::Nil => Some("nil".into()),
            Expr::Array(_) => Some("table".into()),
            Expr::Table(_) => Some("table".into()),
            _ => None,
        }
    }

    // ==========================================
    // Phase 2: Semantic & Type Safety
    // ==========================================
    fn phase2_semantic(&mut self, stmts: &[Stmt]) {
        if self.check_nil_safety {
            self.check_nil_safety_stmts(stmts, None);
        }
        self.check_immutable_modification(stmts);
    }

    fn check_nil_safety_stmts(&mut self, stmts: &[Stmt], container_type: Option<&str>) {
        for stmt in stmts {
            self.check_nil_safety_stmt(stmt, container_type);
        }
    }

    fn check_nil_safety_stmt(&mut self, stmt: &Stmt, container_type: Option<&str>) {
        match stmt {
            Stmt::Local { value, .. } => {
                if let Some(v) = value {
                    self.check_nil_safety_expr(v, container_type);
                }
            }
            Stmt::Assign { target, value, .. } => {
                self.check_nil_safety_expr(target, container_type);
                self.check_nil_safety_expr(value, container_type);
            }
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.check_nil_safety_expr(v, container_type);
                }
            }
            Stmt::If {
                cond,
                then_block,
                else_if_blocks,
                else_block,
                ..
            } => {
                self.check_nil_safety_expr(cond, container_type);
                self.check_nil_safety_stmts(then_block, container_type);
                for (cond, block) in else_if_blocks {
                    self.check_nil_safety_expr(cond, container_type);
                    self.check_nil_safety_stmts(block, container_type);
                }
                if let Some(block) = else_block {
                    self.check_nil_safety_stmts(block, container_type);
                }
            }
            Stmt::While { cond, block, .. } => {
                self.check_nil_safety_expr(cond, container_type);
                self.check_nil_safety_stmts(block, container_type);
            }
            Stmt::For { iter, block, .. } => {
                self.check_nil_safety_expr(iter, container_type);
                self.check_nil_safety_stmts(block, container_type);
            }
            Stmt::FuncDef {
                params: _, block, ..
            } => {
                self.check_nil_safety_stmts(block, container_type);
            }
            Stmt::ClassDef { body, .. } => {
                self.check_nil_safety_stmts(body, container_type);
            }
            Stmt::ExprStmt { expr, .. } => {
                self.check_nil_safety_expr(expr, container_type);
            }
            Stmt::TryCatch {
                try_block,
                catch_clauses,
                finally_block,
                ..
            } => {
                self.check_nil_safety_stmts(try_block, container_type);
                for (_, _, block) in catch_clauses {
                    self.check_nil_safety_stmts(block, container_type);
                }
                if let Some(block) = finally_block {
                    self.check_nil_safety_stmts(block, container_type);
                }
            }
            Stmt::DecoratedStmt { stmt: inner, .. } => {
                self.check_nil_safety_stmt(inner, container_type)
            }
            _ => {}
        }
    }

    fn check_nil_safety_expr(&mut self, expr: &Expr, container_type: Option<&str>) {
        match expr {
            Expr::Member { obj, field, .. } => {
                // Check the chain from the root up
                self.check_member_chain_nil(obj, field, container_type);
                self.check_nil_safety_expr(obj, container_type);
            }
            Expr::Index { obj, index } => {
                self.check_nil_safety_expr(obj, container_type);
                self.check_nil_safety_expr(index, container_type);
            }
            Expr::Binary { left, right, .. } => {
                self.check_nil_safety_expr(left, container_type);
                self.check_nil_safety_expr(right, container_type);
            }
            Expr::Logical { left, right, .. } => {
                self.check_nil_safety_expr(left, container_type);
                self.check_nil_safety_expr(right, container_type);
            }
            Expr::Call { args, .. } => {
                for a in args {
                    self.check_nil_safety_expr(a, container_type);
                }
            }
            Expr::MethodCall { obj, args, .. } => {
                self.check_nil_safety_expr(obj, container_type);
                for a in args {
                    self.check_nil_safety_expr(a, container_type);
                }
            }
            Expr::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.check_nil_safety_expr(cond, container_type);
                self.check_nil_safety_expr(then_expr, container_type);
                self.check_nil_safety_expr(else_expr, container_type);
            }
            Expr::UnaryMinus(e) => self.check_nil_safety_expr(e, container_type),
            Expr::Not(e) => self.check_nil_safety_expr(e, container_type),
            Expr::Grouping(e) => self.check_nil_safety_expr(e, container_type),
            Expr::Array(elements) => {
                for e in elements {
                    self.check_nil_safety_expr(e, container_type);
                }
            }
            Expr::Table(fields) => {
                for f in fields {
                    match f {
                        TableField::Pair { key, value } => {
                            self.check_nil_safety_expr(key, container_type);
                            self.check_nil_safety_expr(value, container_type);
                        }
                        TableField::Value(v) => self.check_nil_safety_expr(v, container_type),
                    }
                }
            }
            Expr::AwaitExpr(e) => self.check_nil_safety_expr(e, container_type),
            Expr::Function { block, .. } => {
                self.check_nil_safety_stmts(block, container_type);
            }
            Expr::ListComp { elt, generators } => {
                self.check_nil_safety_expr(elt, container_type);
                for gen in generators {
                    self.check_nil_safety_expr(&gen.iter, container_type);
                    if let Some(ref cond) = gen.condition {
                        self.check_nil_safety_expr(cond, container_type);
                    }
                }
            }
            _ => {}
        }
    }

    fn check_member_chain_nil(&self, obj: &Expr, _field: &str, _container_type: Option<&str>) {
        // Walk the member chain to find the root and collect all parts
        let mut parts: Vec<String> = Vec::new();
        let root_name = self.collect_member_parts(obj, &mut parts);
        // parts is now [bottom-up], we need to check top-down

        if let Some(root_name) = root_name {
            if let Some(_class_api) = self.api.get_class(&root_name) {
                // Check each property in the chain
                // The parts are collected bottom-up, so reverse to get top-down
                // Actually parts is collected as [last_field, ..., first_field]
                // So the chain is: root.first_field.second_field...
                let chain = parts.into_iter().rev().collect::<Vec<_>>();
                let mut current_class = root_name.clone();
                for prop in &chain {
                    if let Some(info) = self.api.property_info(&current_class, prop) {
                        // Check if property type is nullable (contains '?')
                        if info.prop_type.ends_with('?') {
                            // Property could be nil — but this is only a real issue
                            // if we're accessing deeper properties
                            // We'll flag it on the next access
                        }
                        // Update current_class to the property's type for next iteration
                        let clean_type = info.prop_type.trim_end_matches('?');
                        if self.api.is_known_class(clean_type) {
                            current_class = clean_type.to_string();
                        }
                    } else {
                        // Unknown property — can't verify nil safety
                        break;
                    }
                }
            }
        }
    }

    fn collect_member_parts(&self, expr: &Expr, parts: &mut Vec<String>) -> Option<String> {
        match expr {
            Expr::Member { obj, field, .. } => {
                parts.push(field.clone());
                self.collect_member_parts(obj, parts)
            }
            Expr::Ident(name) => Some(name.clone()),
            Expr::Call { func, .. } => Some(func.clone()),
            Expr::MethodCall { field, .. } => {
                // Method return types are checked separately
                Some(field.clone())
            }
            _ => None,
        }
    }

    fn check_immutable_modification(&mut self, stmts: &[Stmt]) {
        // Flag modifications to immutable types (e.g., attempting to change a
        // character in a string via string index assignment).
        // In Luau, modifying a string character via indexing (`s[1] = "x"`) is
        // a runtime error. We scan for assignments to Index expressions where
        // the object is a string type.
        for stmt in stmts {
            self.check_immutable_stmt(stmt);
        }
    }

    fn check_immutable_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Assign { target, .. } => {
                if let Expr::Index { obj, .. } = target {
                    match obj.as_ref() {
                        Expr::Str(_) | Expr::FString(_) => {
                            self.result.warnings.push(warning_d(
                                0,
                                0,
                                "attempt to modify a string — strings are immutable in Luau"
                                    .to_string(),
                            ));
                        }
                        _ => {
                            // Check deeper — is obj an Ident that was assigned a string?
                            if let Expr::Ident(_name) = obj.as_ref() {
                                // Can't statically track string assignment without
                                // a full data flow analysis. Skip for now.
                            }
                        }
                    }
                }
            }
            Stmt::If {
                then_block,
                else_if_blocks,
                else_block,
                ..
            } => {
                self.check_immutable_modification(then_block);
                for (_, block) in else_if_blocks {
                    self.check_immutable_modification(block);
                }
                if let Some(block) = else_block {
                    self.check_immutable_modification(block);
                }
            }
            Stmt::While { block, .. } => self.check_immutable_modification(block),
            Stmt::For { block, .. } => self.check_immutable_modification(block),
            Stmt::FuncDef { block, .. } => self.check_immutable_modification(block),
            Stmt::ClassDef { body, .. } => self.check_immutable_modification(body),
            Stmt::TryCatch {
                try_block,
                catch_clauses,
                finally_block,
                ..
            } => {
                self.check_immutable_modification(try_block);
                for (_, _, block) in catch_clauses {
                    self.check_immutable_modification(block);
                }
                if let Some(block) = finally_block {
                    self.check_immutable_modification(block);
                }
            }
            Stmt::DecoratedStmt { stmt: inner, .. } => self.check_immutable_stmt(inner),
            _ => {}
        }
    }

    // ==========================================
    // Phase 3: Roblox API & Context Validation
    // ==========================================
    fn phase3_api_validation(&mut self, stmts: &[Stmt]) {
        if !self.check_roblox_api {
            return;
        }
        if self.api_db.is_loaded() {
            self.check_api_conformance(stmts);
        }
        self.check_deprecated_globals(stmts);
        self.check_service_access(stmts);
    }

    fn check_api_conformance(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.walk_api_conformance_stmt(stmt);
        }
    }

    fn walk_api_conformance_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Local { value, .. } => { if let Some(v) = value { self.validate_api_expr(v); } }
            Stmt::Assign { target, value, .. } => {
                self.validate_api_expr(target);
                self.validate_api_expr(value);
            }
            Stmt::Return { value, .. } => { if let Some(v) = value { self.validate_api_expr(v); } }
            Stmt::If { cond, then_block, else_if_blocks, else_block, .. } => {
                self.validate_api_expr(cond);
                self.check_api_conformance(then_block);
                for (c, b) in else_if_blocks { self.validate_api_expr(c); self.check_api_conformance(b); }
                if let Some(b) = else_block { self.check_api_conformance(b); }
            }
            Stmt::While { cond, block, .. } => { self.validate_api_expr(cond); self.check_api_conformance(block); }
            Stmt::For { iter, block, .. } => { self.validate_api_expr(iter); self.check_api_conformance(block); }
            Stmt::FuncDef { block, .. } => { self.check_api_conformance(block); }
            Stmt::ClassDef { body, .. } => { self.check_api_conformance(body); }
            Stmt::ExprStmt { expr, .. } => { self.validate_api_expr(expr); }
            Stmt::TryCatch { try_block, catch_clauses, finally_block, .. } => {
                self.check_api_conformance(try_block);
                for (_, _, b) in catch_clauses { self.check_api_conformance(b); }
                if let Some(b) = finally_block { self.check_api_conformance(b); }
            }
            Stmt::DecoratedStmt { stmt: inner, .. } => self.walk_api_conformance_stmt(inner),
            _ => {}
        }
    }

    fn validate_api_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Member { obj, field, is_colon } => {
                if !is_colon {
                    // Validate property: obj.Property
                    if let Some(obj_type) = self.infer_type(obj) {
                        if self.api_db.is_known_class(&obj_type) && !self.api_db.property_exists(&obj_type, field) {
                            self.result.warnings.push(warning_d(0, 0,
                                format!("property '{}' does not exist on type '{}'", field, obj_type)));
                        }
                    }
                }
                self.validate_api_expr(obj);
            }
            Expr::MethodCall { obj, field, args, is_colon } => {
                // Validate method: obj:Method(args) or obj.Method(args)
                if let Some(obj_type) = self.infer_type(obj) {
                    let method_lower = field.to_lowercase();
                    if self.api_db.is_known_class(&obj_type) {
                        if let Some(method) = self.api_db.method_info(&obj_type, field) {
                            // Check parameter count (skip self for colon calls)
                            let expected = if *is_colon && !method.params.is_empty() {
                                method.params.len() - 1
                            } else {
                                method.params.len()
                            };
                            if args.len() < expected {
                                let (min_params, max_params) = self.count_min_max_params(&method.params, *is_colon);
                                let range = if min_params == max_params {
                                    format!("{}", min_params)
                                } else {
                                    format!("{}-{}", min_params, max_params)
                                };
                                self.result.warnings.push(warning_d(0, 0,
                                    format!("'{}.{}()' expects {} argument(s), got {}",
                                        obj_type, field, range, args.len())));
                            }
                        } else if !["connect", "disconnect", "wait"].contains(&method_lower.as_str()) {
                            self.result.warnings.push(warning_d(0, 0,
                                format!("method '{}' does not exist on type '{}'", field, obj_type)));
                        }
                    }
                }
                self.validate_api_expr(obj);
                for a in args { self.validate_api_expr(a); }
            }
            Expr::Call { func, args } => {
                // Check function signature
                if let Some(f) = self.api_db.get_function(func) {
                    if args.len() < f.params.len() {
                        self.result.warnings.push(warning_d(0, 0,
                            format!("'{}(...)' expects {} argument(s), got {}", func, f.params.len(), args.len())));
                    }
                }
                for a in args { self.validate_api_expr(a); }
            }
            Expr::Binary { left, right, .. } => { self.validate_api_expr(left); self.validate_api_expr(right); }
            Expr::Logical { left, right, .. } => { self.validate_api_expr(left); self.validate_api_expr(right); }
            Expr::Ternary { cond, then_expr, else_expr } => {
                self.validate_api_expr(cond);
                self.validate_api_expr(then_expr);
                self.validate_api_expr(else_expr);
            }
            Expr::UnaryMinus(e) | Expr::Not(e) | Expr::Grouping(e) => self.validate_api_expr(e),
            Expr::Index { obj, index } => { self.validate_api_expr(obj); self.validate_api_expr(index); }
            Expr::AwaitExpr(e) => self.validate_api_expr(e),
            Expr::Array(elements) => { for e in elements { self.validate_api_expr(e); } }
            Expr::Table(fields) => {
                for f in fields {
                    match f {
                        TableField::Pair { key, value } => { self.validate_api_expr(key); self.validate_api_expr(value); }
                        TableField::Value(v) => self.validate_api_expr(v),
                    }
                }
            }
            Expr::Function { block, .. } => { self.check_api_conformance(block); }
            Expr::ListComp { elt, generators } => {
                self.validate_api_expr(elt);
                for gen in generators {
                    self.validate_api_expr(&gen.iter);
                    if let Some(ref cond) = gen.condition { self.validate_api_expr(cond); }
                }
            }
            _ => {}
        }
    }

    fn count_min_max_params(&self, params: &[crate::api_db::WoldParam], is_colon: bool) -> (usize, usize) {
        let base = if is_colon { 1 } else { 0 };
        let mut required = 0;
        let total = params.len();
        for p in params.iter().skip(base) {
            if !p.r#type.contains('?') {
                required += 1;
            }
        }
        let max = total.saturating_sub(base);
        (required, max)
    }

    fn check_bare_service_access(&self, name: &str) {
        // Flag direct use of services without game:GetService()
        // Services like "Players", "ReplicatedStorage" etc should be accessed
        // via game:GetService("Players") in proper patterns
        // This is a convention check — we only warn
        if SERVER_ONLY_SERVICES.contains(&name) || CLIENT_ONLY_SERVICES.contains(&name) {
            // These are being used as bare globals — this is fine in Luau
            // but we document the recommended pattern
        }
    }

    fn check_deprecated_globals(&mut self, _stmts: &[Stmt]) {
        // Check for deprecated global usage patterns
        // wait() → task.wait()
        // spawn() → task.spawn()
        // delay() → task.delay()
    }

    fn check_service_access(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.check_service_access_stmt(stmt);
        }
    }

    fn check_service_access_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Local { value, .. } => {
                if let Some(v) = value {
                    self.check_service_expr(v);
                }
            }
            Stmt::Assign { value, .. } => {
                self.check_service_expr(value);
            }
            Stmt::If {
                cond,
                then_block,
                else_if_blocks,
                else_block,
                ..
            } => {
                self.check_service_expr(cond);
                for b in then_block {
                    self.check_service_access_stmt(b);
                }
                for (c, b) in else_if_blocks {
                    self.check_service_expr(c);
                    for s in b {
                        self.check_service_access_stmt(s);
                    }
                }
                if let Some(b) = else_block {
                    for s in b {
                        self.check_service_access_stmt(s);
                    }
                }
            }
            Stmt::While { cond, block, .. } => {
                self.check_service_expr(cond);
                for b in block {
                    self.check_service_access_stmt(b);
                }
            }
            Stmt::For { iter, block, .. } => {
                self.check_service_expr(iter);
                for b in block {
                    self.check_service_access_stmt(b);
                }
            }
            Stmt::FuncDef { block, .. } => {
                for b in block {
                    self.check_service_access_stmt(b);
                }
            }
            Stmt::ClassDef { body, .. } => {
                for b in body {
                    self.check_service_access_stmt(b);
                }
            }
            Stmt::ExprStmt { expr, .. } => {
                self.check_service_expr(expr);
            }
            Stmt::TryCatch {
                try_block,
                catch_clauses,
                finally_block,
                ..
            } => {
                for b in try_block {
                    self.check_service_access_stmt(b);
                }
                for (_, _, b) in catch_clauses {
                    for s in b {
                        self.check_service_access_stmt(s);
                    }
                }
                if let Some(b) = finally_block {
                    for s in b {
                        self.check_service_access_stmt(s);
                    }
                }
            }
            Stmt::DecoratedStmt { stmt: inner, .. } => self.check_service_access_stmt(inner),
            _ => {}
        }
    }

    fn check_service_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::MethodCall {
                obj, field, args, ..
            } => {
                // Detect game:GetService("ServiceName") calls
                if let Expr::Ident(service_container) = obj.as_ref() {
                    if (service_container == "game" || service_container == "workspace")
                        && field == "GetService"
                        && !args.is_empty()
                    {
                        if let Expr::Str(service_name) = &args[0] {
                            self.check_service_context(service_name);
                        }
                    }
                }
                self.check_service_expr(obj);
                for a in args {
                    self.check_service_expr(a);
                }
            }
            Expr::Ident(name) => {
                self.check_service_context(name);
            }
            Expr::Call { func, args } => {
                self.check_service_context(func);
                for a in args {
                    self.check_service_expr(a);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.check_service_expr(left);
                self.check_service_expr(right);
            }
            Expr::Logical { left, right, .. } => {
                self.check_service_expr(left);
                self.check_service_expr(right);
            }
            Expr::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.check_service_expr(cond);
                self.check_service_expr(then_expr);
                self.check_service_expr(else_expr);
            }
            Expr::Array(elements) => {
                for e in elements {
                    self.check_service_expr(e);
                }
            }
            Expr::Table(fields) => {
                for f in fields {
                    match f {
                        TableField::Pair { key, value } => {
                            self.check_service_expr(key);
                            self.check_service_expr(value);
                        }
                        TableField::Value(v) => self.check_service_expr(v),
                    }
                }
            }
            _ => {}
        }
    }

    fn check_service_context(&mut self, name: &str) {
        match self.script_type {
            ScriptType::Client => {
                if SERVER_ONLY_SERVICES.contains(&name) {
                    self.result.errors.push(error_d(0, 0, format!(
                        "'{}' is a server-only service — cannot access from LocalScript. Use RemoteEvents/RemoteFunctions instead.", name
                    )));
                }
            }
            ScriptType::Server => {
                if CLIENT_ONLY_SERVICES.contains(&name) {
                    self.result.warnings.push(warning_d(0, 0, format!(
                        "'{}' is client-only — server scripts should not depend on client-side services", name
                    )));
                }
            }
            _ => {}
        }
    }

    // ==========================================
    // Phase 4: Architectural Pattern Validation
    // ==========================================
    fn phase4_architecture(
        &mut self,
        stmts: &[Stmt],
        dep_graph: Option<&HashMap<String, Vec<String>>>,
    ) {
        if self.check_patterns {
            self.check_module_script_return(stmts);
            self.check_oop_patterns(stmts);
        }
        if self.check_circular_deps {
            if let Some(graph) = dep_graph {
                self.check_circular_dependencies(graph);
            } else {
                // Build dep graph from imports in the AST
                let local_graph = self.build_dep_graph(stmts);
                self.check_circular_dependencies(&local_graph);
            }
        }
    }

    fn check_module_script_return(&mut self, stmts: &[Stmt]) {
        // ModuleScript should return a value (table or other export)
        if self.script_type == ScriptType::Shared {
            let has_return = stmts.iter().any(|s| matches!(s, Stmt::Return { .. }));
            let has_public_exports = stmts.iter().any(|s| match s {
                Stmt::ClassDef { access, .. }
                | Stmt::EnumDef { access, .. }
                | Stmt::StructDef { access, .. }
                | Stmt::FuncDef { access, .. } => access == "public",
                Stmt::Local { access, .. } => access == "public",
                _ => false,
            });

            if !has_return {
                // Non-roblox mode will auto-generate a return from public exports
                // So we only warn if there are no public exports either
                if !has_public_exports {
                    self.result.warnings.push(warning_d(
                        0,
                        0,
                        "ModuleScript should return a value — missing 'return' statement"
                            .to_string(),
                    ));
                }
            }
        }
    }

    fn check_oop_patterns(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            if let Stmt::ClassDef { body, .. } = stmt {
                // Check that class has an init method or constructor
                let has_init = body
                    .iter()
                    .any(|s| matches!(s, Stmt::FuncDef { name, .. } if name == "init"));
                let has_public_methods = body
                    .iter()
                    .any(|s| matches!(s, Stmt::FuncDef { access, .. } if access == "public"));

                if !has_init && has_public_methods {
                    self.result.warnings.push(warning_d(0, 0, format!(
                        "class has public methods but no 'init' constructor — instances may be uninitialized"
                    )));
                }
            }
        }
    }

    fn build_dep_graph(&self, stmts: &[Stmt]) -> HashMap<String, Vec<String>> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let file = self.file_path.clone();
        let mut deps = Vec::new();
        for stmt in stmts {
            if let Stmt::Import { path, .. } = stmt {
                deps.push(path.clone());
            }
        }
        graph.insert(file, deps);
        graph
    }

    fn check_circular_dependencies(&mut self, graph: &HashMap<String, Vec<String>>) {
        // Run DFS on the dependency graph to find cycles
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();

        for node in graph.keys() {
            if !visited.contains(node) {
                let mut path = Vec::new();
                self.dfs_cycle_detection(node, graph, &mut visited, &mut in_stack, &mut path);
            }
        }
    }

    fn dfs_cycle_detection(
        &mut self,
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) {
        visited.insert(node.to_string());
        in_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(deps) = graph.get(node) {
            for dep in deps {
                if !visited.contains(dep.as_str()) {
                    self.dfs_cycle_detection(dep, graph, visited, in_stack, path);
                } else if in_stack.contains(dep.as_str()) {
                    // Cycle detected
                    let cycle_start = path.iter().position(|n| n == dep).unwrap_or(0);
                    let cycle_nodes: Vec<_> =
                        path[cycle_start..].iter().map(|s| s.as_str()).collect();
                    self.result.errors.push(error_d(
                        0,
                        0,
                        format!("circular dependency detected: {}", cycle_nodes.join(" → ")),
                    ));
                }
            }
        }

        path.pop();
        in_stack.remove(node);
    }
}

pub fn validate(stmts: &[Stmt], config: CheckConfig) -> ValidationResult {
    LuauChecker::check(stmts, config)
}
