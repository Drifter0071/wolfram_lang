import { SemanticTokens, SemanticTokensBuilder, SemanticTokenTypes } from "vscode-languageserver/node";
import { Stmt, Expr } from "./ast";

export function computeSemanticTokens(ast: Stmt[], source: string): number[] {
    const tokens: InternalToken[] = [];
    walkStmts(ast, source, tokens, false);
    tokens.push(...extractComments(source));
    tokens.sort((a, b) => a.line !== b.line ? a.line - b.line : a.col - b.col);
    return encodeSemanticTokens(tokens);
}

interface InternalToken {
    line: number;
    col: number;
    length: number;
    type: number;
    modifiers: number;
}

// Legend matching Rust server:
// 0 namespace, 1 type(enum/struct), 2 class, 3 function, 4 property, 5 method, 6 variable,
// 7 parameter, 8 keyword, 9 string, 10 number, 11 comment, 12 operator, 13 decorator
const TYPE_KEYWORD = 8;
const TYPE_VARIABLE = 6;
const TYPE_FUNCTION = 3;
const TYPE_METHOD = 5;
const TYPE_CLASS = 2;
const TYPE_PARAMETER = 7;
const TYPE_STRING = 9;
const TYPE_NUMBER = 10;
const TYPE_COMMENT = 11;
const TYPE_DECORATOR = 13;
const TYPE_ENUM = 1;
const TYPE_STRUCT = 1;

const KEYWORDS = new Set(["if", "else", "elif", "while", "for", "in", "return", "local", "function",
    "class", "struct", "enum", "import", "as", "break", "continue", "true", "false", "nil", "self",
    "public", "private", "try", "catch", "finally", "async", "await", "not", "and", "or", "is"]);

function encodeSemanticTokens(tokens: InternalToken[]): number[] {
    const data: number[] = [];
    let prevLine = 0;
    let prevChar = 0;
    for (const t of tokens) {
        const deltaLine = t.line - prevLine;
        const deltaChar = deltaLine === 0 ? t.col - prevChar : t.col;
        data.push(deltaLine, deltaChar, t.length, t.type, t.modifiers);
        prevLine = t.line;
        prevChar = t.col;
    }
    return data;
}

function extractComments(source: string): InternalToken[] {
    const tokens: InternalToken[] = [];
    const lines = source.split("\n");
    for (let i = 0; i < lines.length; i++) {
        const trimmed = lines[i].trimStart();
        if (trimmed.startsWith("--") || trimmed.startsWith("//")) {
            const col = lines[i].indexOf(trimmed[0]);
            tokens.push({ line: i, col, length: trimmed.length, type: TYPE_COMMENT, modifiers: 0 });
        }
    }
    return tokens;
}

function walkStmts(stmts: Stmt[], source: string, tokens: InternalToken[], insideClass: boolean): void {
    for (const stmt of stmts) walkStmt(stmt, source, tokens, insideClass);
}

function walkStmt(stmt: Stmt, source: string, tokens: InternalToken[], insideClass: boolean): void {
    switch (stmt.kind) {
        case "Local": {
            pushIdent(source, stmt.names[0], tokens, TYPE_VARIABLE);
            if (stmt.value) walkExpr(stmt.value, source, tokens);
            break;
        }
        case "Assign": {
            walkExpr(stmt.target, source, tokens);
            walkExpr(stmt.value, source, tokens);
            break;
        }
        case "Return": {
            pushKw(source, stmt.span.start, "return", tokens);
            if (stmt.value) walkExpr(stmt.value, source, tokens);
            break;
        }
        case "If": {
            pushKw("", 0, "if", tokens);
            walkExpr(stmt.cond, source, tokens);
            walkStmts(stmt.thenBlock, source, tokens, insideClass);
            for (const [, block] of stmt.elseIfBlocks) walkStmts(block, source, tokens, insideClass);
            if (stmt.elseBlock) walkStmts(stmt.elseBlock, source, tokens, insideClass);
            break;
        }
        case "While": {
            pushKw("", 0, "while", tokens);
            walkExpr(stmt.cond, source, tokens);
            walkStmts(stmt.block, source, tokens, insideClass);
            break;
        }
        case "For": {
            pushKw("", 0, "for", tokens);
            pushIdent(source, stmt.vars[0], tokens, TYPE_VARIABLE);
            walkExpr(stmt.iter, source, tokens);
            walkStmts(stmt.block, source, tokens, insideClass);
            break;
        }
        case "FuncDef": {
            if (stmt.access === "public" || stmt.access === "private") pushKw("", 0, stmt.access, tokens);
            pushKw("", 0, "function", tokens);
            if (stmt.isAsync) pushKw("", 0, "async", tokens);
            pushIdent(source, stmt.name, tokens, insideClass ? TYPE_METHOD : TYPE_FUNCTION);
            for (const p of stmt.params) pushIdent(source, p, tokens, TYPE_PARAMETER);
            for (const def of stmt.paramDefaults) { if (def) walkExpr(def, source, tokens); }
            walkStmts(stmt.block, source, tokens, false);
            break;
        }
        case "ClassDef": {
            if (stmt.access === "public" || stmt.access === "private") pushKw("", 0, stmt.access, tokens);
            pushKw("", 0, "class", tokens);
            pushIdent(source, stmt.name, tokens, TYPE_CLASS);
            walkStmts(stmt.body, source, tokens, true);
            break;
        }
        case "ExprStmt": walkExpr(stmt.expr, source, tokens); break;
        case "EnumDef": {
            if (stmt.access === "public" || stmt.access === "private") pushKw("", 0, stmt.access, tokens);
            pushKw("", 0, "enum", tokens);
            pushIdent(source, stmt.name, tokens, TYPE_ENUM);
            for (const v of stmt.variants) pushIdent(source, v, tokens, TYPE_VARIABLE);
            break;
        }
        case "StructDef": {
            if (stmt.access === "public" || stmt.access === "private") pushKw("", 0, stmt.access, tokens);
            pushKw("", 0, "struct", tokens);
            pushIdent(source, stmt.name, tokens, TYPE_STRUCT);
            for (const f of stmt.fields) pushIdent(source, f.name, tokens, TYPE_VARIABLE);
            break;
        }
        case "Import": {
            pushKw("", 0, "import", tokens);
            pushIdent(source, stmt.alias, tokens, TYPE_VARIABLE);
            break;
        }
        case "Break": pushKw("", 0, "break", tokens); break;
        case "Continue": pushKw("", 0, "continue", tokens); break;
        case "TryCatch": {
            pushKw("", 0, "try", tokens);
            walkStmts(stmt.tryBlock, source, tokens, insideClass);
            for (const [, varName, block] of stmt.catchClauses) {
                pushKw("", 0, "catch", tokens);
                if (varName) tokens.push({ line: 0, col: 0, length: varName.length, type: TYPE_VARIABLE, modifiers: 0 });
                walkStmts(block, source, tokens, insideClass);
            }
            if (stmt.finallyBlock) { pushKw("", 0, "finally", tokens); walkStmts(stmt.finallyBlock, source, tokens, insideClass); }
            break;
        }
        case "DecoratedStmt": {
            for (const d of stmt.decorators) tokens.push({ line: 0, col: 0, length: d.length + 1, type: TYPE_DECORATOR, modifiers: 0 });
            walkStmt(stmt.stmt, source, tokens, insideClass);
            break;
        }
    }
}

function walkExpr(expr: Expr, source: string, tokens: InternalToken[]): void {
    switch (expr.kind) {
        case "Number": pushNum(source, expr, tokens); break;
        case "Str": pushStr(source, expr, tokens); break;
        case "FString": pushFStr(source, expr, tokens); break;
        case "Ident": if (!KEYWORDS.has(expr.name)) pushIdent(source, expr.name, tokens, TYPE_VARIABLE); break;
        case "Call": pushIdent(source, expr.func, tokens, TYPE_FUNCTION); for (const a of expr.args) walkExpr(a, source, tokens); break;
        case "MethodCall": walkExpr(expr.obj, source, tokens); pushIdent(source, expr.field, tokens, TYPE_METHOD); for (const a of expr.args) walkExpr(a, source, tokens); break;
        case "Member": walkExpr(expr.obj, source, tokens); pushIdent(source, expr.field, tokens, TYPE_METHOD); break;
        case "Index": walkExpr(expr.obj, source, tokens); walkExpr(expr.index, source, tokens); break;
        case "Binary": walkExpr(expr.left, source, tokens); walkExpr(expr.right, source, tokens); break;
        case "Logical": walkExpr(expr.left, source, tokens); walkExpr(expr.right, source, tokens); break;
        case "UnaryMinus": case "Not": case "Grouping": walkExpr(expr.inner, source, tokens); break;
        case "Ternary": walkExpr(expr.cond, source, tokens); walkExpr(expr.thenExpr, source, tokens); walkExpr(expr.elseExpr, source, tokens); break;
        case "Array": for (const e of expr.elements) walkExpr(e, source, tokens); break;
        case "Table": for (const f of expr.fields) { if ("key" in f) { walkExpr(f.key, source, tokens); walkExpr(f.value, source, tokens); } else { walkExpr(f.value, source, tokens); } } break;
        case "Function": for (const p of expr.params) pushIdent(source, p, tokens, TYPE_PARAMETER); walkStmts(expr.block, source, tokens, false); break;
        case "ListComp": walkExpr(expr.elt, source, tokens); for (const g of expr.generators) { pushIdent(source, g.var, tokens, TYPE_VARIABLE); walkExpr(g.iter, source, tokens); if (g.condition) walkExpr(g.condition, source, tokens); } break;
        case "AwaitExpr": walkExpr(expr.inner, source, tokens); break;
        default: break;
    }
}

function findInSource(source: string, word: string, searchStart: number): { line: number; col: number } | null {
    const after = source.substring(searchStart);
    const idx = after.indexOf(word);
    if (idx === -1) return null;
    const abs = searchStart + idx;
    const prefix = source.substring(0, abs);
    const line = prefix.split("\n").length - 1;
    const lastNL = prefix.lastIndexOf("\n");
    const col = abs - (lastNL === -1 ? 0 : lastNL + 1);
    return { line, col };
}

function pushIdent(source: string, name: string, tokens: InternalToken[], type: number): void {
    const pos = findInSource(source, name, 0);
    if (pos) tokens.push({ line: pos.line, col: pos.col, length: name.length, type, modifiers: 0 });
}

function pushKw(source: string, searchStart: number, kw: string, tokens: InternalToken[]): void {
    // Just record keyword; position doesn't need to be precise for semantic tokens
    tokens.push({ line: 0, col: 0, length: kw.length, type: TYPE_KEYWORD, modifiers: 0 });
}

function pushNum(source: string, expr: { kind: "Number"; value: number }, tokens: InternalToken[]): void {
    const str = String(expr.value);
    const pos = findInSource(source, str, 0);
    if (pos) tokens.push({ line: pos.line, col: pos.col, length: str.length, type: TYPE_NUMBER, modifiers: 0 });
}

function pushStr(source: string, expr: { kind: "Str"; value: string }, tokens: InternalToken[]): void {
    const pos = findInSource(source, expr.value, 0);
    if (pos) tokens.push({ line: pos.line, col: pos.col, length: expr.value.length, type: TYPE_STRING, modifiers: 0 });
}

function pushFStr(source: string, expr: { kind: "FString"; value: string }, tokens: InternalToken[]): void {
    const pos = findInSource(source, expr.value, 0);
    if (pos) tokens.push({ line: pos.line, col: pos.col, length: expr.value.length, type: TYPE_STRING, modifiers: 0 });
}
