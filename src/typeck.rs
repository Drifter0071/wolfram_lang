use crate::ast::{Stmt, Expr, TableField};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum InferredType {
    Number,
    String,
    Bool,
    Nil,
    Unknown,
    Array(Box<InferredType>),
    Table,
    Function,
    Instance,
    Vector3,
    Vector2,
    CFrame,
    UDim2,
    UDim,
    Color3,
    BrickColor,
}

impl InferredType {
    pub fn name(&self) -> &'static str {
        match self {
            InferredType::Number => "number",
            InferredType::String => "string",
            InferredType::Bool => "boolean",
            InferredType::Nil => "nil",
            InferredType::Unknown => "unknown",
            InferredType::Array(_) => "array",
            InferredType::Table => "table",
            InferredType::Function => "function",
            InferredType::Instance => "Instance",
            InferredType::Vector3 => "Vector3",
            InferredType::Vector2 => "Vector2",
            InferredType::CFrame => "CFrame",
            InferredType::UDim2 => "UDim2",
            InferredType::UDim => "UDim",
            InferredType::Color3 => "Color3",
            InferredType::BrickColor => "BrickColor",
        }
    }
}

#[derive(Default)]
struct TypeScope {
    vars: HashMap<String, InferredType>,
}

pub struct TypeCheckResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl TypeCheckResult {
    pub fn new() -> Self {
        Self { errors: Vec::new(), warnings: Vec::new() }
    }
}

const ROBLOX_TYPES: &[(&str, InferredType)] = &[
    ("game", InferredType::Instance),
    ("workspace", InferredType::Instance),
    ("script", InferredType::Instance),
    ("Vector3", InferredType::Vector3),
    ("Vector2", InferredType::Vector2),
    ("CFrame", InferredType::CFrame),
    ("UDim2", InferredType::UDim2),
    ("UDim", InferredType::UDim),
    ("Color3", InferredType::Color3),
    ("BrickColor", InferredType::BrickColor),
];

fn roblox_type_for(name: &str) -> Option<InferredType> {
    ROBLOX_TYPES.iter().find(|(n, _)| *n == name).map(|(_, t)| t.clone())
}

const ROBLOX_GLOBALS: &[&str] = &[
    "game", "workspace", "script", "Players", "ReplicatedStorage",
    "ServerScriptService", "ServerStorage", "StarterPlayer", "StarterGui",
    "Lighting", "SoundService", "RunService", "UserInputService",
    "ContextActionService", "TweenService", "CollectionService",
    "HttpService", "TeleportService", "MarketplaceService",
    "DataStoreService", "MessagingService", "PathfindingService",
    "PhysicsService", "Teams", "Chat", "LocalizationService",
    "SocialService", "VRService", "GroupService", "PolicyService",
    "AnalyticsService", "AvatarEditorService", "BadgeService",
    "MemoryStoreService", "TextService", "GuiService", "HapticService",
    "Enum", "math", "string", "table", "os", "task", "coroutine", "debug",
    "utf8", "bit32", "buffer",
];

pub struct TypeChecker {
    result: TypeCheckResult,
    scopes: Vec<TypeScope>,
}

impl TypeChecker {
    pub fn check(stmts: &[Stmt]) -> TypeCheckResult {
        let mut checker = TypeChecker {
            result: TypeCheckResult::new(),
            scopes: vec![TypeScope::default()],
        };
        for g in ROBLOX_GLOBALS {
            checker.scopes[0].vars.insert(g.to_string(), InferredType::Instance);
        }
        for (name, ty) in ROBLOX_TYPES {
            checker.scopes[0].vars.insert(name.to_string(), ty.clone());
        }
        checker.scopes[0].vars.insert("print".into(), InferredType::Function);
        checker.scopes[0].vars.insert("warn".into(), InferredType::Function);
        checker.scopes[0].vars.insert("error".into(), InferredType::Function);
        checker.walk_stmts(stmts);
        checker.result
    }

    fn push_scope(&mut self) { self.scopes.push(TypeScope::default()); }
    fn pop_scope(&mut self) { self.scopes.pop(); }

    fn declare(&mut self, name: &str, ty: InferredType) {
        if let Some(s) = self.scopes.last_mut() { s.vars.insert(name.to_string(), ty); }
    }

    fn lookup(&self, name: &str) -> InferredType {
        for s in self.scopes.iter().rev() {
            if let Some(t) = s.vars.get(name) { return t.clone(); }
        }
        InferredType::Unknown
    }

    fn walk_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts { self.walk_stmt(stmt); }
    }

    fn walk_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Local { name, value, .. } => {
                let ty = value.as_ref().map(|v| self.check_expr(v)).unwrap_or(InferredType::Unknown);
                self.declare(name, ty);
            }
            Stmt::Assign { target, value, op, .. } => {
                let target_ty = self.check_expr(target);
                let value_ty = self.check_expr(value);
                if op.is_none() {
                    if target_ty != InferredType::Unknown && value_ty != InferredType::Unknown {
                        if !is_assignable(&target_ty, &value_ty) {
                            self.result.warnings.push(format!(
                                "type mismatch: cannot assign '{}' to '{}'",
                                value_ty.name(), target_ty.name()
                            ));
                        }
                    }
                }
            }
            Stmt::Return { value, .. } => {
                if let Some(v) = value { self.check_expr(v); }
            }
            Stmt::If { cond, then_block, else_if_blocks, else_block, .. } => {
                self.check_expr(cond);
                self.push_scope();
                self.walk_stmts(then_block);
                self.pop_scope();
                for (c, b) in else_if_blocks { self.check_expr(c); self.push_scope(); self.walk_stmts(b); self.pop_scope(); }
                if let Some(b) = else_block { self.push_scope(); self.walk_stmts(b); self.pop_scope(); }
            }
            Stmt::While { cond, block, .. } => {
                self.check_expr(cond); self.push_scope(); self.walk_stmts(block); self.pop_scope();
            }
            Stmt::For { var, iter, block, .. } => {
                self.push_scope();
                let iter_ty = self.check_expr(iter);
                let elem_ty = match &iter_ty {
                    InferredType::Array(inner) => (**inner).clone(),
                    _ => InferredType::Unknown,
                };
                self.declare(var, elem_ty);
                self.walk_stmts(block);
                self.pop_scope();
            }
            Stmt::FuncDef { name, params, param_defaults, block, .. } => {
                self.declare(name, InferredType::Function);
                self.push_scope();
                for p in params { self.declare(p, InferredType::Unknown); }
                for d in param_defaults { if let Some(e) = d { self.check_expr(e); } }
                self.walk_stmts(block);
                self.pop_scope();
            }
            Stmt::ClassDef { name, body, .. } => {
                self.declare(name, InferredType::Table);
                self.walk_stmts(body);
            }
            Stmt::ExprStmt { expr, .. } => { self.check_expr(expr); }
            Stmt::EnumDef { name, .. } => { self.declare(name, InferredType::Table); }
            Stmt::StructDef { name, .. } => { self.declare(name, InferredType::Table); }
            Stmt::Import { alias, .. } => { self.declare(alias, InferredType::Table); }
            Stmt::TryCatch { try_block, catch_clauses, finally_block, .. } => {
                self.push_scope(); self.walk_stmts(try_block); self.pop_scope();
                for (_, var_name, block) in catch_clauses {
                    self.push_scope();
                    if let Some(v) = var_name { self.declare(v, InferredType::String); }
                    self.walk_stmts(block);
                    self.pop_scope();
                }
                if let Some(b) = finally_block { self.push_scope(); self.walk_stmts(b); self.pop_scope(); }
            }
            Stmt::DecoratedStmt { stmt: inner, .. } => self.walk_stmt(inner),
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> InferredType {
        self.infer_expr(expr)
    }

    fn infer_expr(&mut self, expr: &Expr) -> InferredType {
        match expr {
            Expr::Number(_) => InferredType::Number,
            Expr::Str(_) => InferredType::String,
            Expr::FString(_) => InferredType::String,
            Expr::Bool(_) => InferredType::Bool,
            Expr::Nil => InferredType::Nil,
            Expr::Ident(name) => self.lookup(name),
            Expr::SelfExpr => InferredType::Table,
            Expr::UnaryMinus(e) => {
                let inner = self.infer_expr(e);
                if inner != InferredType::Number && inner != InferredType::Unknown {
                    self.result.warnings.push(format!(
                        "unary minus applied to '{}' (expected number)", inner.name()
                    ));
                }
                InferredType::Number
            }
            Expr::Not(e) => { self.infer_expr(e); InferredType::Bool }
            Expr::Grouping(e) => self.infer_expr(e),
            Expr::Array(elements) => {
                let elem_ty = if elements.is_empty() {
                    InferredType::Unknown
                } else {
                    self.infer_expr(&elements[0])
                };
                for e in &elements[1..] { self.infer_expr(e); }
                InferredType::Array(Box::new(elem_ty))
            }
            Expr::Table(fields) => {
                for f in fields {
                    match f {
                        TableField::Pair { key, value } => { self.infer_expr(key); self.infer_expr(value); }
                        TableField::Value(v) => { self.infer_expr(v); }
                    }
                }
                InferredType::Table
            }
            Expr::Index { obj, index } => { self.infer_expr(obj); self.infer_expr(index); InferredType::Unknown }
            Expr::Call { func, args } => {
                for a in args { self.infer_expr(a); }
                self.return_type_of(func, args)
            }
            Expr::MethodCall { obj, field, args, .. } => {
                let obj_ty = self.infer_expr(obj);
                for a in args { self.infer_expr(a); }
                if field == "new" && obj_ty != InferredType::Unknown && obj_ty != InferredType::Table {
                    return obj_ty;
                }
                InferredType::Unknown
            }
            Expr::Member { obj, .. } => { self.infer_expr(obj); InferredType::Unknown }
            Expr::Binary { left, op, right } => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);
                self.check_binary(&lt, op, &rt)
            }
            Expr::Ternary { cond, then_expr, else_expr } => {
                self.infer_expr(cond);
                let tt = self.infer_expr(then_expr);
                let et = self.infer_expr(else_expr);
                if tt == et { tt } else { InferredType::Unknown }
            }
            Expr::Logical { left, right, .. } => { self.infer_expr(left); self.infer_expr(right); InferredType::Bool }
            Expr::AwaitExpr(e) => self.infer_expr(e),
            Expr::Function { params, block } => {
                self.push_scope();
                for p in params { self.declare(p, InferredType::Unknown); }
                self.walk_stmts(block);
                self.pop_scope();
                InferredType::Function
            }
            Expr::ListComp { .. } => InferredType::Array(Box::new(InferredType::Unknown)),
        }
    }

    fn check_binary(&mut self, left: &InferredType, op: &str, right: &InferredType) -> InferredType {
        match op {
            "+" | "-" | "*" | "/" | "%" | "^" => {
                if left == &InferredType::Number && right == &InferredType::Number {
                    return InferredType::Number;
                }
                if left == &InferredType::Vector3 && right == &InferredType::Vector3 {
                    return InferredType::Vector3;
                }
                if left == &InferredType::Vector3 && right == &InferredType::Number {
                    return InferredType::Vector3;
                }
                if left == &InferredType::Number && right == &InferredType::Vector3 {
                    return InferredType::Vector3;
                }
                if left == &InferredType::Vector2 && right == &InferredType::Vector2 {
                    return InferredType::Vector2;
                }
                if left == &InferredType::CFrame && right == &InferredType::CFrame && op == "*" {
                    return InferredType::CFrame;
                }
                if left == &InferredType::CFrame && right == &InferredType::Vector3 && op == "*" {
                    return InferredType::Vector3;
                }
                if left != &InferredType::Unknown && right != &InferredType::Unknown {
                    self.result.warnings.push(format!(
                        "arithmetic '{}' between '{}' and '{}' — may be invalid",
                        op, left.name(), right.name()
                    ));
                }
                InferredType::Unknown
            }
            ".." => {
                if left != &InferredType::Unknown && right != &InferredType::Unknown {
                    if left != &InferredType::String && right != &InferredType::String
                        && left != &InferredType::Number && right != &InferredType::Number
                    {
                        self.result.warnings.push(format!(
                            "concatenation '..' between '{}' and '{}' — unexpected types",
                            left.name(), right.name()
                        ));
                    }
                }
                InferredType::String
            }
            "==" | "~=" | "<" | ">" | "<=" | ">=" => {
                InferredType::Bool
            }
            _ => InferredType::Unknown,
        }
    }

    fn return_type_of(&mut self, func: &str, args: &[Expr]) -> InferredType {
        for a in args { self.infer_expr(a); }
        if let Some(t) = roblox_type_for(func) {
            return t;
        }
        match func {
            "range" => InferredType::Array(Box::new(InferredType::Number)),
            "tonumber" => InferredType::Number,
            "tostring" => InferredType::String,
            "typeof" => InferredType::String,
            "Instance_new" | "Instance.new" => InferredType::Instance,
            "Vector3_new" | "Vector3.new" => InferredType::Vector3,
            "Vector2_new" | "Vector2.new" => InferredType::Vector2,
            "CFrame_new" | "CFrame.new" => InferredType::CFrame,
            "UDim2_new" | "UDim2.new" => InferredType::UDim2,
            "UDim_new" | "UDim.new" => InferredType::UDim,
            "Color3_new" | "Color3.new" => InferredType::Color3,
            "BrickColor_new" | "BrickColor.new" => InferredType::BrickColor,
            _ => InferredType::Unknown,
        }
    }
}

fn is_assignable(target: &InferredType, value: &InferredType) -> bool {
    if target == &InferredType::Unknown || value == &InferredType::Unknown {
        return true;
    }
    if target == value {
        return true;
    }
    if value == &InferredType::Nil {
        return true;
    }
    if let InferredType::Array(_) = target {
        return matches!(value, InferredType::Array(_) | InferredType::Table);
    }
    false
}

pub fn check_types(stmts: &[Stmt]) -> TypeCheckResult {
    TypeChecker::check(stmts)
}
