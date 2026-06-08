import {
    createConnection,
    TextDocuments,
    ProposedFeatures,
    InitializeParams,
    InitializeResult,
    CompletionItem,
    TextDocumentSyncKind,
    TextDocumentPositionParams,
    HoverParams,
    Hover,
    DefinitionParams,
    Location,
    DocumentSymbolParams,
    DocumentSymbol,
    SignatureHelpParams,
    SignatureHelp,
    CodeActionParams,
    CodeAction,
    CodeActionKind,
    TextEdit,
    WorkspaceEdit,
} from "vscode-languageserver/node";

const SymbolKind = {
    File: 1, Module: 2, Namespace: 3, Package: 4, Class: 5, Method: 6,
    Property: 7, Field: 8, Constructor: 9, Enum: 10, Interface: 11,
    Function: 12, Variable: 13, Constant: 14, String: 15, Number: 16,
    Boolean: 17, Array: 18, Object: 19, Key: 20, Null: 21,
    EnumMember: 22, Struct: 23, Event: 24, Operator: 25, TypeParameter: 26,
} as const;
import { TextDocument } from "vscode-languageserver-textdocument";
import { Bindings } from "./bindings";
import { computeDiagnostics } from "./diagnostics";
import { handleCompletion } from "./completion";
import { handleHover } from "./hover";
import { parseDocument } from "./parser";
import { collectProjectWrmFiles } from "./utils";
import * as path from "path";
import * as fs from "fs";

const connection = createConnection(ProposedFeatures.all);
const documents: TextDocuments<TextDocument> = new TextDocuments(TextDocument);
const bindings = new Bindings();

let workspaceRoot: string | null = null;
let workspaceFiles: string[] = [];

connection.onInitialize((params: InitializeParams): InitializeResult => {
    workspaceRoot = params.rootUri ? new URL(params.rootUri).pathname : null;
    if (workspaceRoot && process.platform === "win32") {
        workspaceRoot = workspaceRoot.replace(/^\/([a-zA-Z]:)/, "$1");
    }

    const bindingsDir = findBindingsDir();
    bindings.load(bindingsDir);

    if (workspaceRoot) {
        workspaceFiles = collectProjectWrmFiles(workspaceRoot);
    }

    return {
        capabilities: {
            textDocumentSync: TextDocumentSyncKind.Incremental,
            completionProvider: {
                triggerCharacters: [".", ":"],
                resolveProvider: false,
            },
            hoverProvider: true,
            definitionProvider: true,
            documentSymbolProvider: true,
            signatureHelpProvider: {
                triggerCharacters: ["(", ","],
            },
            codeActionProvider: true,
        },
        serverInfo: {
            name: "wolfram-typescript-lsp",
            version: "1.0.0",
        },
    };
});

function findBindingsDir(): string {
    if (workspaceRoot) {
        const local = path.join(workspaceRoot, "generated", "roblox.wold");
        if (fs.existsSync(local)) return workspaceRoot;
    }
    const candidate = path.join(__dirname, "..", "..");
    const wold = path.join(candidate, "generated", "roblox.wold");
    if (fs.existsSync(wold)) return candidate;
    return __dirname;
}

documents.onDidChangeContent((change) => {
    const diags = computeDiagnostics(change.document);
    connection.sendDiagnostics({ uri: change.document.uri, diagnostics: diags });
});

documents.onDidOpen((open) => {
    const diags = computeDiagnostics(open.document);
    connection.sendDiagnostics({ uri: open.document.uri, diagnostics: diags });
});

connection.onCompletion((params: TextDocumentPositionParams): CompletionItem[] => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return [];
    return handleCompletion(doc, params.position.line, params.position.character, bindings, workspaceFiles);
});

connection.onHover((params: HoverParams): Hover | null => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return null;
    return handleHover(doc, params.position.line, params.position.character, bindings);
});

connection.onDefinition((params: DefinitionParams): Location | Location[] | null => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return null;

    const source = doc.getText();
    const lines = source.split("\n");
    const line = lines[params.position.line] ?? "";
    const word = extractWordAt(line, params.position.character);

    // Check local symbols
    const parsed = parseDocument(source);
    const sym = parsed.symbols.find(s => s.name === word);
    if (sym) {
        return {
            uri: params.textDocument.uri,
            range: {
                start: { line: (sym.location.line as number) - 1, character: (sym.location.column as number) - 1 },
                end: { line: (sym.location.endLine as number) - 1, character: (sym.location.endColumn as number) - 1 },
            },
        };
    }

    // Check imports (for alias.Member pattern)
    const extended = extractExtendedExpr(line, params.position.character);
    if (extended.includes(".")) {
        const dotPos = extended.indexOf(".");
        const aliasPart = extended.substring(0, dotPos);
        const importDef = parsed.imports.find(i => i.alias === aliasPart);
        if (importDef && workspaceRoot) {
            // Try to find the imported file
            const importPath = importDef.path;
            for (const f of workspaceFiles) {
                if (f.toLowerCase() === importPath.toLowerCase() || f.toLowerCase().endsWith("/" + importPath.toLowerCase())) {
                    const fullPath = path.join(workspaceRoot, "src", f + ".wrm");
                    if (fs.existsSync(fullPath)) {
                        const targetUri = "file://" + fullPath.replace(/\\/g, "/");
                        return {
                            uri: targetUri,
                            range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } },
                        };
                    }
                }
            }
            // Return a location that points to the import file (best effort)
            const importFile = path.join(workspaceRoot, "src", importPath + ".wrm");
            if (fs.existsSync(importFile)) {
                return {
                    uri: "file://" + importFile.replace(/\\/g, "/"),
                    range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } },
                };
            }
        }
    }

    return null;
});

connection.onDocumentSymbol((params: DocumentSymbolParams): DocumentSymbol[] => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return [];
    const parsed = parseDocument(doc.getText());
    return parsed.symbols.map(s => {
        const kind = s.kind === "function" ? SymbolKind.Function
            : s.kind === "class" ? SymbolKind.Class
            : s.kind === "struct" ? SymbolKind.Struct
            : s.kind === "enum" ? SymbolKind.Enum
            : SymbolKind.Variable;
        return {
            name: s.name,
            kind,
            range: {
                start: { line: s.location.line - 1, character: s.location.column - 1 },
                end: { line: s.location.endLine - 1, character: s.location.endColumn - 1 },
            },
            selectionRange: {
                start: { line: s.location.line - 1, character: s.location.column - 1 },
                end: { line: s.location.line - 1, character: s.location.column - 1 + s.name.length },
            },
        };
    });
});

connection.onSignatureHelp((params: SignatureHelpParams): SignatureHelp | null => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return null;

    const source = doc.getText();
    const lines = source.split("\n");
    const line = lines[params.position.line] ?? "";
    const col = params.position.character;

    const funcName = extractCallableBefore(line, col, source, params.position.line);
    if (!funcName) return null;

    // Try Roblox functions
    const f = bindings.getFunction(funcName);
    if (f) {
        const params = f.params.map(p => ({ label: `${p.name}: ${p.type}` }));
        return {
            signatures: [{
                label: `${f.name}(${f.params.map(p => `${p.name}: ${p.type}`).join(", ")}): ${f.returns}`,
                parameters: params,
                documentation: f.description ? { kind: "markdown" as const, value: f.description } : undefined,
            }],
            activeSignature: 0,
            activeParameter: countCommasBefore(line, col),
        };
    }

    // Try method calls (obj:method)
    if (funcName.includes(":")) {
        const [obj, method] = funcName.split(":");
        const parsed = parseDocument(source);
        const typeName = parsed.scope.get(obj);
        if (typeName && typeName !== "any") {
            const methods = bindings.getAllMethods(typeName);
            const m = methods.find(m => m.name.toLowerCase() === method.toLowerCase());
            if (m) {
                const params = m.params.map(p => ({ label: `${p.name}: ${p.type}` }));
                return {
                    signatures: [{
                        label: `${obj}:${m.name}(${m.params.map(p => `${p.name}: ${p.type}`).join(", ")}): ${m.returns}`,
                        parameters: params,
                        documentation: m.description ? { kind: "markdown" as const, value: m.description } : undefined,
                    }],
                    activeSignature: 0,
                    activeParameter: countCommasBefore(line, col),
                };
            }
        }
    }

    return null;
});

function extractWordAt(line: string, col: number): string {
    if (col >= line.length) return "";
    const start = line.substring(0, col).search(/[\w.]+$/);
    const startIdx = start === -1 ? col : line.substring(0, col).length - ((line.substring(0, col).length - start) - 1);
    const end = line.substring(col).search(/\W|$/);
    const endIdx = end === -1 ? line.length : col + end;
    return line.substring(startIdx, endIdx);
}

function extractExtendedExpr(line: string, col: number): string {
    const start = line.substring(0, col).search(/[^\w.]+$/);
    const startIdx = start === -1 ? 0 : start + 1;
    const end = line.substring(col).search(/[^\w.]+/);
    const endIdx = end === -1 ? line.length : col + end;
    return line.substring(startIdx, endIdx);
}

function extractCallableBefore(line: string, col: number, source: string, lineNum: number): string | null {
    // Find opening paren walking backwards through source
    const allLines = source.split("\n");
    let offset = 0;
    for (let i = 0; i < lineNum; i++) offset += (allLines[i]?.length ?? 0) + 1;
    offset += col;

    let depth = 0;
    for (let i = offset - 1; i >= 0; i--) {
        const c = source[i];
        if (c === ")" || c === "}" || c === "]") depth++;
        else if (c === "(" || c === "{" || c === "[") {
            if (depth === 0 && c === "(") {
                let end = i;
                while (end > 0 && source[end - 1] === " ") end--;
                let start = end;
                while (start > 0 && /[\w.:]/.test(source[start - 1])) start--;
                return source.substring(start, end);
            }
            depth = Math.max(0, depth - 1);
        }
    }
    return null;
}

function countCommasBefore(line: string, col: number): number {
    let count = 0;
    let depth = 0;
    for (let i = 0; i < col && i < line.length; i++) {
        if (line[i] === "(" || line[i] === "[" || line[i] === "{") depth++;
        if (line[i] === ")" || line[i] === "]" || line[i] === "}") depth = Math.max(0, depth - 1);
        if (line[i] === "," && depth === 0) count++;
    }
    return count;
}

connection.onCodeAction((params: CodeActionParams): CodeAction[] => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return [];
    const diagnostics = params.context.diagnostics;
    const actions: CodeAction[] = [];

    for (const diag of diagnostics) {
        if (diag.message.includes(".length")) {
            const word = extractWordAt(doc.getText().split("\n")[diag.range.start.line] ?? "", diag.range.start.character);
            const varName = word.replace(/\.length$/, "");
            const edit: TextEdit = { range: diag.range, newText: `#${varName}` };
            actions.push({
                title: `Replace '.length' with '#${varName}'`,
                kind: CodeActionKind.QuickFix,
                diagnostics: [diag],
                edit: { changes: { [params.textDocument.uri]: [edit] } },
            });
        }
        if (diag.message.includes("len(")) {
            const line = (doc.getText().split("\n")[diag.range.start.line] ?? "");
            const col = diag.range.start.character;
            const endParen = line.indexOf(")", col);
            if (endParen > col) {
                const arg = line.substring(col + 4, endParen).trim();
                actions.push({
                    title: `Replace 'len(${arg})' with '#${arg}'`,
                    kind: CodeActionKind.QuickFix,
                    diagnostics: [diag],
                    edit: {
                        changes: {
                            [params.textDocument.uri]: [{
                                range: { start: diag.range.start, end: { line: diag.range.start.line, character: endParen + 1 } },
                                newText: `#${arg}`,
                            }],
                        },
                    },
                });
            }
        }
        if (diag.message.includes("undeclared variable") || diag.message.includes("undefined variable")) {
            const word = extractWordAt(doc.getText().split("\n")[diag.range.start.line] ?? "", diag.range.start.character);
            const insertPos = diag.range.start;
            actions.push({
                title: `Add 'local ${word}' declaration`,
                kind: CodeActionKind.QuickFix,
                diagnostics: [diag],
                edit: {
                    changes: {
                        [params.textDocument.uri]: [{
                            range: { start: insertPos, end: insertPos },
                            newText: `local ${word} = `,
                        }],
                    },
                },
            });
        }
    }

    return actions;
});

documents.listen(connection);
connection.listen();
