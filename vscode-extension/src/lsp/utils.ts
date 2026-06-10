import { TextDocument } from "vscode-languageserver-textdocument";

export function getLinePrefix(document: TextDocument, lineIdx: number, charIdx: number): string {
    const content = document.getText();
    const lines = content.split("\n");
    if (lineIdx >= lines.length) return "";
    return lines[lineIdx].substring(0, charIdx);
}

export function extractWordBefore(document: TextDocument, lineIdx: number, charIdx: number): string {
    const line = getLinePrefix(document, lineIdx, charIdx);
    const m = line.match(/([\w.]+)$/);
    return m ? m[1] : "";
}

export function extractWordAround(document: TextDocument, lineIdx: number, charIdx: number): string {
    const content = document.getText();
    const lines = content.split("\n");
    if (lineIdx >= lines.length) return "";
    const line = lines[lineIdx];
    const start = line.substring(0, charIdx).search(/[\w.]+$/);
    const startIdx = start === -1 ? charIdx : charIdx - (line.substring(0, charIdx).length - start);
    const end = line.substring(charIdx).search(/\W|$/);
    const endIdx = end === -1 ? line.length : charIdx + end;
    return line.substring(startIdx, endIdx);
}

export function extractExprBeforeDot(document: TextDocument, lineIdx: number, charIdx: number): string {
    const content = document.getText();
    const lines = content.split("\n");
    let offset = 0;
    for (let i = 0; i < lineIdx; i++) {
        offset += (lines[i]?.length ?? 0) + 1;
    }
    offset += charIdx;
    if (offset > content.length) offset = content.length;
    // offset is now at the character AFTER the dot (cursor position)
    // Back up to just before the dot
    if (offset > 0 && content[offset - 1] === ".") offset--;
    if (offset > 0 && content[offset - 1] === ":") offset--;
    let start = offset;
    while (start > 0) {
        const c = content[start - 1];
        if (/[\w.]/.test(c)) { start--; } else { break; }
    }
    return content.substring(start, offset);
}

export function isComment(line: string): boolean {
    const trimmed = line.trimStart();
    return trimmed.startsWith("//") || trimmed.startsWith("--");
}

export function isInsideString(linePrefix: string): boolean {
    let inStr = false, quote = "";
    for (const ch of linePrefix) {
        if (!inStr && (ch === '"' || ch === "'")) { inStr = true; quote = ch; }
        else if (inStr && ch === quote) { inStr = false; quote = ""; }
    }
    return inStr;
}

export function isValuePosition(text: string): boolean {
    return /[=(,\[\-+*\/<>!]|\b(?:return|and|or)\b\s*$/.test(text);
}

export function isInsideImportString(linePrefix: string): string | null {
    const trimmed = linePrefix.trimStart();
    const m = trimmed.match(/^import\s+(["'])([^"']*)$/);
    return m ? m[2] : null;
}

export function collectProjectWrmFiles(workspacePath: string): string[] {
    const files: string[] = [];
    function walk(dir: string, prefix: string): void {
        try {
            for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
                if (entry.name.startsWith(".") || entry.name === "node_modules" || entry.name === "out" || entry.name === "target") continue;
                const fp = require("path").join(dir, entry.name);
                const rp = prefix ? prefix + "/" + entry.name : entry.name;
                if (entry.isDirectory()) {
                    walk(fp, rp);
                } else if (entry.name.endsWith(".wrm")) {
                    files.push(rp.replace(/\.wrm$/, ""));
                }
            }
        } catch {}
    }
    walk(require("path").join(workspacePath, "src"), "");
    return files;
}

import * as fs from "fs";

/**
 * Two-stage completion match score.
 * Returns 0=exact, 1=prefix, 2=substring (min 2 chars), -1=no match.
 * Used by both VSCode and LSP completion providers.
 */
export function matchScore(label: string, prefix: string): number {
    if (!prefix) return 1;
    const l = label.toLowerCase();
    const p = prefix.toLowerCase();
    if (l === p) return 0;
    if (l.startsWith(p)) return 1;
    if (p.length >= 2 && l.includes(p)) return 2;
    return -1;
}
