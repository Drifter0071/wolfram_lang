import * as fs from "fs";
import * as path from "path";
import {
  WoldFile,
  WoldType,
  WoldGlobal,
  WoldFunction,
  WoldEnum,
  WoldService,
  WoldMethod,
  WoldProperty,
} from "./types";

export interface TypedIdentifier {
  name: string;
  type: string;
  description?: string;
  kind: "global" | "function" | "service" | "local" | "param";
  params?: { name: string; type: string }[];
}

export interface MemberAccess {
  type: string;
  methods: WoldMethod[];
  properties: WoldProperty[];
  inheritedMethods: { name: string; from: string }[];
  inheritedProperties: { name: string; from: string }[];
}

let bindings: WoldFile | null = null;
let typeIndex: Map<string, WoldType> = new Map();

export function loadBuiltinBindings(extensionPath: string): void {
  const woldPath = path.join(extensionPath, "generated", "roblox.wold");
  try {
    const raw = fs.readFileSync(woldPath, "utf-8");
    const file = JSON.parse(raw) as WoldFile;
    mergeBindings(file);
  } catch (e) {
    console.error("Failed to load built-in Roblox bindings:", e);
  }
}

export function loadWorkspaceBindings(workspaceRoot: string): void {
  if (!workspaceRoot) return;
  try {
    const entries = fs.readdirSync(workspaceRoot, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.isFile() && entry.name.endsWith(".wold")) {
        const filePath = path.join(workspaceRoot, entry.name);
        const raw = fs.readFileSync(filePath, "utf-8");
        const file = JSON.parse(raw) as WoldFile;
        mergeBindings(file);
      }
    }
  } catch (e) {
    console.error("Failed to load workspace bindings:", e);
  }
}

function mergeBindings(file: WoldFile): void {
  if (!bindings) {
    bindings = file;
  } else {
    bindings.globals.push(...file.globals);
    bindings.functions.push(...file.functions);
    bindings.types.push(...file.types);
    bindings.enums.push(...file.enums);
    bindings.services.push(...file.services);
  }

  for (const t of file.types) {
    typeIndex.set(t.name.toLowerCase(), t);
  }
}

export function getGlobals(): WoldGlobal[] {
  return bindings?.globals ?? [];
}

export function getFunctions(): WoldFunction[] {
  return bindings?.functions ?? [];
}

export function getServices(): WoldService[] {
  return bindings?.services ?? [];
}

export function getEnums(): WoldEnum[] {
  return bindings?.enums ?? [];
}

export function getType(name: string): WoldType | null {
  const lower = name.toLowerCase();
  const direct = typeIndex.get(lower);
  if (direct) return direct;

  if (!bindings) return null;
  return bindings.types.find((t) => t.name.toLowerCase() === lower) ?? null;
}

function getAllMethods(type: WoldType): WoldMethod[] {
  const methods = [...type.methods];
  if (type.extends) {
    const parent = getType(type.extends);
    if (parent) {
      methods.push(...getAllMethods(parent));
    }
  }
  return methods;
}

function getAllProperties(type: WoldType): WoldProperty[] {
  const props = [...type.properties];
  if (type.extends) {
    const parent = getType(type.extends);
    if (parent) {
      props.push(...getAllProperties(parent));
    }
  }
  return props;
}

export function getMemberAccess(typeName: string): MemberAccess | null {
  const type = getType(typeName);
  if (!type) return null;

  const methods = getAllMethods(type);
  const properties = getAllProperties(type);

  return {
    type: type.name,
    methods,
    properties,
    inheritedMethods: [],
    inheritedProperties: [],
  };
}

export function getMethodReturnType(
  typeName: string,
  methodName: string
): string | null {
  const type = getType(typeName);
  if (!type) return null;

  const method = type.methods.find(
    (m) => m.name.toLowerCase() === methodName.toLowerCase()
  );
  if (method) return method.returns;

  if (type.extends) {
    return getMethodReturnType(type.extends, methodName);
  }
  return null;
}

export function inferTypeFromSource(
  name: string,
  source: string
): string | null {
  // Pattern: local name = Instance.new("ClassName")
  const newPattern = new RegExp(
    `local\\s+${escapeRegex(name)}\\s*=\\s*(\\w+(?:\\.\\w+)*)\\.new\\(`,
    "i"
  );
  const newMatch = source.match(newPattern);
  if (newMatch) {
    return newMatch[1];
  }

  // Pattern: local name = game:GetService("ServiceName")
  const svcPattern = new RegExp(
    `local\\s+${escapeRegex(name)}\\s*=\\s*\\w+(?::\\w+)*:GetService\\(\\s*["'](\\w+)["']`,
    "i"
  );
  const svcMatch = source.match(svcPattern);
  if (svcMatch) {
    return svcMatch[1];
  }

  // Pattern: local name = expression:Method(...)
  const methodPattern = new RegExp(
    `local\\s+${escapeRegex(name)}\\s*=\\s*(\\w+(?:\\.\\w+)*):(\\w+)\\(`,
    "i"
  );
  const methodMatch = source.match(methodPattern);
  if (methodMatch) {
    const objName = methodMatch[1];
    const methodName = methodMatch[2];

    // Check if objName is a known global
    const global = (bindings?.globals ?? []).find(
      (g) => g.name.toLowerCase() === objName.toLowerCase()
    );
    if (global) {
      const retType = getMethodReturnType(global.type, methodName);
      if (retType && retType !== "void" && retType !== "nil") {
        return retType;
      }
    }

    // Check if objName is a local with known type
    const localType = inferTypeFromSource(objName, source);
    if (localType) {
      const retType = getMethodReturnType(localType, methodName);
      if (retType && retType !== "void" && retType !== "nil") {
        return retType;
      }
    }
  }

  // Check pattern: for x in y do - extract x type from compiler symbols
  // (handled separately via the analyze output)

  return null;
}

export function resolveType(name: string, source: string): string | null {
  // Check globals first
  const global = (bindings?.globals ?? []).find(
    (g) => g.name.toLowerCase() === name.toLowerCase()
  );
  if (global) return global.type;

  // Check services
  const service = (bindings?.services ?? []).find(
    (s) => s.name.toLowerCase() === name.toLowerCase()
  );
  if (service) return service.className;

  // Infer from source
  return inferTypeFromSource(name, source);
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function getExpressionBeforeDot(
  text: string,
  cursorOffset: number
): string | null {
  if (cursorOffset <= 0) return null;
  const char = text[cursorOffset - 1];
  if (char !== "." && char !== ":") return null;

  // Walk back to find the start of the expression
  let i = cursorOffset - 2;
  while (i >= 0 && /[\w.]/.test(text[i])) {
    i--;
  }
  i++;

  const expr = text.substring(i, cursorOffset - 1);
  if (!expr || !/^[a-zA-Z_]\w*(?:\.[a-zA-Z_]\w*)*$/.test(expr)) return null;

  return expr;
}

export function resolveExpressionType(
  expr: string,
  source: string
): string | null {
  // Direct global/function name
  if (!expr.includes(".")) {
    return resolveType(expr, source);
  }

  // Chained access: obj.method.chain - resolve from left to right
  const parts = expr.split(".");
  let currentType = resolveType(parts[0], source);
  if (!currentType) return null;

  for (let i = 1; i < parts.length; i++) {
    const member = getMemberAccess(currentType);
    if (!member) return null;

    const prop = member.properties.find(
      (p) => p.name.toLowerCase() === parts[i].toLowerCase()
    );
    if (prop) {
      currentType = prop.type;
      continue;
    }

    const method = member.methods.find(
      (m) => m.name.toLowerCase() === parts[i].toLowerCase()
    );
    if (method) {
      currentType = method.returns;
      continue;
    }

    return null;
  }

  return currentType;
}
