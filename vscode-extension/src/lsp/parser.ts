import { SymbolInfo, ImportInfo } from "./bindings";

type Token = { type: string; value: string; line: number; col: number };

function tokenize(source: string): Token[] {
    const tokens: Token[] = [];
    let i = 0;
    let line = 1;
    let col = 1;
    const len = source.length;

    while (i < len) {
        const c = source[i];

        // Whitespace
        if (c === " " || c === "\t" || c === "\r") { col++; i++; continue; }
        if (c === "\n") { line++; col = 1; i++; continue; }

        // Comments
        if (c === "/" && source[i + 1] === "/") {
            while (i < len && source[i] !== "\n") i++;
            continue;
        }
        if (c === "-" && source[i + 1] === "-") {
            while (i < len && source[i] !== "\n") i++;
            continue;
        }

        // Strings
        if (c === '"' || c === "'") {
            const quote = c;
            let val = quote;
            i++; col++;
            while (i < len) {
                if (source[i] === "\\") { val += source[i] + source[i + 1]; i += 2; col += 2; continue; }
                if (source[i] === quote) { val += quote; i++; col++; break; }
                if (source[i] === "\n") line++;
                val += source[i];
                i++; col++;
            }
            tokens.push({ type: "STRING", value: val, line, col: col - val.length });
            continue;
        }

        // Backtick strings (f-strings)
        if (c === "`") {
            let val = "`";
            i++; col++;
            while (i < len) {
                if (source[i] === "\\") { val += source[i] + source[i + 1]; i += 2; col += 2; continue; }
                if (source[i] === "`") { val += "`"; i++; col++; break; }
                if (source[i] === "\n") line++;
                val += source[i];
                i++; col++;
            }
            tokens.push({ type: "FSTRING", value: val, line, col: col - val.length });
            continue;
        }

        // Numbers
        if (/[\d]/.test(c)) {
            let val = "";
            const startCol = col;
            while (i < len && /[\d.]/.test(source[i])) { if (source[i] === "\n") line++; val += source[i]; i++; col++; }
            tokens.push({ type: "NUMBER", value: val, line, col: startCol });
            continue;
        }

        // Identifiers / keywords
        if (/[a-zA-Z_]/.test(c)) {
            let val = "";
            const startCol = col;
            while (i < len && /[\w]/.test(source[i])) { val += source[i]; i++; col++; }
            const keywords = new Set(["if", "else", "elif", "while", "for", "function", "local", "return",
                "break", "continue", "class", "struct", "enum", "import", "as", "in", "true", "false",
                "nil", "self", "public", "private", "try", "catch", "finally"]);
            tokens.push({ type: keywords.has(val) ? "KEYWORD" : "IDENT", value: val, line, col: startCol });
            continue;
        }

        // Operators and punctuation
        const multiOps: Record<string, string> = {
            "==": "OP", "!=": "OP", "<=": "OP", ">=": "OP", "&&": "OP", "||": "OP",
            "+=": "OP", "-=": "OP", "*=": "OP", "/=": "OP",
        };
        const two = source.substring(i, i + 2);
        if (multiOps[two]) { tokens.push({ type: multiOps[two], value: two, line, col }); i += 2; col += 2; continue; }

        const singleOps = "+-*/<>=!.,:;(){}[]{}";
        if (singleOps.includes(c)) {
            tokens.push({ type: c === "." || c === ":" ? "DOT_COLON" : "OP", value: c, line, col });
            i++; col++;
            continue;
        }

        i++; col++; // skip unknown
    }

    return tokens;
}

export function parseDocument(source: string): { symbols: SymbolInfo[]; imports: ImportInfo[]; scope: Map<string, string> } {
    const tokens = tokenize(source);
    const symbols: SymbolInfo[] = [];
    const imports: ImportInfo[] = [];
    const scope = new Map<string, string>();

    let idx = 0;

    function peek(): Token | null { return idx < tokens.length ? tokens[idx] : null; }
    function advance(): Token | null { return idx < tokens.length ? tokens[idx++] : null; }
    function skipSemicolons(): void { while (peek() && peek()!.value === ";") advance(); }

    function parseStatement(): void {
        skipSemicolons();
        const t = peek();
        if (!t) return;

        if (t.type === "KEYWORD" && t.value === "import") {
            advance(); // 'import'
            const pathTok = advance(); // string literal
            if (advance() && peek() && peek()!.value === "as") advance();
            const aliasTok = peek() && peek()!.type === "IDENT" ? advance() : null;
            if (pathTok && pathTok.type === "STRING") {
                const p = pathTok.value.slice(1, -1); // remove quotes
                const alias = aliasTok?.value ?? p.split("/").pop()?.replace(/\.\w+$/, "") ?? p;
                imports.push({ path: p, alias });
                scope.set(alias, "module");
            }
            return;
        }

        if (t.type === "KEYWORD" && t.value === "local") {
            advance(); // 'local'
            const nameTok = peek();
            if (nameTok?.type === "IDENT") {
                const name = advance()!.value;
                const startLine = nameTok.line;
                const startCol = nameTok.col;
                let eqFound = false;
                while (peek() && peek()!.value !== "=" && peek()!.value !== "\n") advance();
                if (peek() && peek()!.value === "=") { advance(); eqFound = true; }
                // Try to resolve type from RHS
                let varType = "any";
                if (eqFound) {
                    const rhs = resolveExprType(tokens, idx, scope);
                    if (rhs) varType = rhs;
                }
                scope.set(name, varType);
                symbols.push({
                    name,
                    kind: "variable",
                    access: "public",
                    location: { line: startLine, column: startCol, endLine: startLine, endColumn: startCol + name.length },
                    params: [],
                    fields: [],
                });
            }
            return;
        }

        const modifiers = new Set(["public", "private"]);
        let access = "public";
        if (t.type === "KEYWORD" && modifiers.has(t.value)) {
            access = advance()!.value;
        }

        const kw = peek();
        if (!kw || kw.type !== "KEYWORD") return;

        const kwVal = kw.value;
        const startLine = kw.line;
        const startCol = kw.col;
        advance();

        if (kwVal === "function") {
            const nameTok = peek();
            if (nameTok?.type === "IDENT") {
                const name = advance()!.value;
                const params: string[] = [];
                if (peek()?.value === "(") {
                    advance();
                    while (peek() && peek()!.value !== ")") {
                        if (peek()!.type === "IDENT") {
                            params.push(advance()!.value);
                            scope.set(params[params.length - 1], "any");
                        } else {
                            advance();
                        }
                        advance(); // skip colon type annotation if present
                        if (peek()?.value === ",") advance();
                    }
                    advance(); // )
                }
                scope.set(name, "function");
                symbols.push({
                    name,
                    kind: "function",
                    access,
                    location: { line: startLine, column: startCol, endLine: startLine, endColumn: startCol + kwVal.length + name.length + 1 },
                    params,
                    fields: [],
                });
            }
            return;
        }

        if (kwVal === "class" || kwVal === "struct" || kwVal === "enum") {
            const nameTok = peek();
            if (nameTok?.type === "IDENT") {
                const name = advance()!.value;
                scope.set(name, kwVal);
                const kindStr = kwVal;
                symbols.push({
                    name,
                    kind: kindStr,
                    access,
                    location: { line: startLine, column: startCol, endLine: startLine, endColumn: startCol + kwVal.length + name.length + 1 },
                    params: [],
                    fields: [],
                });
            }
            return;
        }

        if (kwVal === "for") {
            const varTok = peek();
            if (varTok?.type === "IDENT") {
                const name = advance()!.value;
                scope.set(name, "any");
            }
            return;
        }
    }

    while (peek()) {
        try { parseStatement(); } catch { break; }
    }

    return { symbols, imports, scope };
}

function resolveExprType(tokens: Token[], startIdx: number, scope: Map<string, string>): string | undefined {
    let idx = startIdx;
    // Check for ClassName.new(...) pattern
    if (idx < tokens.length && tokens[idx].type === "IDENT") {
        const className = tokens[idx].value;
        if (idx + 1 < tokens.length && tokens[idx + 1].value === "." &&
            idx + 2 < tokens.length && tokens[idx + 2].value === "new") {
            return className;
        }
    }
    // Check for expr:GetService("ServiceName")
    if (idx < tokens.length && tokens[idx].type === "IDENT") {
        let j = idx;
        while (j < tokens.length && tokens[j].value !== ":" && tokens[j].value !== "\n") j++;
        if (j + 1 < tokens.length && tokens[j + 1]?.value === "GetService") {
            const strTok = tokens.find((t, i) => i > j && t.type === "STRING");
            if (strTok) return strTok.value.slice(1, -1);
        }
    }
    // Check scope
    if (idx < tokens.length && tokens[idx].type === "IDENT") {
        return scope.get(tokens[idx].value);
    }
    return undefined;
}
