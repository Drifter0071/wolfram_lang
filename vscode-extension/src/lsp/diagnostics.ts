import { TextDocument } from "vscode-languageserver-textdocument";
import { Diagnostic, DiagnosticSeverity, Range, Position } from "vscode-languageserver/node";
import { parseDocument } from "./parser";

export function computeDiagnostics(document: TextDocument): Diagnostic[] {
    const diagnostics: Diagnostic[] = [];
    const source = document.getText();
    const lines = source.split("\n");

    // Check for basic parse errors
    let braceDepth = 0;
    for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
        const line = lines[lineIdx];
        for (let colIdx = 0; colIdx < line.length; colIdx++) {
            const ch = line[colIdx];
            if (ch === "{") braceDepth++;
            if (ch === "}") braceDepth--;
        }
    }
    if (braceDepth !== 0) {
        diagnostics.push({
            severity: DiagnosticSeverity.Error,
            range: Range.create(lines.length - 1, 0, lines.length - 1, (lines[lines.length - 1]?.length ?? 0)),
            message: "Unmatched braces",
            source: "wolfram-parser",
        });
    }

    // Check for .length usage on table/list variables (warning)
    const lenRe = /\b(\w+)\.length\b/g;
    let m: RegExpExecArray | null;
    while ((m = lenRe.exec(source)) !== null) {
        const before = source.lastIndexOf("\n", m.index) + 1;
        const lineIdx = source.substring(0, m.index).split("\n").length - 1;
        const colStart = m.index - before;
        diagnostics.push({
            severity: DiagnosticSeverity.Warning,
            range: Range.create(lineIdx, colStart, lineIdx, colStart + m[0].length),
            message: `".length" is invalid in Luau. Use #${m[1]} for array length.`,
            source: "wolfram-luau",
        });
    }

    // Parse and check scope issues
    try {
        const parsed = parseDocument(source);
        // Check for unused vars (simple: if declared then never referenced again)
        // This is basic - won't catch all cases
        for (const [name] of parsed.scope) {
            const usageRe = new RegExp(`\\b${name}\\b`, "g");
            const allMatches = source.match(usageRe);
            if (allMatches && allMatches.length <= 1 && name !== "self") {
                const declLine = source.split("\n").findIndex(l => l.includes(name) && l.includes("local") && !l.includes("function"));
                if (declLine >= 0) {
                    diagnostics.push({
                        severity: DiagnosticSeverity.Hint,
                        range: Range.create(declLine, 0, declLine, lines[declLine]?.length ?? 0),
                        message: `Variable "${name}" is declared but never used.`,
                        source: "wolfram-scope",
                    });
                }
            }
        }
    } catch {
        // Silently fail if parsing fails
    }

    return diagnostics;
}
