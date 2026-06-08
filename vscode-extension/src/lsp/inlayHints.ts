import { InlayHint, InlayHintKind } from "vscode-languageserver/node";
import { DocumentStore } from "./store";
import { Stmt } from "./ast";
import { inferExprType } from "./parser";

export function computeInlayHints(store: DocumentStore, uri: string): InlayHint[] {
    const hints: InlayHint[] = [];
    const doc = store.get(uri);
    if (!doc) return hints;

    for (const stmt of doc.ast) {
        hints.push(...extractTypeHints(stmt, doc.source));
    }

    return hints;
}

function extractTypeHints(stmt: Stmt, source: string): InlayHint[] {
    const hints: InlayHint[] = [];
    if (stmt.kind === "Local" && stmt.value) {
        const typeStr = inferExprType(stmt.value);
        if (typeStr !== "any" && typeStr !== "" && typeStr !== stmt.name) {
            const pos = findIdentEnd(source, stmt.name);
            if (pos) {
                hints.push({
                    position: { line: pos.line, character: pos.character },
                    label: `: ${typeStr}`,
                    kind: InlayHintKind.Type,
                    paddingLeft: false,
                    paddingRight: true,
                });
            }
        }
    }
    return hints;
}

function findIdentEnd(source: string, name: string): { line: number; character: number } | null {
    const idx = source.indexOf(name);
    if (idx === -1) return null;
    const end = idx + name.length;
    const prefix = source.substring(0, end);
    const line = prefix.split("\n").length - 1;
    const lastNL = prefix.lastIndexOf("\n");
    const col = end - (lastNL === -1 ? 0 : lastNL + 1);
    return { line, character: col };
}
