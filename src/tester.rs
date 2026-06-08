use crate::analyze::Diagnostic;
use crate::api_db::ApiDatabase;
use crate::ast::{Expr, Stmt, TableField};
use crate::constants::ROBLOX_GLOBALS;
use crate::luau_checker::{validate, CheckConfig};
use crate::roblox_context::ScriptType;
use std::collections::HashSet;

pub struct TestConfig {
    pub strict: bool,
    pub fail_fast: bool,
    pub check_dead_code: bool,
    pub check_unused_vars: bool,
    pub check_unused_imports: bool,
    pub check_api_conformance: bool,
    pub check_nil_safety: bool,
    pub api_db_path: Option<String>,
    pub script_type: ScriptType,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            strict: false,
            fail_fast: false,
            check_dead_code: true,
            check_unused_vars: true,
            check_unused_imports: true,
            check_api_conformance: true,
            check_nil_safety: true,
            api_db_path: None,
            script_type: ScriptType::Module,
        }
    }
}

pub struct TestStats {
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
}

pub struct TestResult {
    pub passed: bool,
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
    pub stats: TestStats,
}

pub fn run_test(source: &str, file_path: &str, config: &TestConfig) -> TestResult {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut total_checks = 0;
    let mut passed = 0;

    // Check 1: Parse
    let ast = match crate::tokenize_and_parse(source) {
        Ok(a) => {
            passed += 1;
            a
        }
        Err(e) => {
            errors.push(Diagnostic {
                line: 1,
                column: 1,
                message: format!("Parse error: {}", e),
                severity: "error".into(),
                suggestion: None,
            });
            return TestResult {
                passed: false,
                errors,
                warnings,
                stats: TestStats { total_checks: 1, passed: 0, failed: 1, warnings: 0 },
            };
        }
    };
    total_checks += 1;

    // Check 2: Transpile (generates Luau, validates output syntax)
    let luau = match crate::generator::generate(&ast, false, None, None, None, &[], "out") {
        luau => {
            passed += 1;
            luau
        }
    };
    total_checks += 1;

    // Check 3: Luau output syntax (simple brace check)
    let brace_ok = luau.matches('{').count() == luau.matches('}').count();
    if brace_ok {
        passed += 1;
    } else {
        errors.push(Diagnostic {
            line: 0, column: 0,
            message: "Generated Luau has unbalanced braces".into(),
            severity: "error".into(),
            suggestion: Some("Check the Wolfram source for mismatched { }".into()),
        });
    }
    total_checks += 1;

    // Check 4: Dead code detection
    if config.check_dead_code {
        let (dead, _pass) = check_dead_code(source, &ast);
        total_checks += 1;
        if dead.is_empty() {
            passed += 1;
        } else {
            for d in dead { warnings.push(d); }
        }
    }

    // Check 5: Unused variables
    if config.check_unused_vars {
        let unused = check_unused_variables(&ast);
        total_checks += 1;
        if unused.is_empty() {
            passed += 1;
        } else {
            for u in unused { warnings.push(u); }
        }
    }

    // Check 6: Unused imports
    if config.check_unused_imports {
        let unused_imp = check_unused_imports(&ast, source);
        total_checks += 1;
        if unused_imp.is_empty() {
            passed += 1;
        } else {
            for u in unused_imp { warnings.push(u); }
        }
    }

    // Check 7: API conformance via validator
    if config.check_api_conformance {
        let api_db = if let Some(ref path) = config.api_db_path {
            ApiDatabase::load_from_file(&std::path::Path::new(path))
        } else {
            ApiDatabase::empty()
        };

        let checker_config = CheckConfig {
            script_type: config.script_type,
            file_path: file_path.to_string(),
            source: source.to_string(),
            check_roblox_api: api_db.is_loaded(),
            check_circular_deps: false,
            check_nil_safety: config.check_nil_safety,
            check_patterns: true,
            dependency_graph: None,
        };
        let mut validation = validate(&ast, checker_config);
        total_checks += 1;
        if validation.errors.is_empty() && validation.warnings.is_empty() {
            passed += 1;
        } else {
            errors.append(&mut validation.errors);
            warnings.append(&mut validation.warnings);
        }
    }

    // Check 8: Anti-pattern detection
    total_checks += 1;
    let patterns_ok = check_anti_patterns(source, &mut warnings);
    if patterns_ok { passed += 1; }

    let failed = errors.len();
    let warn_count = warnings.len();

    TestResult {
        passed: errors.is_empty() && (!config.strict || warnings.is_empty()),
        errors,
        warnings,
        stats: TestStats {
            total_checks,
            passed,
            failed,
            warnings: warn_count,
        },
    }
}

fn check_dead_code(source: &str, stmts: &[Stmt]) -> (Vec<Diagnostic>, bool) {
    let mut diagnostics = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (i, _stmt) in stmts.iter().enumerate() {
        if i > 0 {
            if let Stmt::Return { .. } = &stmts[i - 1] {
                let line = lines.len().min(i + 1);
                diagnostics.push(Diagnostic {
                    line,
                    column: 1,
                    message: "unreachable code after return statement".into(),
                    severity: "warning".into(),
                    suggestion: Some("Remove unreachable code or restructure logic".into()),
                });
                break;
            }
            if let Stmt::Break { .. } = &stmts[i - 1] {
                if !matches!(&stmts[i], Stmt::Continue { .. }) {
                    let line = lines.len().min(i + 1);
                    diagnostics.push(Diagnostic {
                        line,
                        column: 1,
                        message: "unreachable code after break statement".into(),
                        severity: "warning".into(),
                        suggestion: None,
                    });
                    break;
                }
            }
        }
    }

    let is_empty = diagnostics.is_empty();
    (diagnostics, is_empty)
}

fn check_unused_variables(stmts: &[Stmt]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut declared = Vec::new();
    let mut used = HashSet::new();

    // Collect declared and used
    for stmt in stmts {
        match stmt {
            Stmt::Local { name, .. } => {
                if !name.starts_with('_') {
                    declared.push(name.clone());
                }
            }
            _ => {}
        }
    }

    // Walk expressions to find identifier usage
    fn collect_idents(expr: &Expr, used: &mut HashSet<String>) {
        match expr {
            Expr::Ident(name) => { used.insert(name.clone()); }
            Expr::Call { args, .. } => { for a in args { collect_idents(a, used); } }
            Expr::MethodCall { obj, args, .. } => { collect_idents(obj, used); for a in args { collect_idents(a, used); } }
            Expr::Member { obj, .. } => collect_idents(obj, used),
            Expr::Index { obj, index } => { collect_idents(obj, used); collect_idents(index, used); }
            Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => { collect_idents(left, used); collect_idents(right, used); }
            Expr::Ternary { cond, then_expr, else_expr } => { collect_idents(cond, used); collect_idents(then_expr, used); collect_idents(else_expr, used); }
            Expr::UnaryMinus(e) | Expr::Not(e) | Expr::Grouping(e) => collect_idents(e, used),
            Expr::Array(elements) => { for e in elements { collect_idents(e, used); } }
            Expr::Table(fields) => {
                for f in fields {
                    match f {
                        TableField::Pair { key, value } => { collect_idents(key, used); collect_idents(value, used); }
                        TableField::Value(v) => { collect_idents(v, used); }
                    }
                }
            }
            Expr::Function { block, .. } => { for s in block { collect_stmt_idents(s, used); } }
            Expr::AwaitExpr(e) => collect_idents(e, used),
            Expr::ListComp { elt, generators } => { collect_idents(elt, used); for g in generators { collect_idents(&g.iter, used); if let Some(ref c) = g.condition { collect_idents(c, used); } } }
            _ => {}
        }
    }
    fn collect_stmt_idents(stmt: &Stmt, used: &mut HashSet<String>) {
        match stmt {
            Stmt::Local { value, .. } | Stmt::Return { value, .. } => { if let Some(v) = value { collect_idents(v, used); } }
            Stmt::Assign { target, value, .. } => { collect_idents(target, used); collect_idents(value, used); }
            Stmt::ExprStmt { expr, .. } => collect_idents(expr, used),
            Stmt::If { cond, then_block, else_if_blocks, else_block, .. } => {
                collect_idents(cond, used);
                for s in then_block { collect_stmt_idents(s, used); }
                for (c, b) in else_if_blocks { collect_idents(c, used); for s in b { collect_stmt_idents(s, used); } }
                if let Some(b) = else_block { for s in b { collect_stmt_idents(s, used); } }
            }
            Stmt::While { cond, block, .. } => { collect_idents(cond, used); for s in block { collect_stmt_idents(s, used); } }
            Stmt::For { iter, block, .. } => { collect_idents(iter, used); for s in block { collect_stmt_idents(s, used); } }
            Stmt::FuncDef { block, .. } => { for s in block { collect_stmt_idents(s, used); } }
            Stmt::ClassDef { body, .. } => { for s in body { collect_stmt_idents(s, used); } }
            _ => {}
        }
    }
    for stmt in stmts {
        collect_stmt_idents(stmt, &mut used);
    }

    for name in &declared {
        if !used.contains(name) && !ROBLOX_GLOBALS.contains(&name.as_str()) {
            diagnostics.push(Diagnostic {
                line: 0, column: 0,
                message: format!("variable '{}' is declared but never used", name),
                severity: "warning".into(),
                suggestion: Some(format!("Remove declaration or prefix with '_' to silence: '_{}'", name)),
            });
        }
    }

    diagnostics
}

fn check_unused_imports(stmts: &[Stmt], source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut imports = Vec::new();
    let used_aliases: HashSet<String> = HashSet::new();
    let _ = used_aliases;

    for stmt in stmts {
        if let Stmt::Import { path, alias, .. } = stmt {
            imports.push((alias.clone(), path.clone()));
        }
    }

    for (alias, _) in &imports {
        let count = source.matches(alias.as_str()).count();
        if count <= 1 { // only the import statement itself
            diagnostics.push(Diagnostic {
                line: 0, column: 0,
                message: format!("import '{}' is never used", alias),
                severity: "warning".into(),
                suggestion: Some("Remove unused import".into()),
            });
        }
    }

    let _ = used_aliases;
    diagnostics
}

fn check_anti_patterns(source: &str, warnings: &mut Vec<Diagnostic>) -> bool {
    let mut ok = true;

    // Pattern: .length usage (already caught by generator but warn pre-transpile)
    for (i, line) in source.lines().enumerate() {
        if line.contains(".length") && !line.contains("\"") {
            warnings.push(Diagnostic {
                line: i + 1, column: line.find(".length").unwrap_or(0) + 1,
                message: ".length is invalid in Luau — use #variable instead".into(),
                severity: "warning".into(),
                suggestion: Some("Replace '.length' with '#' prefix operator".into()),
            });
            ok = false;
        }
        if line.contains("len(") {
            warnings.push(Diagnostic {
                line: i + 1, column: line.find("len(").unwrap_or(0) + 1,
                message: "len() is invalid in Luau — use #variable instead".into(),
                severity: "warning".into(),
                suggestion: Some("Replace 'len(x)' with '#x'".into()),
            });
            ok = false;
        }
        if line.contains("while true") && !line.contains("task.wait") {
            warnings.push(Diagnostic {
                line: i + 1, column: line.find("while true").unwrap_or(0) + 1,
                message: "while true loop without yield may freeze the game".into(),
                severity: "warning".into(),
                suggestion: Some("Add task.wait() or RunService.Heartbeat:Wait() inside the loop".into()),
            });
        }
    }

    ok
}
