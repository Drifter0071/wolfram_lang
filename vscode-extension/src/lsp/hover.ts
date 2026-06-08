import { Hover, MarkupKind } from "vscode-languageserver/node";
import { TextDocument } from "vscode-languageserver-textdocument";
import { Bindings } from "./bindings";
import { parseSource } from "./parser";
import { extractWordAround } from "./utils";

interface KeywordDef { label: string; doc: string; }

const KEYWORDS: KeywordDef[] = [
    { label: "if", doc: "**if** — Conditional branch.\n\n```wolfram\nif (x > 0) {\n    print(\"positive\")\n}\n```" },
    { label: "else", doc: "**else** — Fallback branch for `if` / `elif` chains." },
    { label: "elif", doc: "**elif** — Else-if branch. Tested when previous `if` / `elif` conditions were falsy." },
    { label: "while", doc: "**while** — Loop that repeats as long as the condition is truthy." },
    { label: "for", doc: "**for** — Iterates over an array or table. Uses `ipairs` for arrays." },
    { label: "function", doc: "**function** — Defines a named function." },
    { label: "class", doc: "**class** — Defines a class with constructor, methods, and inheritance support." },
    { label: "struct", doc: "**struct** — Defines an immutable data structure with named fields." },
    { label: "enum", doc: "**enum** — Defines a fixed set of named values." },
    { label: "import", doc: "**import** — Imports another module by relative path." },
    { label: "local", doc: "**local** — Declares a local variable scoped to the current block." },
    { label: "return", doc: "**return** — Returns a value from a function." },
    { label: "true", doc: "**true** — Boolean truth value. Compiles to `true` in Luau." },
    { label: "false", doc: "**false** — Boolean false value. Compiles to `false` in Luau." },
    { label: "nil", doc: "**nil** — Represents the absence of a value. Equivalent to `nil` in Luau." },
    { label: "self", doc: "**self** — Refers to the current instance inside a method." },
    { label: "break", doc: "**break** — Exits the innermost loop immediately." },
    { label: "continue", doc: "**continue** — Skips the rest of the current loop iteration." },
    { label: "public", doc: "**public** — Access modifier. Public members are accessible externally." },
    { label: "private", doc: "**private** — Access modifier. Private members are module-scoped." },
    { label: "try", doc: "**try** — Begins a try-catch block for error handling." },
];

const kwDocs = new Map<string, string>();
for (const kw of KEYWORDS) kwDocs.set(kw.label, kw.doc);

function resolveExprType(expr: string, bindings: Bindings, scope: Map<string, string>): string | undefined {
    if (!expr) return undefined;
    const parts = expr.split(".");
    const root = parts[0];
    const g = bindings.getGlobal(root);
    const rootType: string | undefined = g ? g.type : (scope.get(root) ?? undefined);
    if (!rootType) {
        if (bindings.getType(root)) {
            let current = root;
            for (let i = 1; i < parts.length; i++) {
                const props = bindings.getAllProperties(current);
                const p = props.find(x => x.name.toLowerCase() === parts[i].toLowerCase());
                if (p) { current = p.type; continue; }
                const methods = bindings.getAllMethods(current);
                const m = methods.find(x => x.name.toLowerCase() === parts[i].toLowerCase());
                if (m) { current = m.returns; continue; }
                return undefined;
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
        return current;
    }
    return current;
}

export function handleHover(
    document: TextDocument,
    line: number,
    character: number,
    bindings: Bindings,
): Hover | null {
    const word = extractWordAround(document, line, character);
    const lower = word.toLowerCase();

    // Keywords
    const kwDoc = kwDocs.get(lower);
    if (kwDoc) return { contents: { kind: MarkupKind.Markdown, value: kwDoc } };

    // Roblox globals
    const g = bindings.getGlobal(word);
    if (g) {
        const typeInfo = bindings.getType(g.type);
        let value = `**${g.name}** — \`${g.type}\`\n\n${g.description}`;
        if (typeInfo) {
            const propCount = typeInfo.properties.length;
            const methodCount = typeInfo.methods.length;
            value += `\n\n*${propCount} properties, ${methodCount} methods*`;
        }
        return { contents: { kind: MarkupKind.Markdown, value } };
    }

    // Roblox functions
    const f = bindings.getFunction(word);
    if (f) {
        const params = f.params.map(p => `\`${p.name}: ${p.type}\``).join(", ");
        const value = `**${f.name}(${params})** → \`${f.returns}\`\n\n${f.description}`;
        return { contents: { kind: MarkupKind.Markdown, value } };
    }

    // Roblox types — show full type info
    const t = bindings.getType(word);
    if (t) {
        let value = `**${t.name}**`;
        if (t.extends) value += ` extends \`${t.extends}\``;
        if (t.description) value += `\n\n${t.description}`;
        if (t.properties.length > 0) {
            value += `\n\n### Properties\n`;
            for (const p of t.properties.slice(0, 15)) {
                value += `- \`${p.name}: ${p.type}\`${p.rw ? " (R/W)" : " (read-only)"}${p.description ? " — " + p.description : ""}\n`;
            }
            if (t.properties.length > 15) value += `\n*...and ${t.properties.length - 15} more*\n`;
        }
        if (t.methods.length > 0) {
            value += `\n### Methods\n`;
            for (const m of t.methods.slice(0, 10)) {
                const params = m.params.map(p => `\`${p.name}: ${p.type}\``).join(", ");
                value += `- \`${m.name}(${params})${m.returns ? " → " + m.returns : ""}\`${m.description ? " — " + m.description : ""}\n`;
            }
            if (t.methods.length > 10) value += `\n*...and ${t.methods.length - 10} more*\n`;
        }
        return { contents: { kind: MarkupKind.Markdown, value } };
    }

    // Roblox enums
    for (const [, en] of bindings.enums) {
        if (en.name.toLowerCase() === lower) {
            return { contents: { kind: MarkupKind.Markdown, value: `**Enum.${en.name}**\n\nItems: \`${en.items.slice(0, 20).join(", ")}\`${en.items.length > 20 ? " ..." : ""}\n\n${en.description}` } };
        }
    }

    // Try chained expression hover (e.g., hover over "Character" in "player.Character")
    const source = document.getText();
    const lines = source.split("\n");
    const currentLine = lines[line] ?? "";
    // Extract full expression before cursor
    const beforeCursor = currentLine.substring(0, character);
    const exprMatch = beforeCursor.match(/([\w.]+)$/);
    if (exprMatch) {
        const expr = exprMatch[1];
        if (expr.includes(".")) {
            const parts = expr.split(".");
            const lastPart = parts[parts.length - 1];
            const parentExpr = parts.slice(0, -1).join(".");
            const parsed = parseSource(source);
            const parentType = resolveExprType(parentExpr, bindings, parsed.scope);
            if (parentType) {
                // Try property hover
                const props = bindings.getAllProperties(parentType);
                const prop = props.find(p => p.name.toLowerCase() === lastPart.toLowerCase());
                if (prop) {
                    return { contents: { kind: MarkupKind.Markdown, value: `**${parentType}.${prop.name}** — \`${prop.type}\`${prop.rw ? " (read/write)" : " (read-only)"}\n\n${prop.description}` } };
                }
                // Try method hover
                const methods = bindings.getAllMethods(parentType);
                const method = methods.find(m => m.name.toLowerCase() === lastPart.toLowerCase());
                if (method) {
                    const params = method.params.map(p => `\`${p.name}: ${p.type}\``).join(", ");
                    return { contents: { kind: MarkupKind.Markdown, value: `**${parentType}:${method.name}(${params})** → \`${method.returns}\`\n\n${method.description}` } };
                }
            }
        }
    }

    // Local scope symbols
    const parsed = parseSource(source);
    const localType = parsed.scope.get(word);
    if (localType) {
        const sym = parsed.symbols.find(s => s.name === word);
        if (sym) {
            let value = `**${sym.access} ${sym.kind} ${sym.name}**`;
            if (sym.params.length > 0) value += `\n\nParameters: \`${sym.params.join(", ")}\``;
            value += `\n\n*Line ${sym.location.line}, Column ${sym.location.column}*`;
            return { contents: { kind: MarkupKind.Markdown, value } };
        }
        return { contents: { kind: MarkupKind.Markdown, value: `**local ${word}**${localType !== "any" && localType !== "local" ? `: ${localType}` : ""}` } };
    }

    return null;
}
