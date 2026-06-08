import { Stmt } from "./ast";
import { parseSource, ParseResult } from "./parser";

export interface ScopeMap {
    variables: Map<string, string>;
}

export interface DocumentState {
    uri: string;
    source: string;
    ast: Stmt[];
    symbols: { name: string; kind: string; access: string; location: { line: number; column: number; endLine: number; endColumn: number }; params: string[]; fields: string[] }[];
    imports: { path: string; alias: string }[];
    scope: ScopeMap;
    dirty: boolean;
    lineCount: number;
}

export class DocumentStore {
    private documents = new Map<string, DocumentState>();

    open(uri: string, source: string): void {
        const state = parseDocument(uri, source);
        this.documents.set(uri, state);
    }

    update(uri: string, source: string): void {
        const existing = this.documents.get(uri);
        if (existing) {
            existing.source = source;
            existing.dirty = true;
            existing.lineCount = source.split("\n").length;
        }
    }

    get(uri: string): DocumentState | undefined {
        return this.documents.get(uri);
    }

    getOrParse(uri: string, source: string): DocumentState {
        const existing = this.documents.get(uri);
        if (existing && !existing.dirty) return existing;
        const state = parseDocument(uri, source);
        this.documents.set(uri, state);
        return state;
    }

    close(uri: string): void {
        this.documents.delete(uri);
    }

    getAll(): DocumentState[] {
        return Array.from(this.documents.values());
    }

    findByName(fileName: string): DocumentState | undefined {
        for (const doc of this.documents.values()) {
            const uriPath = doc.uri.replace(/\\/g, "/");
            if (uriPath.endsWith(`/${fileName}`) || uriPath.endsWith(fileName)) {
                return doc;
            }
        }
        return undefined;
    }
}

function parseDocument(uri: string, source: string): DocumentState {
    const result: ParseResult = parseSource(source);

    return {
        uri,
        source,
        ast: result.ast,
        symbols: result.symbols,
        imports: result.imports,
        scope: { variables: result.scope },
        dirty: false,
        lineCount: source.split("\n").length,
    };
}
