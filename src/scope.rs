use crate::ast::{Stmt, Expr};
use std::collections::HashMap;

#[derive(Default)]
struct Scope {
    variables: HashMap<String, bool>, // name → declared
}

pub struct ScopeAnalysis {
    warnings: Vec<String>,
    scopes: Vec<Scope>,
}

impl ScopeAnalysis {
    pub fn analyze(stmts: &[Stmt], _source: &str, _file_path: &str) -> Vec<String> {
        let mut analyzer = ScopeAnalysis {
            warnings: Vec::new(),
            scopes: vec![Scope::default()],
        };

        // Pre-populate globals: Roblox services + common builtins
        for g in GLOBALS {
            analyzer.scopes[0].variables.insert(g.to_string(), true);
        }

        analyzer.walk_stmts(stmts);
        analyzer.warnings
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.variables.insert(name.to_string(), true);
        }
    }

    fn is_declared(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.variables.contains_key(name) {
                return true;
            }
        }
        false
    }

    fn warn_undeclared(&mut self, name: &str, context: &str) {
        if !self.is_declared(name) && !is_keyword_or_builtin(name) {
            self.warnings.push(format!("undefined variable '{}' ({})", name, context));
        }
    }

    fn walk_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.walk_stmt(stmt);
        }
    }

    fn walk_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Local { name, value, .. } => {
                self.declare(name);
                if let Some(v) = value { self.walk_expr(v); }
            }
            Stmt::Assign { target, value, .. } => {
                self.walk_expr(target);
                self.walk_expr(value);
            }
            Stmt::Return { value, .. } => {
                if let Some(v) = value { self.walk_expr(v); }
            }
            Stmt::If { cond, then_block, else_if_blocks, else_block, .. } => {
                self.walk_expr(cond);
                self.push_scope();
                self.walk_stmts(then_block);
                self.pop_scope();
                for (cond, block) in else_if_blocks {
                    self.walk_expr(cond);
                    self.push_scope();
                    self.walk_stmts(block);
                    self.pop_scope();
                }
                if let Some(block) = else_block {
                    self.push_scope();
                    self.walk_stmts(block);
                    self.pop_scope();
                }
            }
            Stmt::While { cond, block, .. } => {
                self.walk_expr(cond);
                self.push_scope();
                self.walk_stmts(block);
                self.pop_scope();
            }
            Stmt::For { var, iter, block, .. } => {
                self.push_scope();
                self.declare(var);
                self.walk_expr(iter);
                self.walk_stmts(block);
                self.pop_scope();
            }
            Stmt::FuncDef { name, params, param_defaults, block, .. } => {
                self.declare(name);
                self.push_scope();
                for p in params { self.declare(p); }
                for d in param_defaults {
                    if let Some(e) = d { self.walk_expr(e); }
                }
                self.walk_stmts(block);
                self.pop_scope();
            }
            Stmt::ClassDef { name, body, .. } => {
                self.declare(name);
                self.walk_stmts(body);
            }
            Stmt::ExprStmt { expr, .. } => self.walk_expr(expr),
            Stmt::EnumDef { name, .. } => { self.declare(name); }
            Stmt::StructDef { name, .. } => { self.declare(name); }
            Stmt::Import { alias, .. } => { self.declare(alias); }
            Stmt::TryCatch { try_block, catch_clauses, finally_block, .. } => {
                self.push_scope();
                self.walk_stmts(try_block);
                self.pop_scope();
                for (_, var_name, block) in catch_clauses {
                    self.push_scope();
                    if let Some(v) = var_name { self.declare(v); }
                    self.walk_stmts(block);
                    self.pop_scope();
                }
                if let Some(block) = finally_block {
                    self.push_scope();
                    self.walk_stmts(block);
                    self.pop_scope();
                }
            }
            Stmt::DecoratedStmt { stmt: inner, .. } => self.walk_stmt(inner),
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }

    fn walk_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident(name) => {
                if name != "self" && !name.starts_with('_') {
                    self.warn_undeclared(name, "variable usage");
                }
            }
            Expr::Call { args, .. } => {
                for a in args { self.walk_expr(a); }
            }
            Expr::MethodCall { obj, args, .. } => {
                self.walk_expr(obj);
                for a in args { self.walk_expr(a); }
            }
            Expr::Member { obj, .. } => self.walk_expr(obj),
            Expr::Index { obj, index } => {
                self.walk_expr(obj);
                self.walk_expr(index);
            }
            Expr::Binary { left, right, .. } => {
                self.walk_expr(left);
                self.walk_expr(right);
            }
            Expr::Logical { left, right, .. } => {
                self.walk_expr(left);
                self.walk_expr(right);
            }
            Expr::Ternary { cond, then_expr, else_expr } => {
                self.walk_expr(cond);
                self.walk_expr(then_expr);
                self.walk_expr(else_expr);
            }
            Expr::UnaryMinus(e) => self.walk_expr(e),
            Expr::Not(e) => self.walk_expr(e),
            Expr::Grouping(e) => self.walk_expr(e),
            Expr::Array(elements) => { for e in elements { self.walk_expr(e); } }
            Expr::Table(fields) => {
                for f in fields {
                    match f {
                        crate::ast::TableField::Pair { key, value } => {
                            self.walk_expr(key);
                            self.walk_expr(value);
                        }
                        crate::ast::TableField::Value(v) => self.walk_expr(v),
                    }
                }
            }
            Expr::AwaitExpr(e) => self.walk_expr(e),
            Expr::Function { params, block } => {
                self.push_scope();
                for p in params { self.declare(p); }
                self.walk_stmts(block);
                self.pop_scope();
            }
            _ => {} // Number, Str, Bool, Nil, SelfExpr, FString, ListComp — no variable refs
        }
    }
}

fn is_keyword_or_builtin(name: &str) -> bool {
    matches!(name,
        "if" | "else" | "elif" | "while" | "for" | "in" | "function" |
        "class" | "struct" | "enum" | "import" | "as" | "local" | "return" |
        "true" | "false" | "nil" | "self" | "break" | "continue" |
        "and" | "or" | "not" | "public" | "private" | "try" | "catch" |
        "finally" | "async" | "await" | "global" | "is"
    )
}

const GLOBALS: &[&str] = &[
    "game", "workspace", "script", "print", "warn", "error",
    "Players", "ReplicatedStorage", "ServerScriptService", "ServerStorage",
    "StarterPlayer", "StarterGui", "StarterPack", "Lighting", "SoundService",
    "RunService", "UserInputService", "ContextActionService", "TweenService",
    "CollectionService", "HttpService", "TeleportService", "MarketplaceService",
    "DataStoreService", "MessagingService", "PathfindingService", "PhysicsService",
    "Teams", "Chat", "LocalizationService", "SocialService", "GroupService",
    "PolicyService", "AnalyticsService", "AvatarEditorService", "BadgeService",
    "MemoryStoreService", "TextService", "GuiService", "HapticService",
    "Enum", "Vector3", "Vector2", "CFrame", "UDim2", "UDim", "Color3",
    "BrickColor", "TweenInfo", "RaycastParams", "Region3", "Rect",
    "NumberRange", "NumberSequence", "ColorSequence", "Ray", "DateTime",
    "Buffer", "Instance", "PhysicalProperties", "Random", "Axes", "Faces",
    "math", "string", "table", "os", "task", "coroutine", "debug",
    "utf8", "bit32", "buffer", "typeof", "ipairs", "pairs", "next",
    "rawget", "rawset", "setmetatable", "getmetatable", "pcall",
    "xpcall", "tostring", "tonumber", "type", "require",
];
