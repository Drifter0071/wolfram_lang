import { WorkspaceSymbol, SymbolKind } from "vscode-languageserver/node";
import { DocumentStore } from "./store";
import { Stmt } from "./ast";

export function computeWorkspaceSymbols(store: DocumentStore, query: string): WorkspaceSymbol[] {
    if (!query) return [];
    const lower = query.toLowerCase();
    const results: WorkspaceSymbol[] = [];

    for (const doc of store.getAll()) {
        for (const stmt of doc.ast) {
            const info = stmtToSymbol(stmt, doc.uri, lower);
            if (info) results.push(info);
        }
    }

    return results.slice(0, 50);
}

function stmtToSymbol(stmt: Stmt, uri: string, query: string): WorkspaceSymbol | null {
    switch (stmt.kind) {
        case "FuncDef":
            if (!stmt.name.toLowerCase().includes(query)) return null;
            return { name: stmt.name, kind: SymbolKind.Function, location: { uri, range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } } } };
        case "ClassDef":
            if (!stmt.name.toLowerCase().includes(query)) return null;
            return { name: stmt.name, kind: SymbolKind.Class, location: { uri, range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } } } };
        case "EnumDef":
            if (!stmt.name.toLowerCase().includes(query)) return null;
            return { name: stmt.name, kind: SymbolKind.Enum, location: { uri, range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } } } };
        case "StructDef":
            if (!stmt.name.toLowerCase().includes(query)) return null;
            return { name: stmt.name, kind: SymbolKind.Struct, location: { uri, range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } } } };
        case "Local":
            if (!stmt.name.toLowerCase().includes(query)) return null;
            return { name: stmt.name, kind: SymbolKind.Variable, location: { uri, range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } } } };
        default:
            return null;
    }
}
