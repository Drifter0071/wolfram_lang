export interface Span {
    start: number;
    end: number;
}

export interface StructField {
    name: string;
    fieldType: string | null;
    typeAnnotation: string | null;
}

export interface CompGenerator {
    var: string;
    iter: Expr;
    condition: Expr | null;
}

export interface TableFieldPair {
    key: Expr;
    value: Expr;
}

export interface TableFieldValue {
    value: Expr;
}

export type TableField = TableFieldPair | TableFieldValue;

export type Expr =
    | { kind: "Number"; value: number }
    | { kind: "Str"; value: string }
    | { kind: "FString"; value: string }
    | { kind: "Bool"; value: boolean }
    | { kind: "Nil" }
    | { kind: "Ident"; name: string }
    | { kind: "SelfExpr" }
    | { kind: "UnaryMinus"; inner: Expr }
    | { kind: "Grouping"; inner: Expr }
    | { kind: "Array"; elements: Expr[] }
    | { kind: "Table"; fields: TableField[] }
    | { kind: "Index"; obj: Expr; index: Expr }
    | { kind: "Call"; func: string; args: Expr[] }
    | { kind: "MethodCall"; obj: Expr; field: string; isColon: boolean; args: Expr[] }
    | { kind: "Member"; obj: Expr; field: string; isColon: boolean }
    | { kind: "Binary"; left: Expr; op: string; right: Expr }
    | { kind: "Ternary"; cond: Expr; thenExpr: Expr; elseExpr: Expr }
    | { kind: "Logical"; left: Expr; op: string; right: Expr }
    | { kind: "Not"; inner: Expr }
    | { kind: "Function"; params: string[]; block: Stmt[] }
    | { kind: "AwaitExpr"; inner: Expr }
    | { kind: "ListComp"; elt: Expr; generators: CompGenerator[] };

export type Stmt =
    | { kind: "Local"; names: string[]; value: Expr | null; access: string; span: Span; typeAnnotations: (string | null)[] }
    | { kind: "Assign"; target: Expr; value: Expr; op: string | null; span: Span }
    | { kind: "Return"; value: Expr | null; span: Span }
    | { kind: "If"; cond: Expr; thenBlock: Stmt[]; elseIfBlocks: [Expr, Stmt[]][]; elseBlock: Stmt[] | null; span: Span }
    | { kind: "While"; cond: Expr; block: Stmt[]; span: Span }
    | { kind: "For"; vars: string[]; iter: Expr; block: Stmt[]; span: Span; typeAnnotations: (string | null)[] }
    | { kind: "FuncDef"; name: string; params: string[]; paramTypes: (string | null)[]; paramDefaults: (Expr | null)[]; block: Stmt[]; access: string; isAsync: boolean; span: Span; returnType: string | null }
    | { kind: "ClassDef"; name: string; body: Stmt[]; access: string; span: Span; extends: string | null }
    | { kind: "ExprStmt"; expr: Expr; span: Span }
    | { kind: "EnumDef"; name: string; variants: string[]; access: string; span: Span }
    | { kind: "StructDef"; name: string; fields: StructField[]; access: string; span: Span }
    | { kind: "Import"; path: string; alias: string; span: Span }
    | { kind: "Break"; span: Span }
    | { kind: "Continue"; span: Span }
    | { kind: "TryCatch"; tryBlock: Stmt[]; catchClauses: [string | null, string | null, Stmt[]][]; finallyBlock: Stmt[] | null; span: Span }
    | { kind: "DecoratedStmt"; decorators: string[]; stmt: Stmt; span: Span };
