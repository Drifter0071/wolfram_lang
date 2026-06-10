import { Expr, Stmt, Span, StructField, TableField, CompGenerator } from "./ast";

export interface SymbolInfo {
    name: string;
    kind: string;
    access: string;
    location: { line: number; column: number; endLine: number; endColumn: number };
    params: string[];
    fields: string[];
}

export interface ImportInfo {
    path: string;
    alias: string;
}

export interface ParseResult {
    ast: Stmt[];
    symbols: SymbolInfo[];
    imports: ImportInfo[];
    scope: Map<string, string>;
    errors: string[];
}

// ==========================================
// TOKENIZER
// ==========================================
type TokenType =
    | "IF" | "ELSE" | "ELIF" | "LOCAL" | "FUNCTION" | "RETURN" | "WHILE" | "FOR"
    | "IN" | "CLASS" | "PUBLIC" | "PRIVATE" | "SELF" | "TRUE" | "FALSE" | "NIL"
    | "ENUM" | "STRUCT" | "IMPORT" | "AS" | "BREAK" | "CONTINUE" | "AND" | "OR"
    | "NOT" | "TRY" | "CATCH" | "FINALLY" | "ASYNC" | "AWAIT" | "IS"
    | "STARSTAR" | "SLASHSLASH" | "DOTDOT" | "EQEQ" | "NOTEQ" | "LTEQ" | "GTEQ"
    | "PLUSASSIGN" | "MINUSASSIGN" | "STARASSIGN" | "SLASHASSIGN" | "PERCENTASSIGN"
    | "ARROW" | "LBRACE" | "RBRACE" | "LBRACKET" | "RBRACKET" | "LPAREN" | "RPAREN"
    | "COMMA" | "DOT" | "COLON" | "ASSIGN" | "SEMICOLON" | "QUESTION" | "AT"
    | "PLUS" | "MINUS" | "STAR" | "SLASH" | "PERCENT" | "LT" | "GT" | "CARET"
    | "IDENT" | "NUMBER" | "FSTRING" | "STRINGLIT" | "COMMENT";

interface Token {
    type: TokenType;
    value: string;
    span: Span;
}

const KEYWORDS: Record<string, TokenType> = {
    if: "IF", else: "ELSE", elif: "ELIF", local: "LOCAL", function: "FUNCTION",
    return: "RETURN", while: "WHILE", for: "FOR", in: "IN", class: "CLASS",
    public: "PUBLIC", private: "PRIVATE", self: "SELF", true: "TRUE", false: "FALSE",
    nil: "NIL", enum: "ENUM", struct: "STRUCT", import: "IMPORT", as: "AS",
    break: "BREAK", continue: "CONTINUE", and: "AND", or: "OR", not: "NOT",
    try: "TRY", catch: "CATCH", finally: "FINALLY", async: "ASYNC", await: "AWAIT",
    is: "IS",
};

const MULTI_CHAR_OPS: [string, TokenType][] = [
    ["**", "STARSTAR"], ["//", "SLASHSLASH"], ["..", "DOTDOT"],
    ["==", "EQEQ"], ["!=", "NOTEQ"], ["~=", "NOTEQ"],
    ["<=", "LTEQ"], [">=", "GTEQ"],
    ["+=", "PLUSASSIGN"], ["-=", "MINUSASSIGN"], ["*=", "STARASSIGN"],
    ["/=", "SLASHASSIGN"], ["%=", "PERCENTASSIGN"],
    ["->", "ARROW"],
];

function tokenize(source: string): Token[] {
    const tokens: Token[] = [];
    let i = 0;
    const len = source.length;

    while (i < len) {
        const c = source[i];

        // Whitespace
        if (c === " " || c === "\t" || c === "\r" || c === "\f") { i++; continue; }
        if (c === "\n") { i++; continue; }

        // Comments (-- only)
        if (c === "-" && source[i + 1] === "-") {
            const start = i;
            while (i < len && source[i] !== "\n") i++;
            tokens.push({ type: "COMMENT", value: source.substring(start, i), span: { start, end: i } });
            continue;
        }

        // f-string: f"..."
        if (c === "f" && (source[i + 1] === '"' || source[i + 1] === "'")) {
            const quote = source[i + 1];
            const start = i;
            i += 2;
            while (i < len) {
                if (source[i] === "\\") { i += 2; continue; }
                if (source[i] === quote) { i++; break; }
                i++;
            }
            tokens.push({ type: "FSTRING", value: source.substring(start, i), span: { start, end: i } });
            continue;
        }

        // String literal
        if (c === '"' || c === "'") {
            const quote = c;
            const start = i;
            i++;
            while (i < len) {
                if (source[i] === "\\") { i += 2; continue; }
                if (source[i] === quote) { i++; break; }
                i++;
            }
            tokens.push({ type: "STRINGLIT", value: source.substring(start, i), span: { start, end: i } });
            continue;
        }

        // Numbers
        if (/\d/.test(c)) {
            const start = i;
            while (i < len && /[\d.]/.test(source[i])) i++;
            tokens.push({ type: "NUMBER", value: source.substring(start, i), span: { start, end: i } });
            continue;
        }

        // Identifiers and keywords
        if (/[a-zA-Z_]/.test(c)) {
            const start = i;
            while (i < len && /[\w]/.test(source[i])) i++;
            const word = source.substring(start, i);
            const type = KEYWORDS[word] ?? "IDENT";
            tokens.push({ type, value: word, span: { start, end: i } });
            continue;
        }

        // Multi-char operators
        let matched = false;
        for (const [op, type] of MULTI_CHAR_OPS) {
            if (source.startsWith(op, i)) {
                tokens.push({ type, value: op, span: { start: i, end: i + op.length } });
                i += op.length;
                matched = true;
                break;
            }
        }
        if (matched) continue;

        // Single-char operators/punctuation
        const singleMap: Record<string, TokenType> = {
            "{": "LBRACE", "}": "RBRACE", "[": "LBRACKET", "]": "RBRACKET",
            "(": "LPAREN", ")": "RPAREN", ",": "COMMA", ".": "DOT",
            ":": "COLON", "=": "ASSIGN", ";": "SEMICOLON", "?": "QUESTION",
            "@": "AT", "+": "PLUS", "-": "MINUS", "*": "STAR",
            "/": "SLASH", "%": "PERCENT", "<": "LT", ">": "GT",
            "^": "CARET",
        };
        if (singleMap[c]) {
            tokens.push({ type: singleMap[c], value: c, span: { start: i, end: i + 1 } });
            i++;
            continue;
        }

        i++; // skip unknown char
    }

    return tokens;
}

// ==========================================
// PARSER
// ==========================================
function isStmtStart(t: Token): boolean {
    return ["IF", "WHILE", "FOR", "RETURN", "FUNCTION", "CLASS", "ENUM", "STRUCT",
        "IMPORT", "LOCAL", "PUBLIC", "PRIVATE", "BREAK", "CONTINUE", "TRY",
        "ASYNC", "ELIF", "ELSE", "IDENT", "TRUE", "FALSE", "NIL", "SELF", "NOT", "MINUS",
        "LPAREN", "LBRACKET", "LBRACE", "NUMBER", "STRINGLIT", "FSTRING", "COMMENT",
        "AT", "AWAIT"].includes(t.type);
}

class Parser {
    private tokens: Token[];
    private pos: number;
    private source: string;
    private errors: string[];
    private symbols: SymbolInfo[];
    private imports: ImportInfo[];
    private scope: Map<string, string>;

    constructor(tokens: Token[], source: string) {
        this.tokens = tokens;
        this.pos = 0;
        this.source = source;
        this.errors = [];
        this.symbols = [];
        this.imports = [];
        this.scope = new Map();
    }

    parseProgram(): ParseResult {
        const ast: Stmt[] = [];
        while (this.pos < this.tokens.length) {
            try {
                const stmt = this.parseStmt();
                if (stmt) ast.push(stmt);
            } catch (e: any) {
                this.errors.push(e.message ?? String(e));
                this.syncToNextStatement();
            }
        }
        return { ast, symbols: this.symbols, imports: this.imports, scope: this.scope, errors: this.errors };
    }

    private syncToNextStatement(): void {
        while (this.pos < this.tokens.length) {
            const t = this.peek();
            if (!t) break;
            if (t.type === "SEMICOLON" || t.type === "RBRACE" ||
                KEYWORDS[t.value as keyof typeof KEYWORDS] !== undefined) {
                if (t.type !== "ELSE" && t.type !== "ELIF" && t.type !== "CATCH" && t.type !== "FINALLY") {
                    this.pos++;
                    return;
                }
            }
            this.pos++;
        }
    }

    private posString(): string {
        const t = this.peek();
        if (!t) return "EOF";
        const prefix = this.source.substring(0, t.span.start);
        const line = prefix.split("\n").length;
        const lastNL = prefix.lastIndexOf("\n");
        const col = t.span.start - (lastNL === -1 ? 0 : lastNL + 1) + 1;
        return `line ${line}, column ${col}`;
    }

    private peek(): Token | null {
        return this.pos < this.tokens.length ? this.tokens[this.pos] : null;
    }

    private advance(): Token | null {
        const t = this.pos < this.tokens.length ? this.tokens[this.pos] : null;
        this.pos++;
        return t;
    }

    private expect(type: TokenType): Token {
        const t = this.peek();
        if (t && t.type === type) return this.advance()!;
        throw new Error(`${this.posString()}: expected ${type}, found ${t?.type ?? "EOF"}`);
    }

    private parseTypeAnnotation(): string | null {
        const t = this.peek();
        if (!t || t.type !== "IDENT") return null;
        const typeName = this.expect("IDENT").value;

        if (this.peek()?.type === "LBRACKET" && this.peekAhead(1)?.type === "RBRACKET") {
            this.advance(); this.advance();
            return typeName + "[]";
        }
        if (this.peek()?.type === "LBRACE" && this.peekAhead(1)?.type === "LBRACKET") {
            this.advance(); this.advance();
            const keyType = this.expect("IDENT").value;
            this.expect("RBRACKET");
            this.expect("COLON");
            const valType = this.expect("IDENT").value;
            this.expect("RBRACE");
            return `{[${keyType}]: ${valType}}`;
        }
        return typeName;
    }

    private peekAhead(n: number): Token | null {
        return (this.pos + n < this.tokens.length) ? this.tokens[this.pos + n] : null;
    }

    private expectIdentOrSelf(): Token {
        const t = this.peek();
        if (t?.type === "SELF") return this.advance()!;
        return this.expect("IDENT");
    }

    private currentSpan(): Span {
        const t = this.tokens[this.pos - 1];
        return t ? { start: t.span.start, end: t.span.end } : { start: 0, end: 0 };
    }

    private semicolonOrEnd(): void {
        const t = this.peek();
        if (!t || t.type === "SEMICOLON") { if (t) this.advance(); return; }
        if (t.type === "RBRACE" || isStmtStart(t)) return;
        throw new Error(`${this.posString()}: expected semicolon or end of statement, found ${t.type}`);
    }

    private parseStmt(): Stmt {
        const t = this.peek();
        if (!t) throw new Error("Unexpected EOF");

        switch (t.value) {
            case "local": return this.tokens[this.pos + 1]?.value === "function" ? this.parseLocalFunction() : this.parseLocal();
            case "if": return this.parseIf();
            case "while": return this.parseWhile();
            case "for": return this.parseFor();
            case "return": return this.parseReturn();
            case "function": return this.parseFunction(false);
            case "async": return this.parseAsyncFunction();
            case "class": return this.parseClass();
            case "enum": return this.parseEnum();
            case "struct": return this.parseStruct();
            case "import": return this.parseImport();
            case "try": return this.parseTryCatch();
            case "break": { this.advance(); this.semicolonOrEnd(); return { kind: "Break", span: this.currentSpan() }; }
            case "continue": { this.advance(); this.semicolonOrEnd(); return { kind: "Continue", span: this.currentSpan() }; }
        }

        switch (t.type) {
            case "PUBLIC": case "PRIVATE": return this.parseModifierStmt();
            case "AT": return this.parseDecoratedStmt();
        }

        return this.parseExprStmt();
    }

    private parseLocal(): Stmt {
        const startTok = this.expect("LOCAL");
        const nameTok = this.expect("IDENT");
        const names = [nameTok.value];
        while (this.peek()?.type === "COMMA") {
            this.advance();
            const n = this.expect("IDENT");
            names.push(n.value);
        }
        const typeAnnotations: (string | null)[] = names.map(() => null);

        let value: Expr | null = null;

        if (this.peek()?.type === "COLON") {
            this.advance();
            for (let i = 0; i < names.length; i++) {
                const typeName = this.parseTypeAnnotation();
                typeAnnotations[i] = typeName;
                if (i < names.length - 1 && this.peek()?.type === "COMMA") {
                    this.advance();
                }
            }
        }

        if (this.peek()?.type === "ASSIGN") {
            this.advance();
            value = this.parseExpr();
        }
        this.semicolonOrEnd();

        for (let i = 0; i < names.length; i++) {
            const n = names[i];
            this.symbols.push({
                name: n, kind: "variable", access: "private",
                location: { line: 0, column: 0, endLine: 0, endColumn: 0 },
                params: [], fields: [],
            });

            let varType = "any";
            if (typeAnnotations[i]) {
                varType = typeAnnotations[i]!;
            } else if (value) {
                varType = this.inferExprType(value);
            }
            this.scope.set(n, varType);
        }

        return { kind: "Local", names, value, access: "private", span: this.currentSpan(), typeAnnotations };
    }

    private parseLocalFunction(): Stmt {
        this.expect("LOCAL");
        this.expect("FUNCTION");
        const nameTok = this.expect("IDENT");
        const name = nameTok.value;
        this.expect("LPAREN");
        const params: string[] = [];
        const paramTypes: (string | null)[] = [];
        const paramDefaults: (Expr | null)[] = [];
        this.parseParamList(params, paramTypes, paramDefaults);
        this.expect("RPAREN");
        const block = this.parseBlock();
        this.scope.set(name, "function");
        this.symbols.push({
            name, kind: "function", access: "private",
            location: { line: 0, column: 0, endLine: 0, endColumn: 0 },
            params, fields: [],
        });
        for (const p of params) this.scope.set(p, "any");
        return { kind: "FuncDef", name, params, paramTypes, paramDefaults, block, access: "private", isAsync: false, span: this.currentSpan(), returnType: null };
    }

    private parseIf(): Stmt {
        this.expect("IF");
        this.expect("LPAREN");
        const cond = this.parseExpr();
        this.expect("RPAREN");
        const thenBlock = this.parseBlock();
        const elseIfBlocks: [Expr, Stmt[]][] = [];
        let elseBlock: Stmt[] | null = null;

        // Handle `else if (...) { ... }` or `elif (...) { ... }`
        while (true) {
            const t = this.peek();
            if (t?.type === "ELSE") {
                this.advance();
                if (this.peek()?.type === "IF") {
                    this.advance();
                    this.expect("LPAREN");
                    const eiCond = this.parseExpr();
                    this.expect("RPAREN");
                    const eiBlock = this.parseBlock();
                    elseIfBlocks.push([eiCond, eiBlock]);
                } else {
                    elseBlock = this.parseBlock();
                    break;
                }
            } else if (t?.type === "ELIF") {
                this.advance();
                this.expect("LPAREN");
                const eiCond = this.parseExpr();
                this.expect("RPAREN");
                const eiBlock = this.parseBlock();
                elseIfBlocks.push([eiCond, eiBlock]);
            } else {
                break;
            }
        }
        return { kind: "If", cond, thenBlock, elseIfBlocks, elseBlock, span: this.currentSpan() };
    }

    private parseWhile(): Stmt {
        this.expect("WHILE");
        this.expect("LPAREN");
        const cond = this.parseExpr();
        this.expect("RPAREN");
        const block = this.parseBlock();
        return { kind: "While", cond, block, span: this.currentSpan() };
    }

    private parseFor(): Stmt {
        this.expect("FOR");
        const varTok = this.expect("IDENT");
        const vars = [varTok.value];
        const typeAnnotations: (string | null)[] = [];
        if (this.peek()?.type === "COMMA") {
            this.advance();
            const v2 = this.expect("IDENT");
            vars.push(v2.value);
            typeAnnotations.push(null);
        }
        if (this.peek()?.type === "COLON") {
            this.advance();
            typeAnnotations[0] = this.parseTypeAnnotation();
            if (this.peek()?.type === "COMMA") {
                this.advance();
                const t = this.peek();
                if (t?.type === "IDENT") {
                    this.advance();
                    typeAnnotations[1] = "any";
                }
            }
        }
        this.expect("IN");
        const iter = this.parseExpr();
        const block = this.parseBlock();
        for (let i = 0; i < vars.length; i++) {
            this.scope.set(vars[i], typeAnnotations[i] || "any");
        }
        return { kind: "For", vars, iter, block, span: this.currentSpan(), typeAnnotations };
    }

    private parseReturn(): Stmt {
        this.expect("RETURN");
        let value: Expr | null = null;
        const t = this.peek();
        if (t && t.type !== "SEMICOLON" && t.type !== "RBRACE") {
            value = this.parseExpr();
        }
        this.semicolonOrEnd();
        return { kind: "Return", value, span: this.currentSpan() };
    }

    private asyncSpan: Span = { start: 0, end: 0 };

    private parseFunction(isAsync: boolean): Stmt {
        this.expect("FUNCTION");
        return this.parseFunctionBody(isAsync);
    }

    private parseAsyncFunction(): Stmt {
        this.expect("ASYNC");
        this.expect("FUNCTION");
        return this.parseFunctionBody(true);
    }

    private parseFunctionBody(isAsync: boolean): Stmt {
        const nameTok = this.expect("IDENT");
        const name = nameTok.value;
        this.expect("LPAREN");
        const params: string[] = [];
        const paramTypes: (string | null)[] = [];
        const paramDefaults: (Expr | null)[] = [];
        this.parseParamList(params, paramTypes, paramDefaults);
        this.expect("RPAREN");
        let returnType: string | null = null;
        if (this.peek()?.type === "COLON") {
            this.advance();
            returnType = this.parseTypeAnnotation();
        }
        const block = this.parseBlock();
        this.scope.set(name, "function");
        this.symbols.push({
            name, kind: "function", access: "private",
            location: { line: 0, column: 0, endLine: 0, endColumn: 0 },
            params, fields: [],
        });
        for (let i = 0; i < params.length; i++) {
            this.scope.set(params[i], paramTypes[i] || "any");
        }
        return { kind: "FuncDef", name, params, paramTypes, paramDefaults, block, access: "private", isAsync, span: this.currentSpan(), returnType };
    }

    private parseParamList(params: string[], paramTypes: (string | null)[], defaults: (Expr | null)[]): void {
        if (!this.peek() || this.peek()!.type === "RPAREN") return;
        const first = this.expectIdentOrSelf();
        params.push(first.value);
        let ptype: string | null = null;
        let def: Expr | null = null;
        if (this.peek()?.type === "COLON") {
            this.advance();
            ptype = this.parseTypeAnnotation();
        }
        if (this.peek()?.type === "ASSIGN") {
            this.advance();
            def = this.parseExpr();
        }
        paramTypes.push(ptype);
        defaults.push(def);

        while (this.peek()?.type === "COMMA") {
            this.advance();
            const t = this.peek();
            if (t?.type === "RPAREN" || !t) break;
            const p = this.expectIdentOrSelf();
            params.push(p.value);
            let pt: string | null = null;
            let d: Expr | null = null;
            if (this.peek()?.type === "COLON") { this.advance(); pt = this.parseTypeAnnotation(); }
            if (this.peek()?.type === "ASSIGN") { this.advance(); d = this.parseExpr(); }
            paramTypes.push(pt);
            defaults.push(d);
        }
    }

    private parseClass(): Stmt {
        this.expect("CLASS");
        const name = this.expect("IDENT").value;
        const body = this.parseBlock();
        this.scope.set(name, "class");
        this.symbols.push({ name, kind: "class", access: "private", location: { line: 0, column: 0, endLine: 0, endColumn: 0 }, params: [], fields: [] });
        return { kind: "ClassDef", name, body, access: "private", span: this.currentSpan(), extends: null };
    }

    private parseEnum(): Stmt {
        this.expect("ENUM");
        const name = this.expect("IDENT").value;
        this.expect("LBRACE");
        const variants: string[] = [];
        if (this.peek()?.type !== "RBRACE") {
            variants.push(this.expect("IDENT").value);
            while (this.peek()?.type === "COMMA") {
                this.advance();
                if (this.peek()?.type === "RBRACE") break;
                variants.push(this.expect("IDENT").value);
            }
        }
        this.expect("RBRACE");
        this.scope.set(name, "enum");
        this.symbols.push({ name, kind: "enum", access: "private", location: { line: 0, column: 0, endLine: 0, endColumn: 0 }, params: [], fields: variants });
        return { kind: "EnumDef", name, variants, access: "private", span: this.currentSpan() };
    }

    private parseStruct(): Stmt {
        this.expect("STRUCT");
        const name = this.expect("IDENT").value;
        this.expect("LBRACE");
        const fields: StructField[] = [];
        if (this.peek()?.type !== "RBRACE") {
            const fname = this.expect("IDENT").value;
            let ftype: string | null = null;
            if (this.peek()?.type === "COLON") { this.advance(); ftype = this.expect("IDENT").value; }
            fields.push({ name: fname, fieldType: ftype, typeAnnotation: null });
            while (this.peek()?.type === "COMMA") {
                this.advance();
                if (this.peek()?.type === "RBRACE") break;
                const fn = this.expect("IDENT").value;
                let ft: string | null = null;
                if (this.peek()?.type === "COLON") { this.advance(); ft = this.expect("IDENT").value; }
                fields.push({ name: fn, fieldType: ft, typeAnnotation: null });
            }
        }
        this.expect("RBRACE");
        this.scope.set(name, "struct");
        this.symbols.push({ name, kind: "struct", access: "private", location: { line: 0, column: 0, endLine: 0, endColumn: 0 }, params: [], fields: fields.map(f => f.name) });
        return { kind: "StructDef", name, fields, access: "private", span: this.currentSpan() };
    }

    private parseImport(): Stmt {
        this.expect("IMPORT");
        const pathTok = this.peek();
        if (!pathTok || pathTok.type !== "STRINGLIT") throw new Error(`${this.posString()}: expected string literal for import path`);
        this.advance();
        const raw = pathTok.value;
        const path = raw.startsWith('"') || raw.startsWith("'") ? raw.slice(1, -1) : raw;
        this.expect("AS");
        const alias = this.expect("IDENT").value;
        this.semicolonOrEnd();
        this.imports.push({ path, alias });
        this.scope.set(alias, "module");
        return { kind: "Import", path, alias, span: this.currentSpan() };
    }

    private parseTryCatch(): Stmt {
        this.expect("TRY");
        const tryBlock = this.parseBlock();
        const catchClauses: [string | null, string | null, Stmt[]][] = [];
        while (this.peek()?.type === "CATCH") {
            this.advance();
            if (this.peek()?.type === "LBRACE") {
                catchClauses.push([null, null, this.parseBlock()]);
            } else {
                const first = this.expect("IDENT").value;
                if (this.peek()?.type === "AS") {
                    this.advance();
                    const varName = this.expect("IDENT").value;
                    catchClauses.push([first, varName, this.parseBlock()]);
                } else {
                    catchClauses.push([null, first, this.parseBlock()]);
                }
            }
        }
        let finallyBlock: Stmt[] | null = null;
        if (this.peek()?.type === "FINALLY") { this.advance(); finallyBlock = this.parseBlock(); }
        return { kind: "TryCatch", tryBlock, catchClauses, finallyBlock, span: this.currentSpan() };
    }

    private parseBlock(): Stmt[] {
        this.expect("LBRACE");
        const stmts: Stmt[] = [];
        while (this.peek() && this.peek()!.type !== "RBRACE") {
            try { stmts.push(this.parseStmt()); }
            catch (e: any) {
                this.errors.push(e.message ?? String(e));
                this.syncToNextStatement();
            }
        }
        this.expect("RBRACE");
        return stmts;
    }

    private parseModifierStmt(): Stmt {
        const access = this.advance()!.value; // "public" or "private"
        const t = this.peek();
        if (!t) throw new Error("Unexpected EOF after modifier");

        switch (t.value) {
            case "local": {
                const stmt = this.parseLocal();
                (stmt as any).access = access;
                return stmt;
            }
            case "function": {
                const stmt = this.parseFunction(false);
                (stmt as any).access = access;
                return stmt;
            }
            case "class": {
                const stmt = this.parseClass();
                (stmt as any).access = access;
                return stmt;
            }
            case "enum": {
                const stmt = this.parseEnum();
                (stmt as any).access = access;
                return stmt;
            }
            case "struct": {
                const stmt = this.parseStruct();
                (stmt as any).access = access;
                return stmt;
            }
        }
        throw new Error(`${this.posString()}: expected local/function/class/enum/struct after access modifier`);
    }

    private parseDecoratedStmt(): Stmt {
        const decorators: string[] = [];
        while (this.peek()?.type === "AT") {
            this.advance();
            decorators.push(this.expect("IDENT").value);
        }
        const stmt = this.parseStmt();
        return { kind: "DecoratedStmt", decorators, stmt, span: this.currentSpan() };
    }

    private parseExprStmt(): Stmt {
        const expr = this.parseExpr();
        if (this.peek()?.type === "ASSIGN") {
            this.advance();
            const value = this.parseExpr();
            this.semicolonOrEnd();
            return { kind: "Assign", target: expr, value, op: null, span: this.currentSpan() };
        }
        if (["PLUSASSIGN", "MINUSASSIGN", "STARASSIGN", "SLASHASSIGN", "PERCENTASSIGN"].includes(this.peek()?.type ?? "")) {
            const opTok = this.advance()!;
            const op = opTok.value[0];
            const value = this.parseExpr();
            this.semicolonOrEnd();
            return { kind: "Assign", target: expr, value, op, span: this.currentSpan() };
        }
        this.semicolonOrEnd();
        return { kind: "ExprStmt", expr, span: this.currentSpan() };
    }

    // ==========================================
    // EXPRESSION PARSING (Pratt-style)
    // ==========================================
    private parseExpr(): Expr {
        let expr = this.parseOr();
        if (this.peek()?.type === "QUESTION") {
            this.advance();
            const thenExpr = this.parseExpr();
            this.expect("COLON");
            const elseExpr = this.parseExpr();
            expr = { kind: "Ternary", cond: expr, thenExpr, elseExpr };
        }
        return expr;
    }

    private parseOr(): Expr {
        let expr = this.parseAnd();
        while (this.peek()?.type === "OR") {
            this.advance();
            expr = { kind: "Logical", left: expr, op: "or", right: this.parseAnd() };
        }
        return expr;
    }

    private parseAnd(): Expr {
        let expr = this.parseComparison();
        while (this.peek()?.type === "AND") {
            this.advance();
            expr = { kind: "Logical", left: expr, op: "and", right: this.parseComparison() };
        }
        return expr;
    }

    private parseComparison(): Expr {
        let expr = this.parseAddition();
        while (["EQEQ", "NOTEQ", "LT", "GT", "LTEQ", "GTEQ"].includes(this.peek()?.type ?? "")) {
            const op = this.advance()!.value;
            expr = { kind: "Binary", left: expr, op, right: this.parseAddition() };
        }
        return expr;
    }

    private parseAddition(): Expr {
        let expr = this.parseMultiplication();
        while (["PLUS", "MINUS", "DOTDOT"].includes(this.peek()?.type ?? "")) {
            const op = this.advance()!.value;
            expr = { kind: "Binary", left: expr, op, right: this.parseMultiplication() };
        }
        return expr;
    }

    private parseMultiplication(): Expr {
        let expr = this.parseUnary();
        while (["STARSTAR", "CARET", "SLASHSLASH", "STAR", "SLASH", "PERCENT"].includes(this.peek()?.type ?? "")) {
            let op = this.advance()!.value;
            if (op === "**") op = "^";
            expr = { kind: "Binary", left: expr, op, right: this.parseUnary() };
        }
        return expr;
    }

    private parseUnary(): Expr {
        if (this.peek()?.type === "MINUS") {
            this.advance();
            return { kind: "UnaryMinus", inner: this.parseUnary() };
        }
        if (this.peek()?.type === "NOT") {
            this.advance();
            return { kind: "Not", inner: this.parseUnary() };
        }
        return this.parsePostfix();
    }

    private parsePostfix(): Expr {
        let expr = this.parsePrimary();
        while (true) {
            const t = this.peek();
            if (t?.type === "LPAREN") {
                this.advance();
                const args: Expr[] = [];
                if (this.peek()?.type !== "RPAREN") {
                    args.push(this.parseExpr());
                    while (this.peek()?.type === "COMMA") { this.advance(); args.push(this.parseExpr()); }
                }
                this.expect("RPAREN");
                if (expr.kind === "Ident") {
                    expr = { kind: "Call", func: expr.name, args };
                } else if (expr.kind === "Member") {
                    expr = { kind: "MethodCall", obj: expr.obj, field: expr.field, isColon: expr.isColon, args };
                } else {
                    throw new Error(`${this.posString()}: complex call not supported`);
                }
            } else if (t?.type === "DOT" || t?.type === "COLON") {
                const isColon = t.type === "COLON";
                this.advance();
                const field = this.expect("IDENT").value;
                expr = { kind: "Member", obj: expr, field, isColon };
            } else if (t?.type === "LBRACKET") {
                this.advance();
                const index = this.parseExpr();
                this.expect("RBRACKET");
                expr = { kind: "Index", obj: expr, index };
            } else {
                break;
            }
        }
        return expr;
    }

    private parsePrimary(): Expr {
        const t = this.advance();
        if (!t) throw new Error("Unexpected EOF");

        switch (t.type) {
            case "AWAIT": return { kind: "AwaitExpr", inner: this.parseExpr() };
            case "NUMBER": return { kind: "Number", value: parseFloat(t.value) };
            case "STRINGLIT": return { kind: "Str", value: t.value };
            case "FSTRING": return { kind: "FString", value: t.value };
            case "TRUE": return { kind: "Bool", value: true };
            case "FALSE": return { kind: "Bool", value: false };
            case "NIL": return { kind: "Nil" };
            case "SELF": return { kind: "SelfExpr" };
            case "IDENT": return { kind: "Ident", name: t.value };

            case "LPAREN": {
                // Try arrow function: (params) -> expr
                const savePos = this.pos;
                const isArrow = this.peek()?.type === "IDENT" || this.peek()?.type === "RPAREN";
                if (isArrow) {
                    const params: string[] = [];
                    const paramTypes: (string | null)[] = [];
                    const defaults: (Expr | null)[] = [];
                    try {
                        this.parseParamList(params, paramTypes, defaults);
                        if (this.peek()?.type === "RPAREN") {
                            this.advance();
                            if (this.peek()?.type === "ARROW") {
                                this.advance();
                                let body: Stmt[];
                                if (this.peek()?.type === "LBRACE") {
                                    body = this.parseBlock();
                                } else {
                                    const e = this.parseExpr();
                                    body = [{ kind: "Return", value: e, span: this.currentSpan() }];
                                }
                                for (const p of params) this.scope.set(p, "any");
                                return { kind: "Function", params, block: body };
                            }
                        }
                    } catch {}
                    this.pos = savePos;
                    // Continue as grouping
                }
                const expr = this.parseExpr();
                this.expect("RPAREN");
                return { kind: "Grouping", inner: expr };
            }

            case "LBRACKET": {
                const elements: Expr[] = [];
                if (this.peek()?.type !== "RBRACKET") {
                    elements.push(this.parseExpr());
                    // List comprehension
                    if (this.peek()?.value === "for") {
                        this.advance(); // for
                        const varTok = this.expect("IDENT");
                        this.scope.set(varTok.value, "any");
                        this.expect("IN"); // "in" keyword
                        const iter = this.parseExpr();
                        const generators: CompGenerator[] = [];
                        let condition: Expr | null = null;
                        if (this.peek()?.value === "if") { this.advance(); condition = this.parseExpr(); }
                        generators.push({ var: varTok.value, iter, condition });
                        this.expect("RBRACKET");
                        return { kind: "ListComp", elt: elements[0], generators };
                    }
                    while (this.peek()?.type === "COMMA") { this.advance(); if (this.peek()?.type === "RBRACKET") break; elements.push(this.parseExpr()); }
                }
                this.expect("RBRACKET");
                return { kind: "Array", elements };
            }

            case "LBRACE": {
                const fields: TableField[] = [];
                if (this.peek()?.type !== "RBRACE") {
                    fields.push(this.parseTableField());
                    while (this.peek()?.type === "COMMA") { this.advance(); if (this.peek()?.type === "RBRACE") break; fields.push(this.parseTableField()); }
                }
                this.expect("RBRACE");
                return { kind: "Table", fields };
            }

            case "FUNCTION": {
                this.expect("LPAREN");
                const params: string[] = [];
                const paramTypes: (string | null)[] = [];
                const defaults: (Expr | null)[] = [];
                this.parseParamList(params, paramTypes, defaults);
                this.expect("RPAREN");
                const block = this.parseBlock();
                for (const p of params) this.scope.set(p, "any");
                return { kind: "Function", params, block };
            }

            default:
                throw new Error(`${this.posString()}: expected expression, found ${t.value}`);
        }
    }

    private parseTableField(): TableField {
        const t1 = this.peek();
        const t2 = this.pos + 1 < this.tokens.length ? this.tokens[this.pos + 1] : null;
        const isPair = (t1?.type === "IDENT" || t1?.type === "STRINGLIT") &&
            (t2?.type === "ASSIGN" || t2?.type === "COLON");

        if (isPair) {
            const key = this.parsePrimary();
            this.advance(); // skip = or :
            const value = this.parseExpr();
            return { key, value };
        } else {
            const value = this.parseExpr();
            return { value };
        }
    }

    private inferExprType(expr: Expr): string {
        return inferExprType(expr);
    }
}

// ==========================================
// PUBLIC API
// ==========================================
export function parseSource(source: string): ParseResult {
    const tokensWithComments = tokenize(source);
    const tokens = tokensWithComments.filter(t => t.type !== "COMMENT");
    const parser = new Parser(tokens, source);
    const result = parser.parseProgram();

    // Fill in line/column info for symbols after parse
    fillSymbolLocations(result.symbols, result.ast, source);

    return result;
}

function fillSymbolLocations(symbols: SymbolInfo[], ast: Stmt[], source: string): void {
    for (const sym of symbols) {
        const loc = findSymbolInSource(source, sym.name);
        if (loc) {
            sym.location = loc;
        }
    }
}

function findSymbolInSource(source: string, name: string): { line: number; column: number; endLine: number; endColumn: number } | null {
    // Search for "function name" or "class name" or "local name" or "enum name" or "struct name"
    const patterns: RegExp[] = [
        new RegExp(`\\bfunction\\s+${escapeRegex(name)}\\b`),
        new RegExp(`\\bclass\\s+${escapeRegex(name)}\\b`),
        new RegExp(`\\benum\\s+${escapeRegex(name)}\\b`),
        new RegExp(`\\bstruct\\s+${escapeRegex(name)}\\b`),
        new RegExp(`\\blocal\\s+${escapeRegex(name)}\\b`),
    ];

    for (const re of patterns) {
        const m = source.match(re);
        if (m && m.index !== undefined) {
            const prefix = source.substring(0, m.index);
            const line = prefix.split("\n").length;
            const lastNL = prefix.lastIndexOf("\n");
            const col = (m.index) - (lastNL === -1 ? 0 : lastNL + 1) + 1;
            // Search forward for keyword position
            const kwMatch = source.substring(m.index).match(/^\w+/);
            const kwLen = kwMatch ? kwMatch[0].length : 0;
            const nameStart = m.index + kwLen + 1; // +1 for space
            const namePrefix = source.substring(0, nameStart);
            const nameLine = namePrefix.split("\n").length;
            const nameLastNL = namePrefix.lastIndexOf("\n");
            const nameCol = nameStart - (nameLastNL === -1 ? 0 : nameLastNL + 1) + 1;
            return {
                line: nameLine,
                column: nameCol,
                endLine: nameLine,
                endColumn: nameCol + name.length,
            };
        }
    }
    return null;
}

function escapeRegex(s: string): string {
    return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function inferExprType(expr: Expr): string {
    switch (expr.kind) {
        case "Number": return "number";
        case "Str":
        case "FString": return "string";
        case "Bool": return "boolean";
        case "Nil": return "nil";
        case "Array": return "array";
        case "Table": return "table";
        case "Call": {
            const known = ["Vector3", "Vector2", "CFrame", "Color3", "UDim2", "UDim", "BrickColor", "TweenInfo", "Ray", "Region3", "DateTime", "Instance", "OverlapParams", "RaycastParams", "RaycastResult", "NumberRange", "NumberSequence", "ColorSequence", "PhysicalProperties", "Faces", "Axes", "Rect", "Random", "PathWaypoint", "DockWidgetPluginGuiInfo"];
            return known.includes(expr.func) ? expr.func : "any";
        }
        case "MethodCall": return expr.field === "GetService" ? "Instance" : "any";
        case "ListComp": return "array";
        case "Function": return "function";
        default: return "any";
    }
}
