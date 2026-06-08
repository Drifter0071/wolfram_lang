import * as fs from "fs";
import * as path from "path";

export interface WoldGlobal { name: string; type: string; description: string; }
export interface WoldFunction { name: string; params: WoldParam[]; returns: string; description: string; }
export interface WoldParam { name: string; type: string; }
export interface WoldProperty { name: string; type: string; rw: boolean; description: string; }
export interface WoldMethod { name: string; params: WoldParam[]; returns: string; description: string; }
export interface WoldType { name: string; description: string; extends?: string | null; tags: string[]; properties: WoldProperty[]; methods: WoldMethod[]; events: any[]; }
export interface WoldEnum { name: string; items: string[]; description: string; }
export interface WoldFile { version: number; globals: WoldGlobal[]; functions: WoldFunction[]; types: WoldType[]; enums: WoldEnum[]; services: any[]; }

export interface SymbolInfo {
    name: string;
    kind: string;
    access: string;
    location: { line: number; column: number; endLine: number; endColumn: number };
    params: string[];
    fields: string[];
}

export interface ImportInfo {
    path: string;
    alias: string;
}

export interface ParsedDocument {
    symbols: SymbolInfo[];
    imports: ImportInfo[];
    scope: Map<string, string>;
    lineCount: number;
}

export class Bindings {
    globals = new Map<string, WoldGlobal>();
    functions = new Map<string, WoldFunction>();
    private types = new Map<string, WoldType>();
    enums = new Map<string, WoldEnum>();

    load(bindingsDir: string): void {
        const p = path.join(bindingsDir, "generated", "roblox.wold");
        if (!fs.existsSync(p)) {
            console.error("[wolfram-lsp] no roblox.wold at", p);
            return;
        }
        try {
            const raw = fs.readFileSync(p, "utf-8");
            const file: WoldFile = JSON.parse(raw);
            for (const g of file.globals) this.globals.set(g.name.toLowerCase(), g);
            for (const f of file.functions) this.functions.set(f.name.toLowerCase(), f);
            for (const t of file.types) this.types.set(t.name.toLowerCase(), t);
            for (const e of file.enums) this.enums.set(e.name.toLowerCase(), e);
        } catch (e: any) {
            console.error("[wolfram-lsp] bindings load error:", e.message);
        }
    }

    getType(name: string): WoldType | undefined { return this.types.get(name.toLowerCase()); }
    getGlobal(name: string): WoldGlobal | undefined { return this.globals.get(name.toLowerCase()); }
    getFunction(name: string): WoldFunction | undefined { return this.functions.get(name.toLowerCase()); }

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
