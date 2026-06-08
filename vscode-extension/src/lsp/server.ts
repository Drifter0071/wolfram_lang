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
    SemanticTokensParams,
    SemanticTokensRangeParams,
    RenameParams,
} from "vscode-languageserver/node";

import { TextDocument } from "vscode-languageserver-textdocument";
import { Bindings } from "./bindings";
import { computeDiagnostics } from "./diagnostics";
import { handleHover } from "./hover";
import { handleCompletion } from "./completion";
import { parseSource } from "./parser";
import { DocumentStore } from "./store";
import { computeSemanticTokens } from "./semanticTokens";
import { handlePrepareRename, handleRename } from "./rename";
import { computeInlayHints } from "./inlayHints";
import { computeWorkspaceSymbols } from "./workspaceSymbols";
import { collectProjectWrmFiles, extractWordAround } from "./utils";
import * as path from "path";
import * as fs from "fs";

const connection = createConnection(ProposedFeatures.all);
const documents: TextDocuments<TextDocument> = new TextDocuments(TextDocument);
const bindings = new Bindings();
const store = new DocumentStore();

let workspaceRoot: string | null = null;
let workspaceFiles: string[] = [];

const SymbolKind = {
    File: 1, Module: 2, Namespace: 3, Package: 4, Class: 5, Method: 6,
    Property: 7, Field: 8, Constructor: 9, Enum: 10, Interface: 11,
    Function: 12, Variable: 13, Constant: 14, String: 15, Number: 16,
    Boolean: 17, Array: 18, Object: 19, Key: 20, Null: 21,
    EnumMember: 22, Struct: 23, Event: 24, Operator: 25, TypeParameter: 26,
} as const;

// ==========================================
// INITIALIZE
// ==========================================
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
            renameProvider: {
                prepareProvider: true,
            },
            semanticTokensProvider: {
                legend: {
                    tokenTypes: [
                        "namespace", "type", "class", "function", "property",
                        "method", "variable", "parameter", "keyword", "string",
                        "number", "comment", "operator", "decorator",
                    ],
                    tokenModifiers: ["declaration", "definition", "readonly", "static", "async"],
                },
                full: true,
                range: true,
            },
            inlayHintProvider: true,
            workspaceSymbolProvider: true,
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

// ==========================================
// DOCUMENT SYNC
// ==========================================
documents.onDidOpen((open) => {
    store.open(open.document.uri, open.document.getText());
    const diags = computeDiagnostics(open.document.getText());
    connection.sendDiagnostics({ uri: open.document.uri, diagnostics: diags });
});

documents.onDidChangeContent((change) => {
    store.update(change.document.uri, change.document.getText());
    const diags = computeDiagnostics(change.document.getText());
    connection.sendDiagnostics({ uri: change.document.uri, diagnostics: diags });
});

documents.onDidClose((close) => {
    store.close(close.document.uri);
});

// ==========================================
// COMPLETION
// ==========================================
connection.onCompletion((params: TextDocumentPositionParams): CompletionItem[] => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return [];
    return handleCompletion(doc, params.position.line, params.position.character, bindings, workspaceFiles);
});

// ==========================================
// HOVER
// ==========================================
connection.onHover((params: HoverParams): Hover | null => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return null;
    return handleHover(doc, params.position.line, params.position.character, bindings);
});

// ==========================================
// GO-TO-DEFINITION
// ==========================================
connection.onDefinition((params: DefinitionParams): Location | Location[] | null => {
    const uri = params.textDocument.uri;
    const doc = documents.get(uri);
    if (!doc) return null;

    const source = doc.getText();
    const lines = source.split("\n");
    const line = lines[params.position.line] ?? "";
    const word = extractWordAround(doc, params.position.line, params.position.character);

    // Check local symbols in cached AST
    const parsed = store.getOrParse(uri, source);
    const sym = parsed.symbols.find(s => s.name === word);
    if (sym && sym.location) {
        return {
            uri,
            range: {
                start: { line: (sym.location.line ?? 1) - 1, character: (sym.location.column ?? 1) - 1 },
                end: { line: (sym.location.endLine ?? 1) - 1, character: (sym.location.endColumn ?? 1) - 1 },
            },
        };
    }

    // Search workspace documents for public symbols
    for (const wsDoc of store.getAll()) {
        const wsParsed = parseSource(wsDoc.source);
        const wsSym = wsParsed.symbols.find(s => s.name === word && s.access === "public");
        if (wsSym && wsSym.location) {
            return {
                uri: wsDoc.uri,
                range: {
                    start: { line: (wsSym.location.line ?? 1) - 1, character: (wsSym.location.column ?? 1) - 1 },
                    end: { line: (wsSym.location.endLine ?? 1) - 1, character: (wsSym.location.endColumn ?? 1) - 1 },
                },
            };
        }
    }

    // Check imports (for alias.Member pattern)
    const extended = extractExtendedExpr(line, params.position.character);
    if (extended.includes(".")) {
        const dotPos = extended.indexOf(".");
        const aliasPart = extended.substring(0, dotPos);
        const memberPart = extended.substring(dotPos + 1);
        const importDef = parsed.imports.find(i => i.alias === aliasPart);
        if (importDef && workspaceRoot) {
            const targetFile = resolveImportFile(importDef.path);
            if (targetFile && fs.existsSync(targetFile)) {
                const targetUri = "file://" + targetFile.replace(/\\/g, "/");
                // Try to resolve member location in target file
                const targetSource = readFileSafe(targetFile);
                if (targetSource) {
                    const targetParsed = parseSource(targetSource);
                    const targetSym = targetParsed.symbols.find(s => s.name === memberPart);
                    if (targetSym && targetSym.location) {
                        return {
                            uri: targetUri,
                            range: {
                                start: { line: (targetSym.location.line ?? 1) - 1, character: (targetSym.location.column ?? 1) - 1 },
                                end: { line: (targetSym.location.endLine ?? 1) - 1, character: (targetSym.location.endColumn ?? 1) - 1 },
                            },
                        };
                    }
                }
                return { uri: targetUri, range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } } };
            }
        }
    }

    return null;
});

function resolveImportFile(importPath: string): string | null {
    if (!workspaceRoot) return null;
    for (const f of workspaceFiles) {
        if (f.toLowerCase() === importPath.toLowerCase() || f.toLowerCase().endsWith("/" + importPath.toLowerCase())) {
            return path.join(workspaceRoot, "src", f + ".wrm");
        }
    }
    return path.join(workspaceRoot, "src", importPath + ".wrm");
}

function readFileSafe(filePath: string): string | null {
    try { return fs.readFileSync(filePath, "utf-8"); } catch { return null; }
}

// ==========================================
// DOCUMENT SYMBOLS
// ==========================================
connection.onDocumentSymbol((params: DocumentSymbolParams): DocumentSymbol[] => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return [];
    const parsed = store.getOrParse(params.textDocument.uri, doc.getText());
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
                start: { line: (s.location.line ?? 1) - 1, character: (s.location.column ?? 1) - 1 },
                end: { line: (s.location.endLine ?? 1) - 1, character: (s.location.endColumn ?? 1) - 1 },
            },
            selectionRange: {
                start: { line: (s.location.line ?? 1) - 1, character: (s.location.column ?? 1) - 1 },
                end: { line: (s.location.line ?? 1) - 1, character: (s.location.column ?? 1) - 1 + s.name.length },
            },
        };
    });
});

// ==========================================
// SIGNATURE HELP
// ==========================================
connection.onSignatureHelp((params: SignatureHelpParams): SignatureHelp | null => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return null;

    const source = doc.getText();
    const lines = source.split("\n");
    const line = lines[params.position.line] ?? "";
    const col = params.position.character;

    const funcName = extractCallableBefore(line, col, source, params.position.line);
    if (!funcName) return null;

    // Roblox global functions
    const f = bindings.getFunction(funcName);
    if (f) {
        return makeSignatureHelp(f.name, f.params.map(p => [p.name, p.type]), f.returns, f.description, countCommasBefore(line, col));
    }

    // Method calls: obj:method(...)
    if (funcName.includes(":")) {
        const colonPos = funcName.lastIndexOf(":");
        const objPart = funcName.substring(0, colonPos);
        const method = funcName.substring(colonPos + 1);
        const parsed = store.getOrParse(params.textDocument.uri, source);
        const typeName = parsed.scope.variables.get(objPart);
        if (typeName && typeName !== "any") {
            const result = findMethodHelp(bindings, typeName, method, line, col);
            if (result) return result;
        }
    }

    // Dotted class method calls: Class.method(...)
    const parts = funcName.split(".");
    if (parts.length > 1) {
        const className = parts[0];
        const method = parts[1];
        const result = findMethodHelp(bindings, className, method, line, col);
        if (result) return result;
    }

    return null;
});

function findMethodHelp(bindings: Bindings, typeName: string, method: string, line: string, col: number): SignatureHelp | null {
    const methods = bindings.getAllMethods(typeName);
    const m = methods.find(m => m.name.toLowerCase() === method.toLowerCase());
    if (!m) return null;
    return makeSignatureHelp(`${typeName}.${m.name}`, m.params.map(p => [p.name, p.type]), m.returns, m.description, countCommasBefore(line, col));
}

function makeSignatureHelp(label: string, params: [string, string][], returns: string, description: string, activeParam: number): SignatureHelp {
    const paramLabels = params.map(p => ({ label: `${p[0]}: ${p[1]}` }));
    return {
        signatures: [{
            label: `${label}(${params.map(p => `${p[0]}: ${p[1]}`).join(", ")}): ${returns}`,
            parameters: paramLabels,
            documentation: description ? { kind: "markdown" as const, value: description } : undefined,
        }],
        activeSignature: 0,
        activeParameter: activeParam,
    };
}

// ==========================================
// CODE ACTIONS
// ==========================================
connection.onCodeAction((params: CodeActionParams): CodeAction[] => {
    const doc = documents.get(params.textDocument.uri);
    if (!doc) return [];
    const diagnostics = params.context.diagnostics;
    const actions: CodeAction[] = [];

    for (const diag of diagnostics) {
        const line = (doc.getText().split("\n")[diag.range.start.line] ?? "");
        const col = diag.range.start.character;
        const msg = diag.message.toLowerCase();

        // .length -> #
        if (diag.message.includes(".length")) {
            const word = extractWordAround(doc, diag.range.start.line, col);
            const varName = word.replace(/\.length$/, "");
            actions.push({
                title: `Replace '.length' with '#${varName}'`,
                kind: CodeActionKind.QuickFix,
                diagnostics: [diag],
                edit: { changes: { [params.textDocument.uri]: [{ range: diag.range, newText: `#${varName}` }] } },
            });
            continue;
        }

        // len() -> #
        if (diag.message.includes("len(")) {
            const endParen = line.indexOf(")", col);
            if (endParen > col) {
                const arg = line.substring(col + 4, endParen).trim();
                actions.push({
                    title: `Replace 'len(${arg})' with '#${arg}'`,
                    kind: CodeActionKind.QuickFix,
                    diagnostics: [diag],
                    edit: { changes: { [params.textDocument.uri]: [{ range: { start: diag.range.start, end: { line: diag.range.start.line, character: endParen + 1 } }, newText: `#${arg}` }] } },
                });
            }
            continue;
        }

        // Undeclared variable
        if (msg.includes("undeclared variable") || msg.includes("undefined variable")) {
            const word = extractWordAround(doc, diag.range.start.line, col);
            if (word) {
                actions.push({
                    title: `Add 'local ${word}' declaration`,
                    kind: CodeActionKind.QuickFix,
                    diagnostics: [diag],
                    edit: { changes: { [params.textDocument.uri]: [{ range: { start: diag.range.start, end: diag.range.start }, newText: `local ${word} = ` }] } },
                });
            }
            continue;
        }

        // Deprecated API: wait()/spawn()/delay() -> task.*
        if (msg.includes("deprecated") && (msg.includes("wait") || msg.includes("spawn") || msg.includes("delay"))) {
            const replacement = msg.includes("task.wait") ? "task.wait" : msg.includes("task.spawn") ? "task.spawn" : msg.includes("task.delay") ? "task.delay" : null;
            if (replacement) {
                const oldFunc = msg.includes("wait(") ? "wait" : msg.includes("spawn(") ? "spawn" : msg.includes("delay(") ? "delay" : null;
                if (oldFunc) {
                    actions.push({
                        title: `Replace '${oldFunc}' with '${replacement}'`,
                        kind: CodeActionKind.QuickFix,
                        diagnostics: [diag],
                        edit: { changes: { [params.textDocument.uri]: [{ range: diag.range, newText: replacement }] } },
                    });
                }
            }
            continue;
        }

        // Server-only service in client
        if (diag.message.includes("server-only") || msg.includes("server only")) {
            actions.push({
                title: "Add RemoteFunction pattern comment",
                kind: CodeActionKind.QuickFix,
                diagnostics: [diag],
                edit: { changes: { [params.textDocument.uri]: [{ range: { start: { line: diag.range.start.line, character: 0 }, end: { line: diag.range.start.line, character: 0 } }, newText: "// TODO: Access server-only services via RemoteEvents/RemoteFunctions\n" }] } },
            });
            continue;
        }

        // ModuleScript missing return
        if (msg.includes("should return") || msg.includes("missing return")) {
            actions.push({
                title: "Add return statement",
                kind: CodeActionKind.QuickFix,
                diagnostics: [diag],
                edit: { changes: { [params.textDocument.uri]: [{ range: { start: { line: (doc.getText().split("\n").length) as number, character: 0 }, end: { line: (doc.getText().split("\n").length) as number, character: 0 } }, newText: "\nreturn {}" }] } },
            });
            continue;
        }

        // Class missing init constructor
        if (msg.includes("init") || msg.includes("constructor")) {
            actions.push({
                title: "Add init() constructor",
                kind: CodeActionKind.QuickFix,
                diagnostics: [diag],
                edit: { changes: { [params.textDocument.uri]: [{ range: { start: { line: diag.range.start.line + 1, character: 0 }, end: { line: diag.range.start.line + 1, character: 0 } }, newText: "    public function init(self)\n        -- Initialize instance here\n    end\n\n" }] } },
            });
            continue;
        }
    }

    return actions;
});

// ==========================================
// RENAME
// ==========================================
connection.onPrepareRename((params) => {
    return handlePrepareRename(store, params.textDocument.uri, params.position.line, params.position.character);
});

connection.onRenameRequest((params: RenameParams): WorkspaceEdit | null => {
    return handleRename(store, params.textDocument.uri, params.position.line, params.position.character, params.newName);
});

// ==========================================
// SEMANTIC TOKENS
// ==========================================
connection.languages.semanticTokens.on((params: SemanticTokensParams): any => {
    const uri = params.textDocument.uri;
    const source = documents.get(uri)?.getText() ?? "";
    const parsed = store.getOrParse(uri, source);
    const data = computeSemanticTokens(parsed.ast, source);
    return { data };
});

connection.languages.semanticTokens.onRange((params: SemanticTokensRangeParams): any => {
    const uri = params.textDocument.uri;
    const source = documents.get(uri)?.getText() ?? "";
    const parsed = store.getOrParse(uri, source);
    const data = computeSemanticTokens(parsed.ast, source);
    return { data };
});

// ==========================================
// INLAY HINTS
// ==========================================
connection.languages.inlayHint.on((params: any): any => {
    return computeInlayHints(store, params.textDocument.uri);
});

// ==========================================
// WORKSPACE SYMBOLS
// ==========================================
connection.onWorkspaceSymbol((params: any): any => {
    return computeWorkspaceSymbols(store, params.query);
});

// ==========================================
// HELPERS
// ==========================================
function extractExtendedExpr(line: string, col: number): string {
    const start = line.substring(0, col).search(/[^\w.]+$/);
    const startIdx = start === -1 ? 0 : start + 1;
    const end = line.substring(col).search(/[^\w.]+/);
    const endIdx = end === -1 ? line.length : col + end;
    return line.substring(startIdx, endIdx);
}

function extractCallableBefore(line: string, col: number, source: string, lineNum: number): string | null {
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

// ==========================================
// START
// ==========================================
documents.listen(connection);
connection.listen();
