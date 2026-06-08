import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";

// ── Wold JSON types ──────────────────────────────────────

interface WoldGlobal { name: string; type: string; description: string; }
interface WoldFunction { name: string; params: WoldParam[]; returns: string; description: string; }
interface WoldParam { name: string; type: string; }
interface WoldProperty { name: string; type: string; rw: boolean; description: string; }
interface WoldMethod { name: string; params: WoldParam[]; returns: string; description: string; }
interface WoldType { name: string; description: string; extends?: string | null; tags: string[]; properties: WoldProperty[]; methods: WoldMethod[]; events: any[]; }
interface WoldEnum { name: string; items: string[]; description: string; }
interface WoldFile { version: number; globals: WoldGlobal[]; functions: WoldFunction[]; types: WoldType[]; enums: WoldEnum[]; services: any[]; }

// ── Bindings class ───────────────────────────────────────

class WoldBindings {
  globals = new Map<string, WoldGlobal>();
  functions = new Map<string, WoldFunction>();
  types = new Map<string, WoldType>();
  enums = new Map<string, WoldEnum>();

  getType(name: string): WoldType | undefined { return this.types.get(name.toLowerCase()); }
  getGlobal(name: string): WoldGlobal | undefined { return this.globals.get(name.toLowerCase()); }

  getAllMethods(typeName: string): WoldMethod[] {
    const methods: WoldMethod[] = [];
    const t = this.getType(typeName);
    if (!t) return methods;
    methods.push(...t.methods);
    if (t.extends) methods.push(...this.getAllMethods(t.extends));
    return methods;
  }

  getAllProperties(typeName: string): WoldProperty[] {
    const props: WoldProperty[] = [];
    const t = this.getType(typeName);
    if (!t) return props;
    props.push(...t.properties);
    if (t.extends) props.push(...this.getAllProperties(t.extends));
    return props;
  }

  getMethodReturn(typeName: string, methodName: string): string | undefined {
    const t = this.getType(typeName);
    if (!t) return undefined;
    const m = t.methods.find(m => m.name.toLowerCase() === methodName.toLowerCase());
    if (m) return m.returns;
    if (t.extends) return this.getMethodReturn(t.extends, methodName);
    return undefined;
  }
}

// ── Loader ───────────────────────────────────────────────

export function loadWoldBindings(extensionPath: string): WoldBindings {
  const bindings = new WoldBindings();
  try {
    const p = path.join(extensionPath, "generated", "roblox.wold");
    if (!fs.existsSync(p)) { console.log("[wolfram] no roblox.wold at " + p); return bindings; }
    const raw = fs.readFileSync(p, "utf-8");
    const file: WoldFile = JSON.parse(raw);
    for (const g of file.globals) bindings.globals.set(g.name.toLowerCase(), g);
    for (const f of file.functions) bindings.functions.set(f.name.toLowerCase(), f);
    for (const t of file.types) bindings.types.set(t.name.toLowerCase(), t);
    for (const e of file.enums) bindings.enums.set(e.name.toLowerCase(), e);
    console.log(`[wolfram] loaded ${bindings.globals.size} globals, ${bindings.functions.size} functions, ${bindings.types.size} types, ${bindings.enums.size} enums`);
  } catch (e: any) { console.error("[wolfram] failed to load bindings: " + e.message); }
  return bindings;
}

// ── Keywords ─────────────────────────────────────────────

interface KeywordDef { label: string; snippet?: string; doc: string; }

const KEYWORDS: KeywordDef[] = [
  { label: "if", snippet: "if (${1:condition}) {\n\t${0}\n}", doc: "Conditional branch. Executes the block if the condition is truthy.\n\n```wolfram\nif (x > 0) {\n    print(\"positive\")\n}\n```" },
  { label: "else", snippet: "else {\n\t${0}\n}", doc: "Fallback branch for `if` / `elif` chains." },
  { label: "elif", snippet: "elif (${1:condition}) {\n\t${0}\n}", doc: "Else-if branch. Tested when previous `if` / `elif` conditions were falsy." },
  { label: "while", snippet: "while (${1:condition}) {\n\t${0}\n}", doc: "Loop that repeats as long as the condition is truthy." },
  { label: "for", snippet: "for ${1:x} in ${2:items} {\n\t${0}\n}", doc: "Iterates over an array or table. Uses `ipairs` for arrays by default." },
  { label: "function", snippet: "function ${1:name}(${2:params}) {\n\t${0}\n}", doc: "Defines a named function." },
  { label: "class", snippet: "class ${1:Name} {\n\t${0}\n}", doc: "Defines a class with constructor, methods, and inheritance support." },
  { label: "struct", snippet: "struct ${1:Name} {\n\t${0}\n}", doc: "Defines an immutable data structure with named fields." },
  { label: "enum", snippet: "enum ${1:Name} {\n\t${0}\n}", doc: "Defines an enumeration — a fixed set of named values." },
  { label: "import", snippet: 'import "${1:path}" as ${2:alias}', doc: "Imports another module by relative path." },
  { label: "local", snippet: "local ${1:name} = ${0}", doc: "Declares a local variable scoped to the current block." },
  { label: "return", snippet: "return ${1:value}", doc: "Returns a value from a function." },
  { label: "true", doc: "Boolean truth value. Compiles to `true` in Luau." },
  { label: "false", doc: "Boolean false value. Compiles to `false` in Luau." },
  { label: "nil", doc: "Represents the absence of a value. Equivalent to `nil` in Luau." },
  { label: "self", doc: "Refers to the current instance inside a method." },
  { label: "break", doc: "Exits the innermost loop immediately." },
  { label: "continue", doc: "Skips the rest of the current loop iteration." },
  { label: "and", doc: "Logical AND operator. Short-circuits." },
  { label: "or", doc: "Logical OR operator. Short-circuits." },
  { label: "not", doc: "Logical NOT operator." },
  { label: "public", doc: "Access modifier for class members. Public members are accessible externally." },
  { label: "private", doc: "Access modifier for class members. Private members are module-scoped." },
  { label: "as", doc: "Alias keyword for `import` statements." },
  { label: "in", doc: "Used in `for` loops to iterate over a collection." },
];

// ── Utility ──────────────────────────────────────────────

function getLinePrefix(document: vscode.TextDocument, position: vscode.Position): string {
  const line = document.lineAt(position.line).text;
  return line.substring(0, position.character);
}

function wordUnderCursor(document: vscode.TextDocument, position: vscode.Position): string {
  const line = document.lineAt(position.line).text;
  const col = position.character;
  const before = line.substring(0, col);
  const m = before.match(/([\w.]+)$/);
  const prefix = m ? m[1] : "";
  const after = line.substring(col);
  const m2 = after.match(/^(\w+)/);
  return prefix + (m2 ? m2[1] : "");
}

function extractExprBeforeDot(source: string, lineIdx: number, col: number): string {
  let offset = 0;
  for (let i = 0; i < lineIdx; i++) {
    offset += source.split("\n")[i]?.length ?? 0;
  }
  offset += col;
  if (offset > source.length) offset = source.length;
  if (offset > 0 && source[offset - 1] === ".") offset--;
  if (offset > 0 && source[offset - 1] === ":") offset--;
  let start = offset;
  while (start > 0) {
    const c = source[start - 1];
    if (/[\w.]/.test(c)) { start--; } else { break; }
  }
  return source.substring(start, offset);
}

function extractLocalTypes(document: vscode.TextDocument, bindings: WoldBindings): Map<string, string> {
  const map = new Map<string, string>();
  const text = document.getText();

  // function MyFunc( / local function MyFunc( / public function MyFunc(
  const fnRe = /(?:^|\s)(?:local\s+|public\s+|private\s+)?function\s+(\w+)/gm;
  let m: RegExpExecArray | null;
  while ((m = fnRe.exec(text)) !== null) map.set(m[1], m[1]);

  // class Name / struct Name / enum Name
  const typeRe = /(?:^|\s)(?:public\s+|private\s+)?(?:class|struct|enum)\s+(\w+)/gm;
  while ((m = typeRe.exec(text)) !== null) { if (!map.has(m[1])) map.set(m[1], m[1]); }

  // local name = ClassName.new(...)
  const newRe = /local\s+(\w+)\s*=\s*(\w+(?:\.\w+)*)\.new\s*\(/g;
  while ((m = newRe.exec(text)) !== null) map.set(m[1], m[2]);

  // local name = expr:GetService("ServiceName")
  const svcRe = /local\s+(\w+)\s*=.*:GetService\s*\(\s*"([^"]+)"/g;
  while ((m = svcRe.exec(text)) !== null) map.set(m[1], m[2]);

  // local name = Chain.Of.Properties — resolve through bindings
  const chainRe = /local\s+(\w+)\s*=\s*([\w.]+)(?!\()/g;
  while ((m = chainRe.exec(text)) !== null) {
    if (map.has(m[1])) continue;
    const rhs = m[2];
    if (rhs.includes(".")) {
      const typeName = resolveExprType(rhs, bindings, new Map());
      if (typeName) map.set(m[1], typeName);
      continue;
    }
    // Simple assignment: resolve through globals
    const g = bindings.getGlobal(rhs);
    if (g) { map.set(m[1], g.type); continue; }
    const t = bindings.getType(rhs);
    if (t) { map.set(m[1], rhs); continue; }
  }

  // local name = <anything>   (generic local assignment — use "local" as placeholder type)
  const localRe = /local\s+(\w+)\s*=\s*[^(]/gm;
  while ((m = localRe.exec(text)) !== null) { if (!map.has(m[1])) map.set(m[1], "local"); }

  // function params (for(let x in ...) etc.)
  const paramRe = /for\s+(\w+)\s*(?:,\s*\w+\s*)?in\s+/g;
  while ((m = paramRe.exec(text)) !== null) { if (!map.has(m[1])) map.set(m[1], "any"); }

  return map;
}

function isInsideImportString(linePrefix: string): { partial: string; quote: string } | null {
  // import "partial|"  or  import 'partial|'
  const m = linePrefix.match(/import\s+(["'])([^"']*)$/);
  if (m) return { partial: m[2], quote: m[1] };
  // import "foo/bar" as alias — after the closing quote but before `as`
  // for now only match when cursor is inside the string
  return null;
}

function collectProjectWrmFiles(workspacePath: string): string[] {
  const files: string[] = [];
  function walk(dir: string, prefix: string) {
    try {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        if (entry.name.startsWith(".") || entry.name === "node_modules" || entry.name === "out" || entry.name === "target") continue;
        const fp = path.join(dir, entry.name);
        const rp = prefix ? prefix + "/" + entry.name : entry.name;
        if (entry.isDirectory()) {
          walk(fp, rp);
        } else if (entry.name.endsWith(".wrm")) {
          files.push(rp.replace(/\.wrm$/, ""));
        }
      }
    } catch {}
  }
  walk(path.join(workspacePath, "src"), "");
  return files;
}

// ── Completion item builders ─────────────────────────────
// sortText priority: 0=keyword  1=local  2=API  3=enum

function kwItem(label: string, snippet?: string): vscode.CompletionItem {
  const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Keyword);
  if (snippet) item.insertText = new vscode.SnippetString(snippet);
  item.sortText = "0" + label;
  return item;
}

function localItem(name: string, localType?: string): vscode.CompletionItem {
  const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
  item.sortText = "1" + name;
  if (localType && localType !== name && localType !== "local") {
    item.detail = localType;
  }
  return item;
}

function globalItem(g: WoldGlobal): vscode.CompletionItem {
  const item = new vscode.CompletionItem(g.name, vscode.CompletionItemKind.Variable);
  item.detail = `${g.type} — ${g.description}`;
  item.sortText = "2" + g.name;
  return item;
}

function funcItem(f: WoldFunction): vscode.CompletionItem {
  const params = f.params.map(p => `${p.name}: ${p.type}`).join(", ");
  const item = new vscode.CompletionItem(f.name, vscode.CompletionItemKind.Function);
  item.detail = `(${params}): ${f.returns}`;
  item.documentation = f.description || undefined;
  if (f.params.length > 0) {
    const sp = f.params.map((p, i) => `\${${i + 1}:${p.name}}`).join(", ");
    item.insertText = new vscode.SnippetString(`${f.name}(${sp})`);
  }
  item.sortText = "2" + f.name;
  return item;
}

function enumItem(e: WoldEnum): vscode.CompletionItem {
  const item = new vscode.CompletionItem(`Enum.${e.name}`, vscode.CompletionItemKind.Enum);
  item.detail = e.items.join(", ");
  item.sortText = "3" + e.name;
  return item;
}

function enumMemberItem(enumName: string, memberName: string): vscode.CompletionItem {
  const item = new vscode.CompletionItem(`Enum.${enumName}.${memberName}`, vscode.CompletionItemKind.EnumMember);
  item.sortText = "3" + enumName + "." + memberName;
  return item;
}

function propItem(p: WoldProperty): vscode.CompletionItem {
  const item = new vscode.CompletionItem(p.name, vscode.CompletionItemKind.Property);
  item.detail = `${p.type}${p.rw ? " (read/write)" : " (read-only)"}`;
  item.documentation = p.description || undefined;
  item.sortText = "3" + p.name;
  return item;
}

function methodItem(m: WoldMethod, _colon: boolean): vscode.CompletionItem {
  const item = new vscode.CompletionItem(m.name, vscode.CompletionItemKind.Method);
  const params = m.params.map(p => `${p.name}: ${p.type}`).join(", ");
  item.detail = `(${params}): ${m.returns}`;
  item.documentation = m.description || undefined;
  if (m.params.length > 0) {
    const sp = m.params.map((p, i) => `\${${i + 1}:${p.name}}`).join(", ");
    item.insertText = new vscode.SnippetString(`${m.name}(${sp})`);
  } else {
    item.insertText = `${m.name}()`;
  }
  item.sortText = "3" + m.name;
  return item;
}

function pathItem(filePath: string): vscode.CompletionItem {
  const item = new vscode.CompletionItem(filePath, vscode.CompletionItemKind.File);
  item.sortText = "0" + filePath;
  return item;
}

// ── Expression type resolver ─────────────────────────────

function resolveExprType(
  expr: string,
  bindings: WoldBindings,
  locals: Map<string, string>,
): string | undefined {
  if (!expr) return undefined;
  const parts = expr.split(".");
  const root = parts[0];

  const g = bindings.getGlobal(root);
  if (g) {
    let current = g.type;
    for (let i = 1; i < parts.length; i++) {
      const props = bindings.getAllProperties(current);
      const p = props.find(x => x.name.toLowerCase() === parts[i].toLowerCase());
      if (p) { current = p.type; continue; }
      const methods = bindings.getAllMethods(current);
      const m = methods.find(x => x.name.toLowerCase() === parts[i].toLowerCase());
      if (m) { current = m.returns; continue; }
      return "Instance";
    }
    return current;
  }

  if (locals.has(root)) return locals.get(root);
  if (bindings.getType(root)) {
    if (parts.length === 1) return root;
    // Chain through properties of the known type
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

// ── Context detection ────────────────────────────────────

const enum Ctx {
  IMPORT_PATH,
  DEFINITION_NAME,
  DOT_COLON,
  ENUM_VALUE,
  VALUE_EXPRESSION,
  COMMENT,
  STRING,
  STATEMENT_START,
  EXPRESSION,
}

function isValuePosition(textBeforeWord: string): boolean {
  return /[=(,\[\-+*\/<>!]|\b(return|and|or)\b\s*$/.test(textBeforeWord);
}

function isComment(line: string): boolean {
  const trimmed = line.trimStart();
  return trimmed.startsWith("//") || trimmed.startsWith("--");
}

function detectContext(linePrefix: string): Ctx {
  const trimmed = linePrefix.trimStart();
  if (!trimmed) return Ctx.STATEMENT_START;

  // Import path
  if (/^import\s+["'][^"']*$/.test(trimmed)) return Ctx.IMPORT_PATH;

  // Comment
  if (isComment(linePrefix)) return Ctx.COMMENT;

  // Regular string (not import)
  let inStr = false; let quote = "";
  for (const ch of linePrefix) {
    if (!inStr && (ch === '"' || ch === "'")) { inStr = true; quote = ch; }
    else if (inStr && ch === quote) { inStr = false; quote = ""; }
  }
  if (inStr) return Ctx.STRING;

  // Definition name: after function/class/struct/enum keyword
  // Matches: "function Foo|" but NOT "function Foo(|" (cursor past name into params)
  const defMatch = trimmed.match(/^(?:local\s+)?(?:public\s+|private\s+)?(function|class|struct|enum)\s+(\w*)$/);
  if (defMatch) {
    const name = defMatch[2];
    if (name) return Ctx.DEFINITION_NAME;
    // Just the keyword with no name yet — still definition context, suppress
    return Ctx.STATEMENT_START;
  }

  // Dot/colon member access
  const lastChar = linePrefix[linePrefix.length - 1] ?? "";
  if (lastChar === "." || lastChar === ":") {
    // Check if it's Enum. specifically (Enum member context)
    if (/Enum\.$/.test(trimmed)) return Ctx.ENUM_VALUE;
    return Ctx.DOT_COLON;
  }

  // Enum value context: line ends with Enum.SomeMember where preceding context is a value position
  const wordMatch = trimmed.match(/([\w.]+)$/);
  const wordPrefix = wordMatch ? wordMatch[1] : "";
  if (/^(?:Enum\.?|Enum\.\w*)$/i.test(wordPrefix)) {
    const beforeWord = trimmed.substring(0, trimmed.length - wordPrefix.length);
    if (isValuePosition(beforeWord)) return Ctx.ENUM_VALUE;
    return Ctx.EXPRESSION;
  }

  // Value expression: after =, ==, !=, <, >, (, ,, return, and, or, [
  const beforeWord = trimmed.substring(0, trimmed.length - (wordPrefix.length));
  if (isValuePosition(beforeWord)) return Ctx.VALUE_EXPRESSION;

  // Statement start
  const indentation = linePrefix.length - trimmed.length;
  if (indentation > 0 && !trimmed) return Ctx.STATEMENT_START;

  return Ctx.EXPRESSION;
}

const VALUE_KEYWORDS: KeywordDef[] = KEYWORDS.filter(k =>
  ["true", "false", "nil", "self"].includes(k.label));
const STRUCTURAL_KEYWORDS: KeywordDef[] = KEYWORDS.filter(k =>
  ["if", "else", "elif", "while", "for", "function", "class", "struct", "enum", "import", "local", "return", "break", "continue", "public", "private"].includes(k.label));

// ── Completion Provider ─────────────────────────────────

export function createCompletionProvider(bindings: WoldBindings): vscode.CompletionItemProvider {
  return {
    provideCompletionItems(
      document: vscode.TextDocument,
      position: vscode.Position,
      token: vscode.CancellationToken,
      _context: vscode.CompletionContext,
    ): vscode.CompletionItem[] {
      if (token.isCancellationRequested) return [];

      const linePrefix = getLinePrefix(document, position);
      const ctx = detectContext(linePrefix);

      // ── Import path ──────────────────────────────────
      if (ctx === Ctx.IMPORT_PATH) {
        const ws = vscode.workspace.workspaceFolders?.[0];
        if (ws) {
          const files = collectProjectWrmFiles(ws.uri.fsPath);
          const importCtx = isInsideImportString(linePrefix);
          const partial = importCtx?.partial.toLowerCase() ?? "";
          return files
            .filter(f => f.toLowerCase().startsWith(partial) || f.toLowerCase().includes(partial))
            .map(pathItem);
        }
        return [];
      }

      // ── Definition name / comment / string → suppress ─
      if (ctx === Ctx.DEFINITION_NAME || ctx === Ctx.COMMENT || ctx === Ctx.STRING) {
        return [];
      }

      // ── Dot/colon member access ──────────────────────
      if (ctx === Ctx.DOT_COLON) {
        const lastChar = linePrefix[linePrefix.length - 1] ?? "";
        const expr = extractExprBeforeDot(document.getText(), position.line, position.character);
        const locals = extractLocalTypes(document, bindings);
        const typeName = resolveExprType(expr, bindings, locals);
        if (typeName) {
          const items: vscode.CompletionItem[] = [];
          if (lastChar === ":") {
            for (const m of bindings.getAllMethods(typeName)) items.push(methodItem(m, true));
          } else {
            for (const p of bindings.getAllProperties(typeName)) items.push(propItem(p));
            for (const m of bindings.getAllMethods(typeName)) items.push(methodItem(m, false));
          }
          return items;
        }
        return [];
      }

      // ── Extract prefix ───────────────────────────────
      const fullLine = linePrefix.trimStart();
      const wordMatch = fullLine.match(/([\w.]+)$/);
      const wordPrefix = wordMatch ? wordMatch[1].toLowerCase() : "";
      const seen = new Set<string>();
      const items: vscode.CompletionItem[] = [];
      const locals = extractLocalTypes(document, bindings);

      function push(item: vscode.CompletionItem) {
        const lbl = item.label as string;
        if (!seen.has(lbl)) { seen.add(lbl); items.push(item); }
      }

      function addLocals() {
        for (const [name, type] of locals) {
          if (name.toLowerCase().startsWith(wordPrefix)) push(localItem(name, type));
        }
      }

      function addApiGlobals() {
        for (const [_, g] of bindings.globals) {
          if (g.name.toLowerCase().startsWith(wordPrefix)) push(globalItem(g));
        }
        for (const [_, f] of bindings.functions) {
          if (f.name.toLowerCase().startsWith(wordPrefix)) push(funcItem(f));
        }
      }

      function addEnumValues() {
        for (const [_, e] of bindings.enums) {
          if (e.name.toLowerCase().startsWith(wordPrefix)) push(enumItem(e));
          for (const member of e.items) {
            const full = `enum.${e.name}.${member}`.toLowerCase();
            if (full.startsWith(fullLine.toLowerCase())) push(enumMemberItem(e.name, member));
          }
        }
      }

      function addValueKeywords() {
        for (const kw of VALUE_KEYWORDS) {
          if (kw.label.startsWith(wordPrefix)) push(kwItem(kw.label, kw.snippet));
        }
      }

      function addStructuralKeywords() {
        for (const kw of STRUCTURAL_KEYWORDS) {
          if (kw.label.startsWith(wordPrefix)) push(kwItem(kw.label, kw.snippet));
        }
      }

      // ── Dispatch by context ──────────────────────────
      switch (ctx) {
        case Ctx.ENUM_VALUE:
          addEnumValues();
          break;

        case Ctx.VALUE_EXPRESSION:
          addLocals();
          addApiGlobals();
          addValueKeywords();
          addEnumValues();
          break;

        case Ctx.STATEMENT_START:
          addStructuralKeywords();
          addLocals();
          break;

        case Ctx.EXPRESSION:
        default:
          addLocals();
          addApiGlobals();
          addStructuralKeywords();
          addValueKeywords();
          break;
      }

      return items;
    },
  };
}

// ── Hover Provider ───────────────────────────────────────

function extractWordAt(document: vscode.TextDocument, position: vscode.Position): string {
  const line = document.lineAt(position.line).text;
  const col = position.character;
  if (col >= line.length) return "";
  const start = col - line.substring(0, col).search(/[\w.]+$/);
  const end = col + line.substring(col).search(/\W|$/);
  const endIdx = end === -1 ? line.length : col + end;
  return line.substring(start, endIdx);
}

export function createHoverProvider(bindings: WoldBindings): vscode.HoverProvider {
  const kwDocs = new Map<string, string>();
  for (const kw of KEYWORDS) kwDocs.set(kw.label, kw.doc);

  return {
    provideHover(
      document: vscode.TextDocument,
      position: vscode.Position,
      _token: vscode.CancellationToken,
    ): vscode.Hover | null {
      const word = extractWordAt(document, position);
      const lower = word.toLowerCase();

      const kwDoc = kwDocs.get(lower);
      if (kwDoc) return new vscode.Hover(new vscode.MarkdownString(kwDoc));

      const g = bindings.getGlobal(word);
      if (g) {
        const md = new vscode.MarkdownString();
        md.appendCodeblock(`global ${g.name}: ${g.type}`, "wolfram");
        if (g.description) md.appendMarkdown(`\n\n${g.description}`);
        return new vscode.Hover(md);
      }

      const f = bindings.functions.get(lower);
      if (f) {
        const params = f.params.map(p => `${p.name}: ${p.type}`).join(", ");
        const md = new vscode.MarkdownString();
        md.appendCodeblock(`function ${f.name}(${params}): ${f.returns}`, "wolfram");
        if (f.description) md.appendMarkdown(`\n\n${f.description}`);
        return new vscode.Hover(md);
      }

      const e = bindings.enums.get(lower);
      if (e) {
        const md = new vscode.MarkdownString();
        md.appendCodeblock(`enum ${e.name}`, "wolfram");
        if (e.items.length > 0) md.appendMarkdown(`\n\n**Items:** ${e.items.join(", ")}`);
        if (e.description) md.appendMarkdown(`\n\n${e.description}`);
        return new vscode.Hover(md);
      }

      const locals = extractLocalTypes(document, bindings);
      const localType = locals.get(word);
      if (localType) {
        const md = new vscode.MarkdownString();
        md.appendCodeblock(`local ${word}${localType !== word && localType !== "local" ? ": " + localType : ""}`, "wolfram");
        return new vscode.Hover(md);
      }

      return null;
    },
  };
}
