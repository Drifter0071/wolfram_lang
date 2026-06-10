import { Diagnostic, DiagnosticSeverity } from "vscode-languageserver/node";
import { parseSource } from "./parser";
import { Stmt } from "./ast";

const DECL_KEYWORD_RE = /\b(local|function|class|struct|enum|import|as|for)\s*$/;

export function computeDiagnostics(source: string, filePath?: string): Diagnostic[] {
    const diags: Diagnostic[] = [];

    const result = parseSource(source);

    // Parse errors
    for (const err of result.errors) {
        const pos = extractLineCol(err);
        if (pos) {
            diags.push({
                range: {
                    start: { line: pos.line - 1, character: pos.column - 1 },
                    end: { line: pos.line - 1, character: pos.column + 20 },
                },
                severity: DiagnosticSeverity.Error,
                message: err,
                source: "wolfram-parse",
            });
        } else {
            diags.push({
                range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
                severity: DiagnosticSeverity.Error,
                message: err,
                source: "wolfram-parse",
            });
        }
    }

    // Script-type files (.client.wrm / .server.wrm) should not use public declarations
    if (filePath) {
        checkPublicInScript(source, filePath, result.ast, diags);
    }

    // Scope analysis — AST-driven (replaces regex-based scan)
    collectUndefinedVars(source, result.scope, result.ast, diags);

    return diags;
}

function collectUndefinedVars(
    source: string,
    scope: Map<string, string>,
    ast: Stmt[],
    diags: Diagnostic[]
): void {
    const definedVars = new Set<string>();
    for (const name of BUILTINS) definedVars.add(name);
    for (const [name] of scope) definedVars.add(name);

    // AST-level: add enum variants to defined set, collect struct body spans
    const declarativeSpans: { start: number; end: number }[] = [];
    walkDeclarative(ast, definedVars, declarativeSpans);

    // Find byte offsets inside member-access positions: .ident or :ident
    const memberAccessOffsets = findMemberAccessOffsets(source);

    // Find byte offsets inside type annotation positions (:Type, {[K]:V})
    const typeAnnotationOffsets = findTypeAnnotationOffsets(source);

    const lines = source.split("\n");
    const identRe = /\b([a-zA-Z_]\w*)\b/g;
    let m: RegExpExecArray | null;
    while ((m = identRe.exec(source)) !== null) {
        const name = m[1];
        const offset = m.index;

        // Skip keywords
        if (["if", "else", "elif", "while", "for", "in", "return", "local", "function", "class",
            "struct", "enum", "import", "as", "break", "continue", "true", "false", "nil", "self",
            "public", "private", "try", "catch", "finally", "async", "await", "not", "and", "or", "is"].includes(name)) {
            continue;
        }

        // Skip if preceded by declaration keyword
        const prefix = source.substring(Math.max(0, offset - 15), offset).trimEnd();
        if (DECL_KEYWORD_RE.test(prefix)) continue;

        // Skip f-string prefix (f"..." or f'...')
        if (name === "f" && (source[offset + 1] === '"' || source[offset + 1] === "'")) continue;

        // Skip table field keys ({key = value} or {key: value})
        if (isTableFieldKey(source, offset, name)) continue;

        // Skip if inside a declarative block span (enum/struct body)
        if (declarativeSpans.some(s => offset >= s.start && offset < s.end)) continue;

        // Skip if this is a member-access target (.ident or :ident)
        if (memberAccessOffsets.has(offset)) continue;

        // Skip type annotation identifiers (:Type, {[K]:V})
        if (typeAnnotationOffsets.has(offset)) continue;

        // Skip if defined
        if (definedVars.has(name)) continue;

        // Skip comment/string lines
        const beforeStr = source.substring(0, offset);
        const matchLine = beforeStr.split("\n").length;
        const textLine = lines[matchLine - 1] ?? "";
        if (isCommentLine(textLine)) continue;
        if (isInsideStringLine(source, offset, matchLine)) continue;

        const lastNL = beforeStr.lastIndexOf("\n");
        const col = offset - (lastNL === -1 ? 0 : lastNL + 1) + 1;

        diags.push({
            range: {
                start: { line: matchLine - 1, character: col - 1 },
                end: { line: matchLine - 1, character: col - 1 + name.length },
            },
            severity: DiagnosticSeverity.Warning,
            message: `Undefined variable '${name}'`,
            source: "wolfram-scope",
        });
        definedVars.add(name);
    }
}

// ====================================================================
// HELPERS
// ====================================================================

/** Built-in Luau + Roblox globals that are always defined. */
const BUILTINS = new Set([
    "true", "false", "nil", "self",
    "game", "workspace", "script",
    "Enum", "task",
    "print", "warn", "error",
    "math", "string", "table", "os",
    "require", "pcall", "xpcall",
    "type", "typeof", "tostring", "tonumber",
    "setmetatable", "getmetatable", "rawget", "rawset",
    "unpack", "next", "ipairs", "pairs",
    "tick", "time", "wait", "spawn", "delay",
    "range",
    "Instance", "Vector3", "Vector2", "CFrame",
    "Color3", "BrickColor", "UDim2", "UDim",
    "Ray", "TweenInfo", "Region3", "DateTime",
    "Players", "ReplicatedStorage", "ServerStorage",
    "ServerScriptService", "StarterPlayer", "Workspace",
    "Lighting", "SoundService", "UserInputService",
    "RunService", "ContextActionService", "debris",
    "PluginManager", "settings",
    "OverlapParams", "RaycastParams", "RaycastResult",
    "NumberRange", "NumberSequence", "ColorSequence",
    "PhysicalProperties", "Faces", "Axes", "Rect",
    "Random", "PathWaypoint", "DockWidgetPluginGuiInfo",
]);

/**
 * Scan the source for member-access patterns (.identifier or :identifier)
 * and return the set of byte offsets where those identifiers start.
 * These should NOT be flagged as undefined variables.
 */
function findMemberAccessOffsets(source: string): Set<number> {
    const offsets = new Set<number>();
    // Match .identifier or :identifier — capture the position of the ident
    const re = /[.:]\s*([a-zA-Z_]\w*)/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(source)) !== null) {
        const identStart = m.index + m[0].indexOf(m[1]);
        offsets.add(identStart);
    }
    return offsets;
}

/**
 * Scan source for identifiers in type annotation positions and return their
 * byte offsets so they are not flagged as undefined variables. Type annotation
 * contexts include:
 *   local name: Type          — variable type
 *   param: Type               — function parameter type  
 *   ): ReturnType             — function return type
 *   for var: Type in ...      — loop variable type
 *   {[Key]: Value}            — table type
 *   struct field: Type        — struct field type
 */
function findTypeAnnotationOffsets(source: string): Set<number> {
    const offsets = new Set<number>();
    // local NAME: Type / for NAME: Type / NAME: Type (= param)
    const localRe = /(?:local\s+[\w,]+|\w+|\))\s*:\s*(\w+)/g;
    let m: RegExpExecArray | null;
    while ((m = localRe.exec(source)) !== null) {
        const idx = m[0].lastIndexOf(m[1]);
        if (idx >= 0) offsets.add(m.index + idx);
    }
    // {[Key]: Value} table types
    const tableRe = /\[\s*(\w+)\s*\]\s*:\s*(\w+)/g;
    while ((m = tableRe.exec(source)) !== null) {
        const keyIdx = m[0].lastIndexOf(m[1]);
        if (keyIdx >= 0) offsets.add(m.index + keyIdx);
        const valIdx = m[0].lastIndexOf(m[2]);
        if (valIdx >= 0) offsets.add(m.index + valIdx);
    }
    // struct field: Type (inside struct blocks)
    const structRe = /\b(\w+)\s*:\s*(\w+)\s*(?:,|\n)/g;
    while ((m = structRe.exec(source)) !== null) {
        // Only add the value side (the type), not the struct field name
        const valIdx = m[0].lastIndexOf(m[2]);
        if (valIdx >= 0) offsets.add(m.index + valIdx);
    }
    return offsets;
}

/**
 * Walk AST to collect enum variant names and struct field names into the
 * defined set, and record the source span of enum/struct blocks so that
 * identifiers inside them (e.g. type annotations like "number") are not flagged.
 */
function walkDeclarative(
    stmts: Stmt[],
    defined: Set<string>,
    spans: { start: number; end: number }[]
): void {
    for (const s of stmts) {
        switch (s.kind) {
            case "EnumDef":
                for (const v of s.variants) defined.add(v);
                if (s.span) spans.push(s.span);
                break;
            case "StructDef":
                for (const f of s.fields) defined.add(f.name);
                if (s.span) spans.push(s.span);
                break;
            case "FuncDef":
                walkDeclarative(s.block, defined, spans);
                break;
            case "ClassDef":
                walkDeclarative(s.body, defined, spans);
                break;
            case "If":
                walkDeclarative(s.thenBlock, defined, spans);
                for (const [, b] of s.elseIfBlocks) walkDeclarative(b, defined, spans);
                if (s.elseBlock) walkDeclarative(s.elseBlock, defined, spans);
                break;
            case "While":
            case "For":
                walkDeclarative(s.block, defined, spans);
                break;
            case "TryCatch":
                walkDeclarative(s.tryBlock, defined, spans);
                for (const [, , b] of s.catchClauses) walkDeclarative(b, defined, spans);
                if (s.finallyBlock) walkDeclarative(s.finallyBlock, defined, spans);
                break;
            case "DecoratedStmt":
                walkDeclarative([s.stmt], defined, spans);
                break;
        }
    }
}

/**
 * Detect if the current identifier is a table literal field key.
 * Pattern: { ... Key = value } or { Key: value }
 * Check: preceded by `{` or `,` (with optional whitespace+newlines),
 * and followed by `=` or `:`.
 */
function isTableFieldKey(source: string, offset: number, name: string): boolean {
    // Must be followed by = or :
    const after = source.substring(offset + name.length);
    const nextNonSpace = after.match(/^\s*([=:])/);
    if (!nextNonSpace) return false;

    // Must be preceded by { or ,
    const before = source.substring(0, offset);
    const prevNonSpace = before.match(/[{,]\s*$/);
    return prevNonSpace !== null;
}

/**
 * Warn when `public` declarations appear in .client.wrm or .server.wrm files.
 * Script files don't produce a module return table, so public exports are ignored.
 */
function checkPublicInScript(
    source: string,
    filePath: string,
    ast: Stmt[],
    diags: Diagnostic[]
): void {
    const lower = filePath.toLowerCase();
    if (!lower.includes(".client.") && !lower.endsWith(".client")
        && !lower.includes(".server.") && !lower.endsWith(".server")) {
        return; // Module file — ok
    }

    const scriptType = lower.includes(".client.") || lower.endsWith(".client") ? "client" : "server";

    const publicStmts = findPublicDeclarations(ast);
    if (publicStmts.length === 0) return;

    const firstLine = publicStmts[0];
    diags.push({
        range: {
            start: { line: firstLine.line, character: 0 },
            end: { line: firstLine.line, character: 6 },
        },
        severity: DiagnosticSeverity.Warning,
        message: `Public declarations in .${scriptType}.wrm files have no effect — scripts don't export a module table. Use a module file (no suffix) for shared code, or remove 'public'.`,
        source: "wolfram-script-type",
    });
}

/**
 * Find all AST statements with `access === "public"`.
 */
function findPublicDeclarations(stmts: Stmt[]): { line: number }[] {
    const result: { line: number }[] = [];
    for (const s of stmts) {
        if ("access" in s && (s as any).access === "public") {
            result.push({ line: ((s as any).span?.start ?? 0) });
        }
        switch (s.kind) {
            case "ClassDef":
                findPublicDeclarations(s.body).forEach(r => { r.line = Math.max(r.line, 0); result.push(r); });
                break;
            case "FuncDef":
                findPublicDeclarations(s.block).forEach(r => { r.line = Math.max(r.line, 0); result.push(r); });
                break;
            case "If":
                findPublicDeclarations(s.thenBlock);
                for (const [, b] of s.elseIfBlocks) findPublicDeclarations(b);
                if (s.elseBlock) findPublicDeclarations(s.elseBlock);
                break;
        }
    }
    return result;
}

function extractLineCol(msg: string): { line: number; column: number } | null {
    const m = msg.match(/line\s+(\d+),\s*column\s+(\d+)/);
    return m ? { line: parseInt(m[1]), column: parseInt(m[2]) } : null;
}

function isCommentLine(line: string): boolean {
    const trimmed = line.trimStart();
    return trimmed.startsWith("//") || trimmed.startsWith("--");
}

function isInsideStringLine(source: string, byteIndex: number, matchLineNum: number): boolean {
    const allLines = source.split("\n");
    let lineStart = 0;
    for (let i = 0; i < matchLineNum - 1; i++) lineStart += (allLines[i]?.length ?? 0) + 1;
    const colInLine = byteIndex - lineStart;
    const line = allLines[matchLineNum - 1] ?? "";
    let inStr = false;
    let quote = "";
    for (let i = 0; i < colInLine && i < line.length; i++) {
        const c = line[i];
        if (!inStr && (c === '"' || c === "'")) { inStr = true; quote = c; }
        else if (inStr && c === quote) { inStr = false; quote = ""; }
    }
    return inStr;
}
