import {
    CompletionItem, CompletionItemKind, InsertTextFormat,
    MarkupKind,
} from "vscode-languageserver/node";
import { TextDocument } from "vscode-languageserver-textdocument";
import { Bindings } from "./bindings";
import { parseSource } from "./parser";
import {
    getLinePrefix, extractExprBeforeDot,
    isComment, isInsideString, isValuePosition, isInsideImportString, collectProjectWrmFiles,
} from "./utils";

interface KeywordDef { label: string; snippet?: string; doc: string; }

const KEYWORDS: KeywordDef[] = [
    { label: "if", snippet: "if (${1:condition}) {\n\t${0}\n}", doc: "Conditional branch." },
    { label: "else", snippet: "else {\n\t${0}\n}", doc: "Fallback branch." },
    { label: "elif", snippet: "elif (${1:condition}) {\n\t${0}\n}", doc: "Else-if branch." },
    { label: "while", snippet: "while (${1:condition}) {\n\t${0}\n}", doc: "Loop." },
    { label: "for", snippet: "for ${1:x} in ${2:items} {\n\t${0}\n}", doc: "Iterate array/table." },
    { label: "function", snippet: "function ${1:name}(${2:params}) {\n\t${0}\n}", doc: "Define function." },
    { label: "class", snippet: "class ${1:Name} {\n\t${0}\n}", doc: "Define class." },
    { label: "struct", snippet: "struct ${1:Name} {\n\t${0}\n}", doc: "Define struct." },
    { label: "enum", snippet: "enum ${1:Name} {\n\t${0}\n}", doc: "Define enum." },
    { label: "import", snippet: 'import "${1:path}" as ${2:alias}', doc: "Import module." },
    { label: "local", snippet: "local ${1:name} = ${0}", doc: "Declare variable." },
    { label: "return", snippet: "return ${1:value}", doc: "Return value." },
    { label: "true", doc: "Boolean true." },
    { label: "false", doc: "Boolean false." },
    { label: "nil", doc: "Absence of value." },
    { label: "self", doc: "Current instance." },
    { label: "break", doc: "Exit loop." },
    { label: "continue", doc: "Skip iteration." },
    { label: "public", doc: "Public access." },
    { label: "private", doc: "Private access." },
    { label: "try", snippet: "try {\n\t${0}\n} catch {\n\n}", doc: "Try-catch block." },
];

enum Ctx { IMPORT_PATH, DEFINITION_NAME, DOT_COLON, ENUM_VALUE, VALUE_EXPRESSION, COMMENT, STRING, STATEMENT_START, EXPRESSION }

function detectContext(linePrefix: string): Ctx {
    const trimmed = linePrefix.trimStart();
    if (!trimmed) return Ctx.STATEMENT_START;
    if (/^import\s+["'][^"']*$/.test(trimmed)) return Ctx.IMPORT_PATH;
    if (isComment(linePrefix)) return Ctx.COMMENT;
    if (isInsideString(linePrefix) && !trimmed.startsWith("import")) return Ctx.STRING;
    const defMatch = trimmed.match(/^(?:local\s+)?(?:public\s+|private\s+)?(function|class|struct|enum)\s+(\w*)$/);
    if (defMatch) return Ctx.STATEMENT_START;
    const lastChar = linePrefix[linePrefix.length - 1] ?? "";
    if (lastChar === "." || lastChar === ":") {
        if (/Enum\.$/.test(trimmed)) return Ctx.ENUM_VALUE;
        return Ctx.DOT_COLON;
    }
    const wordMatch = trimmed.match(/([\w.]+)$/);
    const wordPrefix = wordMatch ? wordMatch[1] : "";
    if (/^(?:Enum\.?|Enum\.\w*)$/i.test(wordPrefix)) {
        const beforeWord = trimmed.substring(0, trimmed.length - wordPrefix.length);
        if (isValuePosition(beforeWord)) return Ctx.ENUM_VALUE;
        return Ctx.EXPRESSION;
    }
    const beforeWord = trimmed.substring(0, trimmed.length - wordPrefix.length);
    if (isValuePosition(beforeWord)) return Ctx.VALUE_EXPRESSION;
    return Ctx.EXPRESSION;
}

function resolveChainedType(prefix: string, bindings: Bindings, scope: Map<string, string>): string | undefined {
    if (!prefix) return undefined;
    const parts = prefix.split(".");
    const root = parts[0];

    const g = bindings.getGlobal(root);
    const rootType = g ? g.type : (scope.get(root) ?? undefined);
    if (!rootType) {
        // Check if root itself is a known type (e.g. Players, ReplicatedStorage, etc.)
        if (bindings.getType(root)) {
            let current = root;
            for (let i = 1; i < parts.length; i++) {
                const props = bindings.getAllProperties(current);
                const p = props.find(x => x.name.toLowerCase() === parts[i].toLowerCase());
                if (p) { current = p.type; continue; }
                const methods = bindings.getAllMethods(current);
                const m = methods.find(x => x.name.toLowerCase() === parts[i].toLowerCase());
                if (m) { current = m.returns; continue; }
                return current;
            }
            return current;
        }
        return undefined;
    }

    let current = rootType;
    for (let i = 1; i < parts.length; i++) {
        const seg = parts[i];
        const props = bindings.getAllProperties(current);
        const p = props.find(x => x.name.toLowerCase() === seg.toLowerCase());
        if (p) { current = p.type; continue; }
        const methods = bindings.getAllMethods(current);
        const m = methods.find(x => x.name.toLowerCase() === seg.toLowerCase());
        if (m) { current = m.returns; continue; }
        if (current === "Instance" || bindings.getType("Instance")?.extends) {
            return "Instance";
        }
        return current;
    }
    return current;
}

const VALUE_KW = KEYWORDS.filter(k => ["true", "false", "nil", "self"].includes(k.label));
const STRUCT_KW = KEYWORDS.filter(k => ["if", "else", "elif", "while", "for", "function", "class", "struct", "enum", "import", "local", "return", "break", "continue", "public", "private", "try", "catch"].includes(k.label));

function buildEnrichedScope(source: string, bindings: Bindings): Map<string, string> {
    const scope = new Map<string, string>();

    // local name = bindings.new(...)  /  local name = Type.new(...)
    const newRe = /local\s+(\w+)\s*=\s*(\w+(?:\.\w+)*)\.new\s*\(/g;
    let m: RegExpExecArray | null;
    while ((m = newRe.exec(source)) !== null) {
        const typeName = resolveChainedType(m[2], bindings, new Map());
        scope.set(m[1], typeName ?? m[2]);
    }

    // local name = expr:GetService("ServiceName")
    const svcRe = /local\s+(\w+)\s*=.*:GetService\s*\(\s*"([^"]+)"/g;
    while ((m = svcRe.exec(source)) !== null) scope.set(m[1], m[2]);

    // local name = Expr.Chain — resolve through bindings
    const chainRe = /local\s+(\w+)\s*=\s*([\w.]+)(?!\()/g;
    while ((m = chainRe.exec(source)) !== null) {
        const varName = m[1];
        const rhs = m[2];
        if (scope.has(varName)) continue;
        if (rhs.includes(".")) {
            const typeName = resolveChainedType(rhs, bindings, new Map());
            if (typeName) scope.set(varName, typeName);
            continue;
        }
        // Simple assignment: local x = y — try to look up y
        const g = bindings.getGlobal(rhs);
        if (g) { scope.set(varName, g.type); continue; }
        // Check if rhs is a known type name itself
        const t = bindings.getType(rhs);
        if (t) { scope.set(varName, rhs); continue; }
    }

    // for var in expr
    const forRe = /for\s+(\w+)\s*(?:,\s*\w+\s*)?in\s+(.+?)(?:\{|$)/g;
    while ((m = forRe.exec(source)) !== null) {
        if (!scope.has(m[1])) scope.set(m[1], "any");
    }

    return scope;
}

export function handleCompletion(
    document: TextDocument,
    line: number,
    character: number,
    bindings: Bindings,
    workspaceFiles: string[],
): CompletionItem[] {
    const linePrefix = getLinePrefix(document, line, character);
    const ctx = detectContext(linePrefix);
    const items: CompletionItem[] = [];
    const seen = new Set<string>();

    function push(item: CompletionItem): void {
        const lbl = item.label as string;
        if (!seen.has(lbl)) { seen.add(lbl); items.push(item); }
    }

    // Import path context
    if (ctx === Ctx.IMPORT_PATH) {
        const partial = (isInsideImportString(linePrefix) ?? "").toLowerCase();
        for (const f of workspaceFiles) {
            if (f.toLowerCase().startsWith(partial) || f.toLowerCase().includes(partial)) {
                push({ label: f, kind: CompletionItemKind.File, sortText: "0" + f });
            }
        }
        return items;
    }

    if (ctx === Ctx.DEFINITION_NAME || ctx === Ctx.COMMENT || ctx === Ctx.STRING) return [];

    // Dot/colon member access — uses type chaining
    if (ctx === Ctx.DOT_COLON) {
        const lastChar = linePrefix[linePrefix.length - 1] ?? "";
        const expr = extractExprBeforeDot(document, line, character);
        const scope = buildEnrichedScope(document.getText(), bindings);
        const typeName = resolveChainedType(expr, bindings, scope);
        if (typeName) {
            if (lastChar === ":") {
                for (const m of bindings.getAllMethods(typeName)) {
                    const params = m.params.map(p => `${p.name}: ${p.type}`).join(", ");
                    push({
                        label: m.name, kind: CompletionItemKind.Method,
                        detail: `(${params}): ${m.returns}`,
                        insertText: m.params.length > 0
                            ? `${m.name}(${m.params.map((p, i) => `\${${i + 1}:${p.name}}`).join(", ")})`
                            : `${m.name}()`,
                        insertTextFormat: m.params.length > 0 ? InsertTextFormat.Snippet : InsertTextFormat.PlainText,
                        sortText: "3" + m.name,
                        documentation: m.description ? { kind: MarkupKind.Markdown, value: m.description } : undefined,
                    });
                }
            } else {
                for (const p of bindings.getAllProperties(typeName)) {
                    push({
                        label: p.name, kind: CompletionItemKind.Property,
                        detail: `${p.type}${p.rw ? " (read/write)" : " (read-only)"}`,
                        sortText: "3" + p.name,
                        documentation: p.description ? { kind: MarkupKind.Markdown, value: p.description } : undefined,
                    });
                }
                for (const m of bindings.getAllMethods(typeName)) {
                    const params = m.params.map(p => `${p.name}: ${p.type}`).join(", ");
                    push({
                        label: m.name, kind: CompletionItemKind.Method,
                        detail: `(${params}): ${m.returns}`,
                        insertText: m.params.length > 0
                            ? `${m.name}(${m.params.map((p, i) => `\${${i + 1}:${p.name}}`).join(", ")})`
                            : `${m.name}()`,
                        insertTextFormat: m.params.length > 0 ? InsertTextFormat.Snippet : InsertTextFormat.PlainText,
                        sortText: "3" + m.name,
                        documentation: m.description ? { kind: MarkupKind.Markdown, value: m.description } : undefined,
                    });
                }
            }
        }
        return items;
    }

    const fullLine = linePrefix.trimStart();
    const wp = (fullLine.match(/([\w.]+)$/) ?? [""])[0].toLowerCase();
    const scope = buildEnrichedScope(document.getText(), bindings);

    // Locals
    for (const [name, type] of scope) {
        if (name.toLowerCase().startsWith(wp)) {
            push({
                label: name, kind: CompletionItemKind.Variable,
                detail: type !== "any" && type !== "local" && type !== name ? type : undefined,
                sortText: "1" + name,
            });
        }
    }

    // Keywords
    if (ctx === Ctx.STATEMENT_START) {
        for (const kw of STRUCT_KW) {
            if (kw.label.startsWith(wp)) {
                const item: CompletionItem = { label: kw.label, kind: CompletionItemKind.Keyword, sortText: "0" + kw.label };
                if (kw.snippet) { item.insertText = kw.snippet; item.insertTextFormat = InsertTextFormat.Snippet; }
                push(item);
            }
        }
    }

    if (ctx === Ctx.VALUE_EXPRESSION || ctx === Ctx.EXPRESSION) {
        for (const kw of VALUE_KW) {
            if (kw.label.startsWith(wp)) push({ label: kw.label, kind: CompletionItemKind.Keyword, sortText: "0" + kw.label });
        }
        for (const [, g] of bindings.globals) {
            if (g.name.toLowerCase().startsWith(wp)) {
                push({ label: g.name, kind: CompletionItemKind.Variable, detail: `${g.type} — ${g.description}`, sortText: "2" + g.name });
            }
        }
        for (const [, f] of bindings.functions) {
            if (f.name.toLowerCase().startsWith(wp)) {
                const params = f.params.map(p => `${p.name}: ${p.type}`).join(", ");
                push({
                    label: f.name, kind: CompletionItemKind.Function,
                    detail: `(${params}): ${f.returns}`,
                    insertText: f.params.length > 0 ? `${f.name}(${f.params.map((p, i) => `\${${i + 1}:${p.name}}`).join(", ")})` : f.name,
                    insertTextFormat: f.params.length > 0 ? InsertTextFormat.Snippet : InsertTextFormat.PlainText,
                    sortText: "2" + f.name,
                    documentation: f.description ? { kind: MarkupKind.Markdown, value: f.description } : undefined,
                });
            }
        }
    }

    // Enums
    if (ctx === Ctx.ENUM_VALUE || ctx === Ctx.VALUE_EXPRESSION || ctx === Ctx.EXPRESSION) {
        if (wp.startsWith("enum.")) {
            for (const [, en] of bindings.enums) {
                for (const itemName of en.items) {
                    const full = `Enum.${en.name}.${itemName}`;
                    if (full.toLowerCase().startsWith(wp)) {
                        push({ label: full, kind: CompletionItemKind.EnumMember, detail: en.name, sortText: "3" + full });
                    }
                }
            }
        } else {
            for (const [, en] of bindings.enums) {
                if (en.name.toLowerCase().startsWith(wp)) {
                    push({ label: `Enum.${en.name}`, kind: CompletionItemKind.Enum, detail: en.items.join(", "), sortText: "3Enum." + en.name });
                }
            }
        }
    }

    if (ctx === Ctx.EXPRESSION || ctx === Ctx.STATEMENT_START) {
        for (const kw of STRUCT_KW) {
            if (kw.label.startsWith(wp)) {
                const item: CompletionItem = {
                    label: kw.label, kind: CompletionItemKind.Keyword, sortText: "0" + kw.label,
                    insertText: kw.snippet, insertTextFormat: kw.snippet ? InsertTextFormat.Snippet : InsertTextFormat.PlainText,
                };
                push(item);
            }
        }
        for (const kw of VALUE_KW) {
            if (kw.label.startsWith(wp)) push({ label: kw.label, kind: CompletionItemKind.Keyword, sortText: "0" + kw.label });
        }
    }

    return items;
}
