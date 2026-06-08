import { WorkspaceEdit, TextEdit, PrepareRenameResult, Range } from "vscode-languageserver/node";
import { DocumentStore } from "./store";

export function handlePrepareRename(store: DocumentStore, uri: string, line: number, character: number): PrepareRenameResult | null {
    const doc = store.get(uri);
    if (!doc) return null;

    const word = extractWordAt(doc.source, line, character);
    if (!word || isKeyword(word)) return null;

    const symbolExists = doc.symbols.some(s => s.name === word) ||
        doc.scope.variables.has(word);
    if (!symbolExists) return null;

    const range = wordRange(doc.source, line, character);
    return { range, placeholder: word };
}

export function handleRename(store: DocumentStore, uri: string, line: number, character: number, newName: string): WorkspaceEdit | null {
    const doc = store.get(uri);
    if (!doc) return null;

    const oldName = extractWordAt(doc.source, line, character);
    if (!oldName || isKeyword(oldName) || oldName === newName) return null;

    const changes: Record<string, TextEdit[]> = {};

    // Rename in current file
    const localEdits = findOccurrences(doc.source, oldName);
    if (localEdits.length > 0) {
        changes[uri] = localEdits.map(e => ({ range: e, newText: newName }));
    }

    // Rename across workspace for public symbols
    const isPublic = doc.symbols.some(s => s.name === oldName && s.access === "public");
    if (isPublic) {
        for (const otherDoc of store.getAll()) {
            if (otherDoc.uri === uri) continue;
            const edits = findOccurrences(otherDoc.source, oldName);
            if (edits.length > 0) {
                changes[otherDoc.uri] = edits.map(e => ({ range: e, newText: newName }));
            }
        }
    }

    if (Object.keys(changes).length === 0) return null;
    return { changes };
}

function findOccurrences(source: string, word: string): Range[] {
    const ranges: Range[] = [];
    const lines = source.split("\n");
    for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
        const line = lines[lineIdx];
        let start = 0;
        while (true) {
            const found = line.indexOf(word, start);
            if (found === -1) break;
            const before = found > 0 ? line[found - 1] : " ";
            const after = found + word.length < line.length ? line[found + word.length] : " ";
            if (isWordBoundary(before) && isWordBoundary(after)) {
                ranges.push({
                    start: { line: lineIdx, character: found },
                    end: { line: lineIdx, character: found + word.length },
                });
            }
            start = found + word.length;
        }
    }
    return ranges;
}

function extractWordAt(source: string, line: number, character: number): string {
    const lines = source.split("\n");
    const l = lines[line] ?? "";
    if (character >= l.length) return "";
    const bytes = l;
    let start = character;
    while (start > 0 && isIdChar(bytes[start - 1])) start--;
    let end = character;
    while (end < l.length && isIdChar(bytes[end])) end++;
    return l.substring(start, end);
}

function wordRange(source: string, _line: number, _character: number): Range {
    const lines = source.split("\n");
    const l = lines[_line] ?? "";
    let start = _character;
    while (start > 0 && isIdChar(l[start - 1])) start--;
    let end = _character;
    while (end < l.length && isIdChar(l[end])) end++;
    return { start: { line: _line, character: start }, end: { line: _line, character: end } };
}

function isIdChar(c: string): boolean {
    return /[\w]/.test(c);
}

function isWordBoundary(c: string): boolean {
    return !/[\w]/.test(c);
}

function isKeyword(word: string): boolean {
    return ["if", "else", "elif", "while", "for", "in", "function", "class", "struct", "enum",
        "import", "as", "local", "return", "true", "false", "nil", "self", "break", "continue",
        "and", "or", "not", "public", "private", "try", "catch", "finally", "async", "await", "is"].includes(word);
}
