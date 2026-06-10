use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

// ==========================================
// 2. THE AST
// ==========================================
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StructField {
    pub name: String,
    pub field_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompGenerator {
    pub var: String,
    pub iter: Expr,
    pub condition: Option<Expr>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TableField {
    Pair { key: Expr, value: Expr },
    Value(Expr),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Expr {
    Number(f64),
    Str(String),
    FString(String),
    Bool(bool),
    Nil,
    Ident(String),
    SelfExpr,
    UnaryMinus(Box<Expr>),
    Grouping(Box<Expr>),
    Array(Vec<Expr>),
    Table(Vec<TableField>),
    Index {
        obj: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        func: String,
        args: Vec<Expr>,
    },
    MethodCall {
        obj: Box<Expr>,
        field: String,
        is_colon: bool,
        args: Vec<Expr>,
    },
    Member {
        obj: Box<Expr>,
        field: String,
        is_colon: bool,
    },
    Binary {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    Logical {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Not(Box<Expr>),
    Function {
        params: Vec<String>,
        block: Vec<Stmt>,
    },
    AwaitExpr(Box<Expr>),
    ListComp {
        elt: Box<Expr>,
        generators: Vec<CompGenerator>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Stmt {
    Local {
        name: String,
        value: Option<Expr>,
        access: String,
        type_annotation: Option<String>,
        #[serde(skip)]
        span: Span,
    },
    Assign {
        target: Expr,
        value: Expr,
        op: Option<String>,
        #[serde(skip)]
        span: Span,
    },
    Return {
        value: Option<Expr>,
        #[serde(skip)]
        span: Span,
    },
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_if_blocks: Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
        #[serde(skip)]
        span: Span,
    },
    While {
        cond: Expr,
        block: Vec<Stmt>,
        #[serde(skip)]
        span: Span,
    },
    For {
        var: String,
        iter: Expr,
        block: Vec<Stmt>,
        type_annotation: Option<String>,
        #[serde(skip)]
        span: Span,
    },
    FuncDef {
        name: String,
        params: Vec<String>,
        param_types: Vec<Option<String>>,
        param_defaults: Vec<Option<Expr>>,
        block: Vec<Stmt>,
        access: String,
        is_async: bool,
        return_type: Option<String>,
        #[serde(skip)]
        span: Span,
    },
    ClassDef {
        name: String,
        body: Vec<Stmt>,
        access: String,
        #[serde(skip)]
        span: Span,
    },
    ExprStmt {
        expr: Expr,
        #[serde(skip)]
        span: Span,
    },
    EnumDef {
        name: String,
        variants: Vec<String>,
        access: String,
        #[serde(skip)]
        span: Span,
    },
    StructDef {
        name: String,
        fields: Vec<StructField>,
        access: String,
        #[serde(skip)]
        span: Span,
    },
    Import {
        path: String,
        alias: String,
        #[serde(skip)]
        span: Span,
    },
    Break {
        #[serde(skip)]
        span: Span,
    },
    Continue {
        #[serde(skip)]
        span: Span,
    },
    TryCatch {
        try_block: Vec<Stmt>,
        catch_clauses: Vec<(Option<String>, Option<String>, Vec<Stmt>)>,
        finally_block: Option<Vec<Stmt>>,
        #[serde(skip)]
        span: Span,
    },
    DecoratedStmt {
        decorators: Vec<String>,
        stmt: Box<Stmt>,
        #[serde(skip)]
        span: Span,
    },
}
