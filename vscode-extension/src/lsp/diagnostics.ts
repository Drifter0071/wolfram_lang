import { Diagnostic, DiagnosticSeverity } from "vscode-languageserver/node";
import { parseSource } from "./parser";

const DECL_KEYWORD_RE = /\b(local|function|class|struct|enum|import|as|for)\s*$/;

export function computeDiagnostics(source: string): Diagnostic[] {
    const diags: Diagnostic[] = [];

    // Parse the source
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

    // Luau compatibility diagnostics
    const lines = source.split("\n");
    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const trimmed = line.trimStart();

        // .length usage
        if (line.includes(".length")) {
            const pos = line.indexOf(".length");
            diags.push({
                range: {
                    start: { line: i, character: pos },
                    end: { line: i, character: pos + 7 },
                },
                severity: DiagnosticSeverity.Warning,
                message: "Luau uses # operator instead of .length",
                source: "wolfram-luau",
            });
        }

        // len() function
        if (trimmed.match(/len\s*\(/)) {
            const pos = line.indexOf("len");
            diags.push({
                range: {
                    start: { line: i, character: pos },
                    end: { line: i, character: pos + 3 },
                },
                severity: DiagnosticSeverity.Warning,
                message: "Luau uses # operator instead of len()",
                source: "wolfram-luau",
            });
        }
    }

    // Scope analysis - undefined variables
    const definedVars = new Set<string>();
    definedVars.add("true"); definedVars.add("false"); definedVars.add("nil"); definedVars.add("self");
    definedVars.add("game"); definedVars.add("script");
    definedVars.add("print"); definedVars.add("warn"); definedVars.add("error");
    definedVars.add("math"); definedVars.add("string"); definedVars.add("table"); definedVars.add("os");
    definedVars.add("require"); definedVars.add("pcall"); definedVars.add("xpcall");
    definedVars.add("type"); definedVars.add("typeof"); definedVars.add("tostring"); definedVars.add("tonumber");
    definedVars.add("setmetatable"); definedVars.add("getmetatable"); definedVars.add("rawget"); definedVars.add("rawset");

    // Add defined variables from scope
    for (const [name] of result.scope) {
        definedVars.add(name);
    }

    // Find references to undefined variables
    const identRe = /\b([a-zA-Z_]\w*)\b/g;
    let m: RegExpExecArray | null;
    while ((m = identRe.exec(source)) !== null) {
        const name = m[1];
        // Skip keywords
        if (["if", "else", "elif", "while", "for", "in", "return", "local", "function", "class",
            "struct", "enum", "import", "as", "break", "continue", "true", "false", "nil", "self",
            "public", "private", "try", "catch", "finally", "async", "await", "not", "and", "or", "is"].includes(name)) {
            continue;
        }
        // Skip after keywords
        const prefix = source.substring(Math.max(0, m.index - 15), m.index).trimEnd();
        if (DECL_KEYWORD_RE.test(prefix)) continue;

        if (!definedVars.has(name) && !definedVars.has(name)) {
            const beforeStr = source.substring(0, m.index);
            const line = beforeStr.split("\n").length;
            const lastNL = beforeStr.lastIndexOf("\n");
            const col = (m.index) - (lastNL === -1 ? 0 : lastNL + 1) + 1;
            diags.push({
                range: {
                    start: { line: line - 1, character: col - 1 },
                    end: { line: line - 1, character: col - 1 + name.length },
                },
                severity: DiagnosticSeverity.Warning,
                message: `Undefined variable '${name}'`,
                source: "wolfram-scope",
            });
            // Add to defined so we don't repeat
            definedVars.add(name);
        }
    }

    return diags;
}

function extractLineCol(msg: string): { line: number; column: number } | null {
    const m = msg.match(/line\s+(\d+),\s*column\s+(\d+)/);
    if (m) return { line: parseInt(m[1]), column: parseInt(m[2]) };
    return null;
}
