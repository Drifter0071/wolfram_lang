/**
 * lsp/tester.ts — Wolfram LSP & Autocomplete Tester CLI
 *
 * Usage:
 *   npx ts-node src/lsp/tester.ts --snippet "local x = Vector3.|"
 *   npx ts-node src/lsp/tester.ts --snippet "Instance.|new" --context dot
 *   npx ts-node src/lsp/tester.ts --snippet "part.Touched:|" --context dot
 *   npx ts-node src/lsp/tester.ts --snippet "local x = Ov|"
 *   npx ts-node src/lsp/tester.ts --file path/to/file.wrm
 *   npx ts-node src/lsp/tester.ts --snippet "local x = unknown_var|" --diagnostics
 *
 * The `|` character marks the cursor position in the snippet.
 * If omitted, the cursor is placed at the end of the snippet.
 * If pipe is at the very end (after last char), it marks cursor position
 * after the last character (useful for . or : completions).
 *
 * Modes:
 *   --context dot    — test dot/colon completions (what appears after typing .)
 *   --context value  — test value-expression completions (after =, return, etc.)
 *   --context expr   — test general expression completions
 *   --context stmt   — test statement-start completions (keywords, etc.)
 *   --diagnostics    — also run diagnostic checks
 *   --hover          — test hover at cursor position
 *   --all            — all of the above
 */

import * as path from "path";
import * as fs from "fs";
import { computeDiagnostics } from "./diagnostics";
import { Bindings } from "./bindings";
import { WoldType } from "./bindings";
import { matchScore as sharedMatchScore } from "./utils";

// ── Parse CLI args ────────────────────────────────────────────────────────

interface Options {
	snippet: string | null;
	filePath: string | null;
	context: "dot" | "value" | "expr" | "stmt" | "auto";
	diagnostics: boolean;
	hover: boolean;
}

function parseArgs(): Options {
	const args = process.argv.slice(2);
	const opts: Options = {
		snippet: null,
		filePath: null,
		context: "auto",
		diagnostics: false,
		hover: false,
	};

	for (let i = 0; i < args.length; i++) {
		switch (args[i]) {
			case "--snippet":
				opts.snippet = args[++i] ?? "";
				break;
			case "--file":
				opts.filePath = args[++i] ?? "";
				break;
			case "--context":
				opts.context = (args[++i] as Options["context"]) ?? "auto";
				break;
			case "--diagnostics":
				opts.diagnostics = true;
				break;
			case "--hover":
				opts.hover = true;
				break;
			case "--all":
				opts.diagnostics = true;
				opts.hover = true;
				break;
		}
	}

	return opts;
}

// ── Helpers ────────────────────────────────────────────────────────────────

function gray(s: string): string {
	return `\x1b[90m${s}\x1b[0m`;
}

function green(s: string): string {
	return `\x1b[32m${s}\x1b[0m`;
}

function yellow(s: string): string {
	return `\x1b[33m${s}\x1b[0m`;
}

function red(s: string): string {
	return `\x1b[31m${s}\x1b[0m`;
}

function cyan(s: string): string {
	return `\x1b[36m${s}\x1b[0m`;
}

function bold(s: string): string {
	return `\x1b[1m${s}\x1b[0m`;
}

interface SnippetInfo {
	source: string;
	line: number;
	char: number;
}

/**
 * Extracts the source text and cursor position from a snippet.
 * The `|` character marks the cursor. If not found, cursor is at end.
 */
function parseSnippet(raw: string): SnippetInfo {
	const pipeIdx = raw.indexOf("|");
	if (pipeIdx === -1) {
		return { source: raw, line: 0, char: raw.length };
	}
	const before = raw.slice(0, pipeIdx);
	const after = raw.slice(pipeIdx + 1);
	const source = before + after;
	const line = (before.match(/\n/g) || []).length;
	const lastNewline = before.lastIndexOf("\n");
	const char = lastNewline === -1 ? pipeIdx : pipeIdx - lastNewline - 1;
	return { source, line, char };
}

function detectContext(linePrefix: string): string {
	const trimmed = linePrefix.trimStart();
	if (!trimmed) return "statement_start";
	if (/^import\s+["'][^"']*$/.test(trimmed)) return "import_path";
	if (/^\/\/|^--/.test(trimmed)) return "comment";
	let inStr = false;
	let quote = "";
	for (const ch of linePrefix) {
		if (!inStr && (ch === '"' || ch === "'")) {
			inStr = true;
			quote = ch;
		} else if (inStr && ch === quote) {
			inStr = false;
			quote = "";
		}
	}
	if (inStr) return "string";
	const lastChar = linePrefix[linePrefix.length - 1] ?? "";
	if (lastChar === "." || lastChar === ":") {
		if (/Enum\.$/.test(trimmed)) return "enum_member";
		return "dot_colon";
	}
	const defMatch = trimmed.match(/^(?:local\s+)?(?:public\s+|private\s+)?(function|class|struct|enum)\s+(\w*)$/);
	if (defMatch) return "definition_name";
	if (/[=(,\[\-+*\/<>!]|\b(return|and|or)\b\s*$/.test(trimmed))
		return "value_expression";
	if (/^[\w.]+$/.test(trimmed) && trimmed === linePrefix)
		return "statement_start";
	return "expression";
}

function extractExprBeforeDot(source: string, lineIdx: number, col: number): string {
	let offset = 0;
	const lines = source.split("\n");
	for (let i = 0; i < lineIdx; i++) {
		offset += (lines[i]?.length ?? 0) + 1;
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

// ── Load bindings ─────────────────────────────────────────────────────────

function getExtensionRoot(): string {
	let dir = __dirname;
	while (dir && path.basename(dir) !== "vscode-extension") {
		const parent = path.dirname(dir);
		if (parent === dir) break;
		dir = parent;
	}
	return dir;
}

function loadBindings(): Bindings {
	const root = getExtensionRoot();
	const b = new Bindings();
	b.load(root);
	return b;
}

// ── Resolve type chain ────────────────────────────────────────────────────

function resolveExprType(
	expr: string,
	bindings: Bindings,
	locals: Map<string, string>,
): WoldType | undefined {
	if (!expr) return undefined;
	const parts = expr.split(".");

	// Try the full expression as a known type first
	const fullType = bindings.getType(parts.join("."));
	if (fullType) return fullType;

	const root = parts[0];
	const g = bindings.getGlobal(root);
	if (g) {
		let current = g.type;
		for (let i = 1; i < parts.length; i++) {
			const t = bindings.getType(current);
			if (!t) return undefined;
			const prop = t.properties.find(p => p.name.toLowerCase() === parts[i].toLowerCase());
			if (prop) { current = prop.type; continue; }
			const meth = t.methods.find(m => m.name.toLowerCase() === parts[i].toLowerCase());
			if (meth) { current = meth.returns; continue; }
			return undefined;
		}
		return bindings.getType(current);
	}

	if (locals.has(root)) {
		const lt = locals.get(root)!;
		if (parts.length === 1) return bindings.getType(lt);
		let current = lt;
		for (let i = 1; i < parts.length; i++) {
			const t = bindings.getType(current);
			if (!t) return undefined;
			const prop = t.properties.find(p => p.name.toLowerCase() === parts[i].toLowerCase());
			if (prop) { current = prop.type; continue; }
			const meth = t.methods.find(m => m.name.toLowerCase() === parts[i].toLowerCase());
			if (meth) { current = meth.returns; continue; }
			const ev = t.events.find(e => e.name.toLowerCase() === parts[i].toLowerCase());
			if (ev) return bindings.getType("RBXScriptSignal");
			return undefined;
		}
		return bindings.getType(current);
	}

	return bindings.getType(root);
}

// ── Extract local types ───────────────────────────────────────────────────

function extractLocalTypes(source: string, bindings: Bindings): Map<string, string> {
	const map = new Map<string, string>();
	const text = source;

	const fnRe = /(?:^|\s)(?:local\s+|public\s+|private\s+)?function\s+(\w+)/gm;
	let m: RegExpExecArray | null;
	while ((m = fnRe.exec(text)) !== null) map.set(m[1], m[1]);

	const typeRe = /(?:^|\s)(?:public\s+|private\s+)?(?:class|struct|enum)\s+(\w+)/gm;
	while ((m = typeRe.exec(text)) !== null) {
		if (!map.has(m[1])) map.set(m[1], m[1]);
	}

	const newRe = /local\s+(\w+)\s*=\s*(\w+(?:\.\w+)*)\.new\s*\(/g;
	while ((m = newRe.exec(text)) !== null) map.set(m[1], m[2]);

	const svcRe = /local\s+(\w+)\s*=.*:GetService\s*\(\s*"([^"]+)"/g;
	while ((m = svcRe.exec(text)) !== null) map.set(m[1], m[2]);

	const chainRe = /local\s+(\w+)\s*=\s*([\w.]+)(?!\()/g;
	while ((m = chainRe.exec(text)) !== null) {
		if (map.has(m[1])) continue;
		const rhs = m[2];
		if (rhs.includes(".")) {
			const t = resolveChainedType(rhs, bindings);
			if (t) map.set(m[1], t.name);
			continue;
		}
		const g = bindings.getGlobal(rhs);
		if (g) { map.set(m[1], g.type); continue; }
		const t = bindings.getType(rhs);
		if (t) { map.set(m[1], rhs); continue; }
	}

	const localRe = /local\s+(\w+)\s*=\s*[^(]/gm;
	while ((m = localRe.exec(text)) !== null) {
		if (!map.has(m[1])) map.set(m[1], "local");
	}

	const paramRe = /for\s+(\w+)\s*(?:,\s*\w+\s*)?in\s+/g;
	while ((m = paramRe.exec(text)) !== null) {
		if (!map.has(m[1])) map.set(m[1], "any");
	}

	return map;
}

function resolveChainedType(expr: string, bindings: Bindings): WoldType | undefined {
	const parts = expr.split(".");
	const root = parts[0];
	const g = bindings.getGlobal(root);
	if (g) {
		let current = g.type;
		for (let i = 1; i < parts.length; i++) {
			const t = bindings.getType(current);
			if (!t) return undefined;
			const prop = t.properties.find(p => p.name.toLowerCase() === parts[i].toLowerCase());
			if (prop) { current = prop.type; continue; }
			const meth = t.methods.find(m => m.name.toLowerCase() === parts[i].toLowerCase());
			if (meth) { current = meth.returns; continue; }
			return undefined;
		}
		return bindings.getType(current);
	}
	// Root is not a global — check if it's a known type and chain through it
	const rootType = bindings.getType(root);
	if (rootType) {
		if (parts.length === 1) return rootType;
		let current = root;
		for (let i = 1; i < parts.length; i++) {
			const t = bindings.getType(current);
			if (!t) return undefined;
			const prop = t.properties.find(p => p.name.toLowerCase() === parts[i].toLowerCase());
			if (prop) { current = prop.type; continue; }
			return undefined;
		}
		return bindings.getType(current);
	}
	return undefined;
}

// ── Print helpers ─────────────────────────────────────────────────────────

function printDiagnostics(source: string, filePath?: string): void {
	console.log(bold("\n── Diagnostics ──"));
	const diags = computeDiagnostics(source, filePath);
	if (diags.length === 0) {
		console.log(green("  ✓ No issues found"));
		return;
	}
	const lines = source.split("\n");
	for (const d of diags) {
		const prefix = d.severity === 1 ? yellow("⚠ Warning") : red("✗ Error");
		const line = d.range.start.line;
		const snippet = lines[line]?.trim() ?? "";
		console.log(`  ${prefix}  ${gray(`[${d.source}]`)}  ${d.message}`);
		if (snippet) console.log(gray(`    → line ${line + 1}: ${snippet}`));
	}
}

function printHover(source: string, line: number, char: number, bindings: Bindings): void {
	console.log(bold("\n── Hover ──"));
	const word = wordAt(source, line, char);
	if (!word) {
		console.log(yellow("  (no word at cursor)"));
		return;
	}
	console.log(`${gray("Word:")} ${cyan(word)}`);

	const lower = word.toLowerCase();

	const g = bindings.getGlobal(word);
	if (g) {
		console.log(`  ${bold("global")} ${green(g.name)}: ${g.type}`);
		if (g.description) console.log(gray(`  ${g.description}`));
		const t = bindings.getType(g.type);
		if (t) {
			console.log(gray(`  ${t.properties.length} properties, ${t.methods.length} methods`));
		}
		return;
	}

	const t = bindings.getType(word);
	if (t) {
		console.log(`  ${bold("type")} ${green(t.name)}${t.extends ? ` extends ${t.extends}` : ""}`);
		if (t.description) console.log(gray(`  ${t.description}`));
		if (t.properties.length > 0) {
			console.log(gray(`  Properties:`));
			for (const p of t.properties.slice(0, 10)) {
				console.log(gray(`    .${p.name}: ${p.type}${p.rw ? " (rw)" : ""}`));
			}
			if (t.properties.length > 10) console.log(gray(`    ... and ${t.properties.length - 10} more`));
		}
		if (t.methods.length > 0) {
			console.log(gray(`  Methods:`));
			for (const m of t.methods.slice(0, 10)) {
				const params = m.params.map(p => `${p.name}: ${p.type}`).join(", ");
				console.log(gray(`    :${m.name}(${params}): ${m.returns}`));
			}
			if (t.methods.length > 10) console.log(gray(`    ... and ${t.methods.length - 10} more`));
		}
		return;
	}

	console.log(yellow(`  (no hover info for '${word}')`));
}

function wordAt(source: string, line: number, char: number): string | null {
	const lines = source.split("\n");
	if (line >= lines.length) return null;
	const l = lines[line];
	if (char >= l.length) return null;
	const start = char - l.substring(0, char).search(/[\w.]+$/);
	const end = char + l.substring(char).search(/\W|$/);
	const endIdx = end === -1 ? l.length : char + end;
	return l.substring(start, endIdx);
}

function printCompletions(
	source: string,
	line: number,
	char: number,
	context: string,
	bindings: Bindings,
): void {
	console.log(bold("\n── Completions ──"));

	const linePrefix = source.split("\n")[line]?.substring(0, char) ?? "";
	const autoCtx = detectContext(linePrefix);
	const ctxMap: Record<string, string> = { dot: "dot_colon", value: "value_expression", expr: "expression", stmt: "statement_start" };
	const effectiveCtx = context === "auto" ? autoCtx : (ctxMap[context] || context);

	console.log(`${gray("Context:")} ${cyan(effectiveCtx)}`);

	if (effectiveCtx === "comment" || effectiveCtx === "string" || effectiveCtx === "definition_name") {
		console.log(yellow("  (completions suppressed in this context)"));
		return;
	}

	if (effectiveCtx === "dot_colon") {
		const lastChar = linePrefix[linePrefix.length - 1] ?? "";
		const expr = extractExprBeforeDot(source, line, char);
		console.log(`${gray("Expr before dot/colon:")} ${cyan(expr)}`);

		const locals = extractLocalTypes(source, bindings);
		const type = resolveExprType(expr, bindings, locals);
		if (!type) {
			console.log(red(`  Unknown type for '${expr}' — no completions available`));
			return;
		}
		console.log(`${gray("Resolved type:")} ${green(type.name)}`);

		if (lastChar === ":") {
			const methods = bindings.getAllMethods(type.name);
			if (methods.length === 0) {
				console.log(yellow("  (no methods)"));
				return;
			}
			console.log(gray(`  ${methods.length} methods:`));
			for (const m of methods) {
				const params = m.params.map(p => `${p.name}: ${p.type}`).join(", ");
				console.log(`  ${green(`:${m.name}(${params})`)} ${yellow("→")} ${m.returns}${m.description ? gray(`  — ${m.description}`) : ""}`);
			}
		} else {
			const props = bindings.getAllProperties(type.name);
			const methods = bindings.getAllMethods(type.name);
			const events = bindings.getAllEvents(type.name);
			console.log(gray(`  ${props.length} properties + ${methods.length} methods + ${events.length} events:`));
			for (const p of props) {
				console.log(`  ${cyan(`.${p.name}`)}: ${p.type}${p.rw ? " (rw)" : " (ro)"}${p.description ? gray(` — ${p.description}`) : ""}`);
			}
			for (const e of events) {
				console.log(`  ${yellow(`.${e.name}`)}: RBXScriptSignal${e.description ? gray(` — ${e.description}`) : ""}`);
			}
			for (const m of methods) {
				const params = m.params.map(p => `${p.name}: ${p.type}`).join(", ");
				console.log(`  ${green(`.${m.name}(${params})`)} ${yellow("→")} ${m.returns}${m.description ? gray(`  — ${m.description}`) : ""}`);
			}
		}
		return;
	}

	// General expression / value / statement completions
	const fullLine = linePrefix.trimStart();
	const wordMatch = fullLine.match(/([\w.]+)$/);
	const wordPrefix = (wordMatch ? wordMatch[1] : "").toLowerCase();

	console.log(`${gray("Prefix:")} ${cyan(wordPrefix || "(empty)")}`);

	const items: { label: string; kind: string; detail?: string; sort: string }[] = [];
	const seen = new Set<string>();

	function ms(label: string): number { return sharedMatchScore(label, wordPrefix); }

	const add = (label: string, kind: string, detail?: string, sort: string = "9") => {
		if (seen.has(label)) return;
		const score = ms(label);
		if (score < 0) return;
		seen.add(label);
		// Prefix matches (0-1) before substring (2)
		const prefix = score <= 1 ? "0" : "1";
		items.push({ label, kind, detail, sort: prefix + sort + label });
	};

	const locals = extractLocalTypes(source, bindings);

	// Locals
	for (const [name, type] of locals) {
		add(name, "variable", type !== "local" ? `: ${type}` : undefined, "1");
	}

	// Keywords
	const keywords = effectiveCtx === "statement_start"
		? ["if", "else", "elif", "while", "for", "function", "class", "struct", "enum", "import", "local", "return", "break", "continue", "public", "private"]
		: ["true", "false", "nil", "self"];
	for (const kw of keywords) {
		add(kw, "keyword", undefined, "0");
	}

	// Globals & types from bindings
	for (const [, g] of bindings.globals) {
		if (!g.name.toLowerCase().startsWith(wordPrefix) && wordPrefix) continue;
		add(g.name, "global", g.type, "2");
	}
	for (const [, t] of bindings.types) {
		if (!t.name.toLowerCase().startsWith(wordPrefix) && wordPrefix) continue;
		const methodCount = t.methods.length;
		const propCount = t.properties.length;
		let detail = t.extends ? `extends ${t.extends}` : "";
		if (propCount > 0 || methodCount > 0) {
			detail += (detail ? ", " : "") + `${propCount}p ${methodCount}m`;
		}
		add(t.name, "type", detail || undefined, "2");
	}

	// Sort and display
	items.sort((a, b) => a.sort.localeCompare(b.sort) || a.label.localeCompare(b.label));

	const maxShow = 40;
	console.log(gray(`  ${items.length} matches${items.length > maxShow ? ` (showing first ${maxShow})` : ""}:`));
	for (const item of items.slice(0, maxShow)) {
		const kindIcon = { variable: "V", keyword: "K", global: "G", type: "T", function: "F" }[item.kind] ?? "?";
		const kindColor = kindIcon === "K" ? yellow : kindIcon === "T" ? cyan : kindIcon === "G" ? green : gray;
		console.log(`  ${kindColor(kindIcon)} ${item.label}${item.detail ? gray(`  ${item.detail}`) : ""}`);
	}
	if (items.length > maxShow) {
		console.log(gray(`  ... and ${items.length - maxShow} more`));
	}
}

// ── Main ───────────────────────────────────────────────────────────────────

function main(): void {
	const opts = parseArgs();

	if (!opts.snippet && !opts.filePath) {
		console.log(`
${bold("Wolfram LSP Tester")} ${gray("— test completions, diagnostics, and hover")}

${yellow("Usage:")}
  npx ts-node src/lsp/tester.ts --snippet "${cyan("local x = Vector3.|")}"
  npx ts-node src/lsp/tester.ts --snippet "${cyan("part.Touched:|")}" --context dot
  npx ts-node src/lsp/tester.ts --snippet "${cyan("CCFRa|")}"
  npx ts-node src/lsp/tester.ts --snippet "${cyan("local x = Ov|")}" --diagnostics
  npx ts-node src/lsp/tester.ts --file ${cyan("path/to/file.wrm")} --all

${yellow("Flags:")}
  --snippet <code>   Code snippet with | as cursor marker
  --file <path>      Read source from a file
  --context <mode>    dot | value | expr | stmt | auto (default: auto)
  --diagnostics       Run diagnostic checks
  --hover             Test hover information at cursor
  --all               Enable all checks
`);
		return;
	}

	let source: string;
	let line: number;
	let char: number;
	let filePath: string | undefined;

	if (opts.snippet !== null) {
		const info = parseSnippet(opts.snippet);
		source = info.source;
		line = info.line;
		char = info.char;
		filePath = undefined;
	} else {
		const rawSource = fs.readFileSync(opts.filePath!, "utf-8");
		const info = parseSnippet(rawSource);
		source = info.source;
		line = info.line;
		char = info.char;
		filePath = opts.filePath ?? undefined;
	}

	console.log(bold("Wolfram LSP Tester"));
	console.log(gray("───────────────────"));

	const bindings = loadBindings();

	// Show the code with cursor
	const lines = source.split("\n");
	if (lines.length <= 30) {
		console.log(bold("\n── Source ──"));
		for (let i = 0; i < lines.length; i++) {
			const num = gray(`${i + 1}`.padStart(2));
			if (i === line) {
				const before = lines[i].substring(0, char);
				const at = lines[i][char] ?? "";
				const after = lines[i].substring(char + (at ? 1 : 0));
				console.log(`${num} ${before}${yellow(at || "█")}${after}`);
			} else {
				console.log(`${num} ${lines[i]}`);
			}
		}
	}

	// Diagnostics
	if (opts.diagnostics || opts.snippet !== null) {
		printDiagnostics(source, filePath);
	}

	// Hover
	if (opts.hover) {
		printHover(source, line, char, bindings);
	}

	// Completions
	printCompletions(source, line, char, opts.context, bindings);
}

main();
