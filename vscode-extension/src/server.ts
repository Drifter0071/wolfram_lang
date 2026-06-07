import {
  createConnection,
  TextDocuments,
  ProposedFeatures,
  InitializeParams,
  DidChangeConfigurationNotification,
  CompletionItem,
  CompletionItemKind,
  TextDocumentSyncKind,
  InitializeResult,
  Hover,
  MarkupKind,
  Diagnostic,
  DiagnosticSeverity,
  DocumentSymbol,
  SymbolKind,
} from "vscode-languageserver/node";

import { TextDocument } from "vscode-languageserver-textdocument";
import { execFile } from "child_process";
import * as path from "path";
import * as fs from "fs";
import {
  loadBuiltinBindings,
  loadWorkspaceBindings,
  getGlobals,
  getFunctions,
  getServices,
  getEnums,
  getMemberAccess,
  resolveType,
  getExpressionBeforeDot,
  resolveExpressionType,
} from "./bindings";

const connection = createConnection(ProposedFeatures.all);
const documents: TextDocuments<TextDocument> = new TextDocuments(TextDocument);

let hasConfigurationCapability = false;
let workspaceRoot = "";

const KEYWORDS = [
  "if", "else", "while", "for", "in", "function", "local", "return",
  "class", "struct", "enum", "import", "as", "public", "private",
  "true", "false", "nil", "self", "break", "continue", "and", "or", "not",
];

const KEYWORD_SNIPPETS: Record<string, string> = {
  if: "if (${1:condition}) {\n\t${0}\n}",
  while: "while (${1:condition}) {\n\t${0}\n}",
  for: "for ${1:x} in ${2:items} {\n\t${0}\n}",
  function: "function ${1:name}(${2:params}) {\n\t${0}\n}",
  local: "local ${1:name} = ",
  class: "class ${1:Name} {\n\t${0}\n}",
  struct: "struct ${1:Name} {\n\t${0}\n}",
  enum: "enum ${1:Name} {\n\t${0}\n}",
  import: 'import "${1:path}" as ${2:alias}',
  return: "return ${1:value}",
};

interface AnalyzeSymbol {
  name: string;
  kind: string;
  access: string;
  location: {
    line: number;
    column: number;
    end_line: number;
    end_column: number;
  };
  params: string[];
  fields: string[];
}

interface AnalyzeImport {
  path: string;
  alias: string;
}

interface AnalyzeDiagnostic {
  line: number;
  column: number;
  message: string;
  severity: string;
}

interface AnalyzeResult {
  ok: boolean;
  diagnostics: AnalyzeDiagnostic[];
  symbols: AnalyzeSymbol[];
  imports: AnalyzeImport[];
}

let symbolCache: Map<string, AnalyzeResult> = new Map();
let diagnosticTimers: Map<string, NodeJS.Timeout> = new Map();

let wolPathCache: string[] = [];
let wolPathCacheTime = 0;

function collectWolPaths(root: string): string[] {
  const results: string[] = [];
  try {
    const stack = [root];
    while (stack.length > 0) {
      const dir = stack.pop()!;
      const entries = fs.readdirSync(dir, { withFileTypes: true });
      for (const entry of entries) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) {
          if (entry.name !== "node_modules" && entry.name !== ".git" && entry.name !== "target" && entry.name !== "out") {
            stack.push(full);
          }
        } else if (entry.name.endsWith(".wol")) {
          results.push(full);
        }
      }
    }
  } catch {
    // ignore
  }
  return results;
}

function getCachedWolPaths(sourceDir: string): string[] {
  const now = Date.now();
  if (now - wolPathCacheTime > 5000) {
    wolPathCache = collectWolPaths(sourceDir);
    wolPathCacheTime = now;
  }
  return wolPathCache;
}

function isInsideImportString(linePrefix: string): { isImport: boolean; partialPath: string } {
  const trimmed = linePrefix.trimStart();
  // Match: import "partial/path
  const match = trimmed.match(/^import\s+["'](.*)$/);
  if (match) {
    return { isImport: true, partialPath: match[1] };
  }
  return { isImport: false, partialPath: "" };
}

connection.onInitialize((params: InitializeParams) => {
  if (params.workspaceFolders && params.workspaceFolders.length > 0) {
    workspaceRoot = params.workspaceFolders[0].uri;
    const wsPath = fs.existsSync(workspaceRoot) ? workspaceRoot : params.workspaceFolders[0].uri.replace(/^file:\/\//, "");
    workspaceRoot = wsPath;
  }

  const capabilities = params.capabilities;

  hasConfigurationCapability = !!(
    capabilities.workspace && !!capabilities.workspace.configuration
  );

  const result: InitializeResult = {
    capabilities: {
      textDocumentSync: TextDocumentSyncKind.Incremental,
      completionProvider: {
        resolveProvider: false,
        triggerCharacters: [".", ":", "("],
      },
      hoverProvider: true,
      documentSymbolProvider: true,
      definitionProvider: true,
    },
  };

  return result;
});

connection.onInitialized(() => {
  if (hasConfigurationCapability) {
    connection.client.register(
      DidChangeConfigurationNotification.type,
      undefined
    );
  }

  try {
    const extPath = path.join(__dirname, "..");
    loadBuiltinBindings(extPath);
    if (workspaceRoot) {
      loadWorkspaceBindings(workspaceRoot);
    }
    connection.console.log("Wolfram bindings loaded");
  } catch (e) {
    connection.console.error(`Failed to load bindings: ${e}`);
  }
});

async function runAnalyze(filePath: string, source: string): Promise<AnalyzeResult> {
  return new Promise((resolve) => {
    const compilerPath = "wolfram";

    const child = execFile(
      compilerPath,
      ["--analyze", filePath],
      { maxBuffer: 10 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (error && !stdout) {
          resolve({
            ok: false,
            diagnostics: [
              {
                line: 1,
                column: 1,
                message: stderr || error.message,
                severity: "error",
              },
            ],
            symbols: [],
            imports: [],
          });
          return;
        }
        try {
          const result = JSON.parse(stdout) as AnalyzeResult;
          resolve(result);
        } catch {
          resolve({
            ok: false,
            diagnostics: [],
            symbols: [],
            imports: [],
          });
        }
      }
    );

    child.stdin?.write(source);
    child.stdin?.end();

    setTimeout(() => {
      child.kill();
      resolve({
        ok: false,
        diagnostics: [
          {
            line: 1,
            column: 1,
            message: "Analysis timed out",
            severity: "error",
          },
        ],
        symbols: [],
        imports: [],
      });
    }, 5000);
  });
}

async function analyzeDocument(
  textDocument: TextDocument
): Promise<AnalyzeResult> {
  const filePath = textDocument.uri;
  const source = textDocument.getText();

  const cached = symbolCache.get(filePath);
  if (cached) return cached;

  const result = await runAnalyze(filePath, source);
  symbolCache.set(filePath, result);
  return result;
}

documents.onDidChangeContent((change) => {
  const uri = change.document.uri;
  symbolCache.delete(uri);

  const existingTimer = diagnosticTimers.get(uri);
  if (existingTimer) {
    clearTimeout(existingTimer);
  }

  diagnosticTimers.set(
    uri,
    setTimeout(async () => {
      diagnosticTimers.delete(uri);
      const result = await runAnalyze(uri, change.document.getText());
      symbolCache.set(uri, result);

      const diagnostics: Diagnostic[] = result.diagnostics.map((d) => ({
        severity:
          d.severity === "error"
            ? DiagnosticSeverity.Error
            : DiagnosticSeverity.Warning,
        range: {
          start: {
            line: Math.max(0, d.line - 1),
            character: Math.max(0, d.column - 1),
          },
          end: {
            line: Math.max(0, d.line - 1),
            character: Math.max(0, d.column + 20),
          },
        },
        message: d.message,
        source: "wolfram",
      }));

      connection.sendDiagnostics({ uri, diagnostics });
    }, 500)
  );
});

connection.onCompletion(
  async (params): Promise<CompletionItem[]> => {
    const document = documents.get(params.textDocument.uri);
    if (!document) return [];

    const text = document.getText();
    const offset = document.offsetAt(params.position);
    const linePrefix = document
      .getText({
        start: { line: params.position.line, character: 0 },
        end: params.position,
      })
      .toLowerCase();

    const items: CompletionItem[] = [];

    // Check if this is dot/colon completion (member access)
    const beforeDot = getExpressionBeforeDot(text, offset);
    const isColon = offset > 0 && text[offset - 1] === ":";

    if (beforeDot) {
      const docSource = document.getText();
      const exprType = resolveExpressionType(beforeDot, docSource);

      if (exprType) {
        const memberAccess = getMemberAccess(exprType);
        if (memberAccess) {
          if (isColon) {
            // Method completion with :
            for (const method of memberAccess.methods) {
              const params = method.params.map((p) => `\${${p.name}}`).join(", ");
              items.push({
                label: method.name,
                kind: CompletionItemKind.Method,
                detail: `(${method.params.map((p) => `${p.name}: ${p.type}`).join(", ")}): ${method.returns}`,
                insertText: `${method.name}(${params})${method.params.length === 0 ? ")" : ""}`,
                insertTextFormat: method.params.length > 0 ? 2 : 1,
                documentation: method.description,
              });
            }
          } else {
            // Property completion with .
            for (const prop of memberAccess.properties) {
              items.push({
                label: prop.name,
                kind: CompletionItemKind.Property,
                detail: `${prop.name}: ${prop.type}${prop.rw ? " (read/write)" : " (read-only)"}`,
                documentation: prop.description,
              });
            }
            // Also show methods for .
            for (const method of memberAccess.methods) {
              const params = method.params.map((p) => `\${${p.name}}`).join(", ");
              items.push({
                label: method.name,
                kind: CompletionItemKind.Method,
                detail: `(${method.params.map((p) => `${p.name}: ${p.type}`).join(", ")}): ${method.returns}`,
                insertText: `${method.name}(${params})${method.params.length === 0 ? ")" : ""}`,
                insertTextFormat: method.params.length > 0 ? 2 : 1,
                documentation: method.description,
              });
            }
          }

          if (items.length > 0) return items;
        }
      }

      // Fallback: show all methods + properties for the expression
      if (exprType && items.length === 0) {
        items.push({
          label: `-- No bindings for type '${exprType}'`,
          kind: CompletionItemKind.Text,
        });
      }
    }

    // Import path completion
    const { isImport, partialPath } = isInsideImportString(linePrefix);
    if (isImport) {
      const docUri = document.uri;
      const docDir = path.dirname(fs.existsSync(docUri) ? docUri : docUri.replace(/^file:\/\//, ""));
      const srcDir = workspaceRoot && fs.existsSync(workspaceRoot)
        ? workspaceRoot
        : docDir;

      const allPaths = getCachedWolPaths(srcDir);
      const partialLower = partialPath.toLowerCase();

      for (const fullPath of allPaths) {
        let relPath = path.relative(docDir, fullPath).replace(/\\/g, "/");
        // Strip .wol extension for completions
        const relNoExt = relPath.replace(/\.wol$/, "");

        if (relNoExt.toLowerCase().startsWith(partialLower) || partialPath === "") {
          items.push({
            label: relNoExt,
            kind: CompletionItemKind.File,
            detail: relPath,
            filterText: relNoExt,
          });
        }
      }

      // Show "." and ".." directory completions
      if (partialPath === "" || partialPath.endsWith("/")) {
        const scanDir = partialPath
          ? path.join(docDir, partialPath.replace(/\/$/, ""))
          : docDir;
        try {
          const entries = fs.readdirSync(scanDir, { withFileTypes: true });
          for (const entry of entries) {
            if (entry.isDirectory() && !entry.name.startsWith(".")) {
              items.push({
                label: `${partialPath}${entry.name}/`,
                kind: CompletionItemKind.Folder,
                detail: "Directory",
                filterText: `${partialPath}${entry.name}`,
              });
            }
          }
        } catch {
          // ignore
        }
      }

      return items;
    }

    // Keyword completions
    for (const kw of KEYWORDS) {
      if (kw.startsWith(linePrefix.trimStart())) {
        const item: CompletionItem = {
          label: kw,
          kind: CompletionItemKind.Keyword,
          detail: "Keyword",
        };
        if (KEYWORD_SNIPPETS[kw]) {
          item.insertText = KEYWORD_SNIPPETS[kw];
          item.insertTextFormat = 2;
        }
        items.push(item);
      }
    }

    // Roblox globals
    for (const global of getGlobals()) {
      if (global.name.toLowerCase().startsWith(linePrefix.trimStart())) {
        items.push({
          label: global.name,
          kind: CompletionItemKind.Value,
          detail: `${global.type} — ${global.description}`,
        });
      }
    }

    // Roblox global functions
    for (const func of getFunctions()) {
      if (func.name.toLowerCase().startsWith(linePrefix.trimStart())) {
        const params = func.params.map((p) => `\${${p.name}}`).join(", ");
        items.push({
          label: func.name,
          kind: CompletionItemKind.Function,
          detail: `(${func.params.map((p) => `${p.name}: ${p.type}`).join(", ")}): ${func.returns}`,
          insertText: func.params.length > 0 ? `${func.name}(${params})` : func.name,
          insertTextFormat: func.params.length > 0 ? 2 : 1,
          documentation: func.description,
        });
      }
    }

    // Roblox enums
    for (const en of getEnums()) {
      if (en.name.toLowerCase().startsWith(linePrefix.trimStart())) {
        items.push({
          label: `Enum.${en.name}`,
          kind: CompletionItemKind.Enum,
          detail: en.items.slice(0, 5).join(", ") + (en.items.length > 5 ? "..." : ""),
          documentation: en.description,
        });
      }
    }

    // Enum value completions (after typing "Enum.")
    const enumPrefix = linePrefix.trimStart();
    if (enumPrefix.toLowerCase().startsWith("enum.")) {
      const afterDot = enumPrefix.substring(5);
      for (const en of getEnums()) {
        for (const item of en.items) {
          const fullName = `Enum.${en.name}.${item}`;
          if (fullName.toLowerCase().startsWith(enumPrefix)) {
            items.push({
              label: fullName,
              kind: CompletionItemKind.EnumMember,
              detail: `${en.name}`,
            });
          }
        }
      }
    }

    try {
      const result = await analyzeDocument(document);

      for (const symbol of result.symbols) {
        if (
          symbol.name
            .toLowerCase()
            .startsWith(linePrefix.trimStart())
        ) {
          let kind: CompletionItemKind = CompletionItemKind.Variable;
          switch (symbol.kind) {
            case "function":
              kind = CompletionItemKind.Function;
              break;
            case "class":
              kind = CompletionItemKind.Class;
              break;
            case "struct":
              kind = CompletionItemKind.Struct;
              break;
            case "enum":
              kind = CompletionItemKind.Enum;
              break;
          }

          items.push({
            label: symbol.name,
            kind,
            detail: `${symbol.access} ${symbol.kind}${
              symbol.params.length > 0
                ? `(${symbol.params.join(", ")})`
                : ""
            }`,
          });
        }
      }

      for (const imp of result.imports) {
        items.push({
          label: imp.alias,
          kind: CompletionItemKind.Module,
          detail: `import "${imp.path}"`,
        });
      }
    } catch {
      // ignore analysis errors in completion
    }

    return items;
  }
);

connection.onHover(
  async (params): Promise<Hover | null> => {
    const document = documents.get(params.textDocument.uri);
    if (!document) return null;

    const offset = document.offsetAt(params.position);
    const source = document.getText();
    const start = source.lastIndexOf(" ", offset) + 1;
    const end = source.indexOf(" ", offset);
    const word = source.substring(
      start,
      end === -1 ? source.length : end
    );

    if (!word.match(/^[a-zA-Z_][a-zA-Z0-9_]*$/)) return null;

    // Check Roblox globals
    for (const global of getGlobals()) {
      if (global.name === word) {
        return {
          contents: {
            kind: MarkupKind.Markdown,
            value: `**${global.name}**\n\nType: \`${global.type}\`\n\n${global.description}`,
          },
        };
      }
    }

    // Check Roblox functions
    for (const func of getFunctions()) {
      if (func.name === word) {
        const params = func.params
          .map((p) => `\`${p.name}: ${p.type}\``)
          .join(", ");
        return {
          contents: {
            kind: MarkupKind.Markdown,
            value: `**${func.name}(${params})** → \`${func.returns}\`\n\n${func.description}`,
          },
        };
      }
    }

    // Check service globals
    for (const svc of getServices()) {
      if (svc.name === word) {
        return {
          contents: {
            kind: MarkupKind.Markdown,
            value: `**${svc.name}** — Service\n\nType: \`${svc.className}\`\n\n${svc.description}\n\nAccess via: \`game:GetService("${svc.name}")\``,
          },
        };
      }
    }

    // Check source symbols (from compiler)
    try {
      const result = await analyzeDocument(document);

      for (const symbol of result.symbols) {
        if (symbol.name === word) {
          let content = `**${symbol.access} ${symbol.kind}** \`${symbol.name}\``;
          if (symbol.params.length > 0) {
            content += `\n\nParameters: \`${symbol.params.join(", ")}\``;
          }
          if (symbol.fields.length > 0) {
            const fieldList = symbol.fields.map((f) => `\`${f}\``).join(", ");
            content += `\n\n${symbol.kind === "enum" ? "Variants" : "Fields"}: ${fieldList}`;
          }
          content += `\n\n*Line ${symbol.location.line}, Column ${symbol.location.column}*`;

          return {
            contents: {
              kind: MarkupKind.Markdown,
              value: content,
            },
          };
        }
      }
    } catch {
      // ignore
    }

    return null;
  }
);

connection.onDocumentSymbol(
  async (params): Promise<DocumentSymbol[]> => {
    const document = documents.get(params.textDocument.uri);
    if (!document) return [];

    try {
      const result = await analyzeDocument(document);

      return result.symbols.map((symbol): DocumentSymbol => {
        let kind: SymbolKind = SymbolKind.Variable;
        switch (symbol.kind) {
          case "function":
            kind = SymbolKind.Function;
            break;
          case "class":
            kind = SymbolKind.Class;
            break;
          case "struct":
            kind = SymbolKind.Struct;
            break;
          case "enum":
            kind = SymbolKind.Enum;
            break;
        }

        return {
          name: symbol.name,
          detail: `${symbol.access} ${symbol.kind}`,
          kind,
          range: {
            start: {
              line: Math.max(0, symbol.location.line - 1),
              character: Math.max(0, symbol.location.column - 1),
            },
            end: {
              line: Math.max(0, symbol.location.end_line - 1),
              character: Math.max(0, symbol.location.end_column - 1),
            },
          },
          selectionRange: {
            start: {
              line: Math.max(0, symbol.location.line - 1),
              character: Math.max(0, symbol.location.column - 1),
            },
            end: {
              line: Math.max(0, symbol.location.line - 1),
              character:
                Math.max(0, symbol.location.column - 1) + symbol.name.length,
            },
          },
        };
      });
    } catch {
      return [];
    }
  }
);

connection.onDefinition(
  async (params) => {
    const document = documents.get(params.textDocument.uri);
    if (!document) return null;

    const source = document.getText();
    const offset = document.offsetAt(params.position);
    const start = source.lastIndexOf(" ", offset) + 1;
    const end = source.indexOf(" ", offset);
    const word = source.substring(
      start,
      end === -1 ? source.length : end
    );

    if (!word.match(/^[a-zA-Z_][a-zA-Z0-9_]*$/)) return null;

    try {
      const result = await analyzeDocument(document);

      for (const symbol of result.symbols) {
        if (symbol.name === word) {
          return {
            uri: document.uri,
            range: {
              start: {
                line: Math.max(0, symbol.location.line - 1),
                character: Math.max(0, symbol.location.column - 1),
              },
              end: {
                line: Math.max(0, symbol.location.line - 1),
                character:
                  Math.max(0, symbol.location.column - 1) +
                  symbol.name.length,
              },
            },
          };
        }
      }
    } catch {
      // ignore
    }

    return null;
  }
);

documents.listen(connection);
connection.listen();
