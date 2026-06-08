import { Diagnostic, DiagnosticSeverity } from "vscode-languageserver/node";
import { parseSource } from "./parser";

const DECL_KEYWORD_RE = /\b(local|function|class|struct|enum|import|as|for)\s*$/;

export function computeDiagnostics(source: string): Diagnostic[] {
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

    // Scope analysis — only when parser had errors, otherwise parsed scope is trusted
    if (result.errors.length > 0) {
        collectUndefinedVars(source, result.scope, diags);
    }

    return diags;
}

function collectUndefinedVars(source: string, scope: Map<string, string>, diags: Diagnostic[]): void {
    const definedVars = new Set<string>();
    definedVars.add("true"); definedVars.add("false"); definedVars.add("nil"); definedVars.add("self");
    definedVars.add("game"); definedVars.add("workspace"); definedVars.add("script");
    definedVars.add("Enum"); definedVars.add("task");
    definedVars.add("print"); definedVars.add("warn"); definedVars.add("error");
    definedVars.add("math"); definedVars.add("string"); definedVars.add("table"); definedVars.add("os");
    definedVars.add("require"); definedVars.add("pcall"); definedVars.add("xpcall");
    definedVars.add("type"); definedVars.add("typeof"); definedVars.add("tostring"); definedVars.add("tonumber");
    definedVars.add("setmetatable"); definedVars.add("getmetatable"); definedVars.add("rawget"); definedVars.add("rawset");
    definedVars.add("unpack"); definedVars.add("next"); definedVars.add("ipairs"); definedVars.add("pairs");
    definedVars.add("tick"); definedVars.add("time"); definedVars.add("wait"); definedVars.add("spawn"); definedVars.add("delay");
    definedVars.add("Instance"); definedVars.add("Vector3"); definedVars.add("Vector2"); definedVars.add("CFrame");
    definedVars.add("Color3"); definedVars.add("BrickColor"); definedVars.add("UDim2"); definedVars.add("UDim");
    definedVars.add("Ray"); definedVars.add("TweenInfo"); definedVars.add("Region3"); definedVars.add("DateTime");
    definedVars.add("Players"); definedVars.add("ReplicatedStorage"); definedVars.add("ServerStorage");
    definedVars.add("ServerScriptService"); definedVars.add("StarterPlayer"); definedVars.add("Workspace");
    definedVars.add("Lighting"); definedVars.add("SoundService"); definedVars.add("UserInputService");
    definedVars.add("RunService"); definedVars.add("ContextActionService"); definedVars.add("debris");
    definedVars.add("PluginManager"); definedVars.add("settings");

    for (const [name] of scope) definedVars.add(name);

    const lines = source.split("\n");
    const identRe = /\b([a-zA-Z_]\w*)\b/g;
    let m: RegExpExecArray | null;
    while ((m = identRe.exec(source)) !== null) {
        const name = m[1];
        if (["if", "else", "elif", "while", "for", "in", "return", "local", "function", "class",
            "struct", "enum", "import", "as", "break", "continue", "true", "false", "nil", "self",
            "public", "private", "try", "catch", "finally", "async", "await", "not", "and", "or", "is"].includes(name)) {
            continue;
        }
        const prefix = source.substring(Math.max(0, m.index - 15), m.index).trimEnd();
        if (DECL_KEYWORD_RE.test(prefix)) continue;

        const matchLine = source.substring(0, m.index).split("\n").length;
        const textLine = lines[matchLine - 1] ?? "";
        if (isCommentLine(textLine)) continue;
        if (isInsideStringLine(source, m.index, matchLine)) continue;

        if (!definedVars.has(name)) {
            const beforeStr = source.substring(0, m.index);
            const l = beforeStr.split("\n").length;
            const lastNL = beforeStr.lastIndexOf("\n");
            const col = (m.index) - (lastNL === -1 ? 0 : lastNL + 1) + 1;
            diags.push({
                range: { start: { line: l - 1, character: col - 1 }, end: { line: l - 1, character: col - 1 + name.length } },
                severity: DiagnosticSeverity.Warning,
                message: `Undefined variable '${name}'`,
                source: "wolfram-scope",
            });
            definedVars.add(name);
        }
    }
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
