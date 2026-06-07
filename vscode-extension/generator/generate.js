const https = require("https");
const fs = require("fs");
const path = require("path");

const API_DUMP_URL =
  "https://raw.githubusercontent.com/CloneTrooper1019/Roblox-Client-Tracker/refs/heads/roblox/API-Dump.json";

const EXCLUDED_TAGS = new Set(["Deprecated", "Hidden", "NotScriptable"]);

const STANDARD_GLOBALS = {
  "game": { type: "DataModel", desc: "The root of the game hierarchy." },
  "workspace": { type: "Workspace", desc: "Shortcut for game.Workspace." },
  "script": { type: "Instance", desc: "Reference to the currently running script." },
  "shared": { type: "any", desc: "Shared table accessible by all scripts in the same context." },
  "plugin": { type: "any", desc: "Only available in Studio plugins." },
  "Enum": { type: "table", desc: "Global access to all Roblox Enums (e.g., Enum.Material)." },
};

const GLOBAL_FUNCTIONS = {
  "print": { params: [{ name: "...", type: "any" }], returns: "void", desc: "Prints arguments to the output." },
  "require": { params: [{ name: "module", type: "Instance" }], returns: "any", desc: "Loads and returns the result of a ModuleScript." },
  "warn": { params: [{ name: "...", type: "any" }], returns: "void", desc: "Prints a warning message to the output." },
  "error": { params: [{ name: "message", type: "string" }, { name: "level", type: "number", optional: true }], returns: "void", desc: "Terminates the last protected function." },
  "spawn": { params: [{ name: "func", type: "any" }, { name: "...", type: "any" }], returns: "void", desc: "Schedules func to run on the next step." },
  "delay": { params: [{ name: "time", type: "number" }, { name: "func", type: "any" }], returns: "void", desc: "Schedules func to run after time seconds." },
  "wait": { params: [{ name: "seconds", type: "number", optional: true }], returns: "void", desc: "Yields execution for the given time." },
  "time": { params: [], returns: "number", desc: "Returns the total time the game has been running." },
  "tick": { params: [], returns: "number", desc: "Returns the system time in seconds since epoch." },
  "typeof": { params: [{ name: "v", type: "any" }], returns: "string", desc: "Returns the Roblox-specific type of v." },
  "UserSettings": { params: [], returns: "any", desc: "Returns GlobalSettings for Studio preferences." },
  "xpcall": { params: [{ name: "f", type: "any" }, { name: "err", type: "any" }, { name: "...", type: "any" }], returns: "bool", desc: "Calls f in protected mode with custom error handler." },
  "pcall": { params: [{ name: "f", type: "any" }, { name: "...", type: "any" }], returns: "bool", desc: "Calls f in protected mode." },
  "assert": { params: [{ name: "v", type: "any" }, { name: "message", type: "string", optional: true }], returns: "any", desc: "Throws error if v is false/nil." },
  "gcinfo": { params: [], returns: "number", desc: "Returns the current memory usage in KB." },
};

const STANDARD_LIBRARIES = {
  "math": [
    { name: "abs", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "acos", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "asin", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "atan", params: [{ name: "y", type: "number" }, { name: "x", type: "number", optional: true }], returns: "number" },
    { name: "atan2", params: [{ name: "y", type: "number" }, { name: "x", type: "number" }], returns: "number" },
    { name: "ceil", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "clamp", params: [{ name: "x", type: "number" }, { name: "min", type: "number" }, { name: "max", type: "number" }], returns: "number" },
    { name: "cos", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "cosh", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "deg", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "exp", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "floor", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "fmod", params: [{ name: "x", type: "number" }, { name: "y", type: "number" }], returns: "number" },
    { name: "frexp", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "ldexp", params: [{ name: "m", type: "number" }, { name: "e", type: "number" }], returns: "number" },
    { name: "log", params: [{ name: "x", type: "number" }, { name: "base", type: "number", optional: true }], returns: "number" },
    { name: "log10", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "max", params: [{ name: "x", type: "number" }, { name: "...", type: "number" }], returns: "number" },
    { name: "min", params: [{ name: "x", type: "number" }, { name: "...", type: "number" }], returns: "number" },
    { name: "modf", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "noise", params: [{ name: "x", type: "number" }, { name: "y", type: "number", optional: true }, { name: "z", type: "number", optional: true }], returns: "number" },
    { name: "pow", params: [{ name: "x", type: "number" }, { name: "y", type: "number" }], returns: "number" },
    { name: "rad", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "random", params: [{ name: "m", type: "number", optional: true }, { name: "n", type: "number", optional: true }], returns: "number" },
    { name: "randomseed", params: [{ name: "x", type: "number" }], returns: "void" },
    { name: "round", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "sign", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "sin", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "sinh", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "sqrt", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "tan", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "tanh", params: [{ name: "x", type: "number" }], returns: "number" },
  ],
  "task": [
    { name: "cancel", params: [{ name: "thread", type: "any" }], returns: "void" },
    { name: "defer", params: [{ name: "func", type: "any" }, { name: "...", type: "any" }], returns: "void" },
    { name: "delay", params: [{ name: "time", type: "number" }, { name: "func", type: "any" }, { name: "...", type: "any" }], returns: "void" },
    { name: "desynchronize", params: [], returns: "void" },
    { name: "spawn", params: [{ name: "func", type: "any" }, { name: "...", type: "any" }], returns: "void" },
    { name: "synchronize", params: [], returns: "void" },
    { name: "wait", params: [{ name: "duration", type: "number", optional: true }], returns: "void" },
  ],
  "table": [
    { name: "clear", params: [{ name: "t", type: "any" }], returns: "void" },
    { name: "clone", params: [{ name: "t", type: "any" }], returns: "any" },
    { name: "concat", params: [{ name: "list", type: "any" }, { name: "sep", type: "string", optional: true }, { name: "i", type: "number", optional: true }, { name: "j", type: "number", optional: true }], returns: "string" },
    { name: "create", params: [{ name: "count", type: "number" }, { name: "value", type: "any", optional: true }], returns: "any" },
    { name: "find", params: [{ name: "t", type: "any" }, { name: "value", type: "any" }, { name: "init", type: "number", optional: true }], returns: "any" },
    { name: "freeze", params: [{ name: "t", type: "any" }], returns: "any" },
    { name: "insert", params: [{ name: "list", type: "any" }, { name: "pos/value", type: "any" }, { name: "value", type: "any", optional: true }], returns: "void" },
    { name: "move", params: [{ name: "a1", type: "any" }, { name: "f", type: "number" }, { name: "e", type: "number" }, { name: "t", type: "number" }, { name: "a2", type: "any", optional: true }], returns: "any" },
    { name: "pack", params: [{ name: "...", type: "any" }], returns: "any" },
    { name: "remove", params: [{ name: "list", type: "any" }, { name: "pos", type: "number", optional: true }], returns: "any" },
    { name: "sort", params: [{ name: "list", type: "any" }, { name: "comp", type: "any", optional: true }], returns: "void" },
    { name: "unpack", params: [{ name: "list", type: "any" }, { name: "i", type: "number", optional: true }, { name: "j", type: "number", optional: true }], returns: "any" },
  ],
  "string": [
    { name: "byte", params: [{ name: "s", type: "string" }, { name: "i", type: "number", optional: true }, { name: "j", type: "number", optional: true }], returns: "any" },
    { name: "char", params: [{ name: "...", type: "number" }], returns: "string" },
    { name: "find", params: [{ name: "s", type: "string" }, { name: "pattern", type: "string" }, { name: "init", type: "number", optional: true }, { name: "plain", type: "bool", optional: true }], returns: "any" },
    { name: "format", params: [{ name: "s", type: "string" }, { name: "...", type: "any" }], returns: "string" },
    { name: "gmatch", params: [{ name: "s", type: "string" }, { name: "pattern", type: "string" }], returns: "any" },
    { name: "gsub", params: [{ name: "s", type: "string" }, { name: "pattern", type: "string" }, { name: "repl", type: "any" }, { name: "n", type: "number", optional: true }], returns: "string" },
    { name: "len", params: [{ name: "s", type: "string" }], returns: "number" },
    { name: "lower", params: [{ name: "s", type: "string" }], returns: "string" },
    { name: "match", params: [{ name: "s", type: "string" }, { name: "pattern", type: "string" }, { name: "init", type: "number", optional: true }], returns: "any" },
    { name: "rep", params: [{ name: "s", type: "string" }, { name: "n", type: "number" }, { name: "sep", type: "string", optional: true }], returns: "string" },
    { name: "reverse", params: [{ name: "s", type: "string" }], returns: "string" },
    { name: "split", params: [{ name: "s", type: "string" }, { name: "sep", type: "string" }], returns: "any" },
    { name: "sub", params: [{ name: "s", type: "string" }, { name: "i", type: "number" }, { name: "j", type: "number", optional: true }], returns: "string" },
    { name: "upper", params: [{ name: "s", type: "string" }], returns: "string" },
  ],
  "bit32": [
    { name: "arshift", params: [{ name: "x", type: "number" }, { name: "disp", type: "number" }], returns: "number" },
    { name: "band", params: [{ name: "...", type: "number" }], returns: "number" },
    { name: "bnot", params: [{ name: "x", type: "number" }], returns: "number" },
    { name: "bor", params: [{ name: "...", type: "number" }], returns: "number" },
    { name: "btest", params: [{ name: "...", type: "number" }], returns: "bool" },
    { name: "bxor", params: [{ name: "...", type: "number" }], returns: "number" },
    { name: "extract", params: [{ name: "x", type: "number" }, { name: "field", type: "number" }, { name: "width", type: "number", optional: true }], returns: "number" },
    { name: "lrotate", params: [{ name: "x", type: "number" }, { name: "disp", type: "number" }], returns: "number" },
    { name: "lshift", params: [{ name: "x", type: "number" }, { name: "disp", type: "number" }], returns: "number" },
    { name: "replace", params: [{ name: "x", type: "number" }, { name: "v", type: "number" }, { name: "field", type: "number" }, { name: "width", type: "number", optional: true }], returns: "number" },
    { name: "rrotate", params: [{ name: "x", type: "number" }, { name: "disp", type: "number" }], returns: "number" },
    { name: "rshift", params: [{ name: "x", type: "number" }, { name: "disp", type: "number" }], returns: "number" },
  ],
  "utf8": [
    { name: "char", params: [{ name: "...", type: "number" }], returns: "string" },
    { name: "codepoint", params: [{ name: "s", type: "string" }, { name: "i", type: "number", optional: true }, { name: "j", type: "number", optional: true }], returns: "any" },
    { name: "codes", params: [{ name: "s", type: "string" }], returns: "any" },
    { name: "len", params: [{ name: "s", type: "string" }, { name: "i", type: "number", optional: true }, { name: "j", type: "number", optional: true }], returns: "number" },
    { name: "nfcnormalize", params: [{ name: "s", type: "string" }], returns: "string" },
    { name: "nfdnormalize", params: [{ name: "s", type: "string" }], returns: "string" },
    { name: "offset", params: [{ name: "s", type: "string" }, { name: "n", type: "number" }, { name: "i", type: "number", optional: true }], returns: "number" },
  ],
  "coroutine": [
    { name: "close", params: [{ name: "co", type: "any" }], returns: "void" },
    { name: "create", params: [{ name: "f", type: "any" }], returns: "any" },
    { name: "isyieldable", params: [], returns: "bool" },
    { name: "resume", params: [{ name: "co", type: "any" }, { name: "...", type: "any" }], returns: "any" },
    { name: "running", params: [], returns: "any" },
    { name: "status", params: [{ name: "co", type: "any" }], returns: "string" },
    { name: "wrap", params: [{ name: "f", type: "any" }], returns: "any" },
    { name: "yield", params: [{ name: "...", type: "any" }], returns: "any" },
  ],
  "os": [
    { name: "clock", params: [], returns: "number" },
    { name: "date", params: [{ name: "format", type: "string", optional: true }, { name: "time", type: "number", optional: true }], returns: "string" },
    { name: "difftime", params: [{ name: "t2", type: "number" }, { name: "t1", type: "number" }], returns: "number" },
    { name: "time", params: [{ name: "table", type: "any", optional: true }], returns: "number" },
  ],
  "buffer": [
    { name: "create", params: [{ name: "size", type: "number" }], returns: "any" },
    { name: "fromstring", params: [{ name: "str", type: "string" }], returns: "any" },
    { name: "tostring", params: [{ name: "buf", type: "any" }], returns: "string" },
  ],
  "debug": [
    { name: "info", params: [{ name: "thread", type: "any" }, { name: "level", type: "number" }, { name: "options", type: "string" }], returns: "any" },
    { name: "traceback", params: [{ name: "thread", type: "any", optional: true }, { name: "message", type: "string", optional: true }, { name: "level", type: "number", optional: true }], returns: "string" },
  ],
};

function mapType(apiType) {
  if (!apiType) return "void";
  const name = apiType.Name || apiType;
  const map = {
    "string": "string",
    "int": "number",
    "int64": "number",
    "float": "number",
    "double": "number",
    "number": "number",
    "bool": "bool",
    "boolean": "bool",
    "void": "void",
    "nil": "nil",
    "any": "any",
    "Instance": "Instance",
    "Array": "any",
    "Tuple": "any",
    "Dictionary": "any",
    "Variant": "any",
    "Objects": "any",
    "ProtectedString": "string",
    "Content": "string",
    "BinaryString": "string",
  };
  return map[name] || name;
}

function hasTag(member, tag) {
  const tags = member.Tags || member.Security?.Tags || [];
  return tags.includes(tag);
}

function hasExcludedTag(member) {
  const tags = member.Tags || [];
  return tags.some((t) => EXCLUDED_TAGS.has(t));
}

function mapParams(apiParams) {
  if (!apiParams) return [];
  return apiParams.map((p) => ({
    name: p.Name || "arg",
    type: mapType(p.Type),
    ...(p.Default ? { optional: true } : {}),
  }));
}

function generateWold(apiDump) {
  const types = [];
  const enums = [];
  const services = [];

  // Process Classes
  for (const cls of apiDump.Classes || []) {
    if (hasExcludedTag(cls)) continue;

    const members = cls.Members || [];
    const properties = [];
    const methods = [];
    const events = [];

    for (const m of members) {
      if (hasExcludedTag(m)) continue;

      const desc = m.Description || "";

      if (m.MemberType === "Property") {
        const security = m.Security || {};
        const canRead = security.Read !== "None";
        const canWrite = security.Write !== "None";
        properties.push({
          name: m.Name,
          type: mapType(m.ValueType),
          rw: canRead && canWrite,
          description: desc,
        });
      } else if (m.MemberType === "Function") {
        methods.push({
          name: m.Name,
          params: mapParams(m.Parameters),
          returns: mapType(m.ReturnValue?.Type || m.ReturnType),
          description: desc,
        });
      } else if (m.MemberType === "Event") {
        events.push({
          name: m.Name,
          params: mapParams(m.Parameters),
          description: desc,
        });
      } else if (m.MemberType === "Callback") {
        methods.push({
          name: m.Name,
          params: mapParams(m.Parameters),
          returns: mapType(m.ReturnType),
          description: desc,
        });
      }
    }

    types.push({
      name: cls.Name,
      description: cls.Description || "",
      extends: cls.Superclass || null,
      tags: cls.Tags || [],
      properties,
      methods,
      events,
    });

    // Mark services
    if ((cls.Tags || []).includes("Service")) {
      services.push({
        name: cls.Name,
        className: cls.Name,
        description: `Service: ${cls.Description || cls.Name}`,
      });
    }
  }

  // Process Enums
  for (const en of apiDump.Enums || []) {
    enums.push({
      name: en.Name,
      items: (en.Items || []).map((i) => i.Name),
      description: en.Description || "",
    });
  }

  // Build globals list
  const globals = [];
  for (const [name, info] of Object.entries(STANDARD_GLOBALS)) {
    globals.push({
      name,
      type: info.type,
      description: info.desc,
    });
  }

  // Build global functions
  const funcs = [];
  for (const [name, info] of Object.entries(GLOBAL_FUNCTIONS)) {
    funcs.push({
      name,
      params: info.params,
      returns: info.returns,
      description: info.desc,
    });
  }

  // Standard libraries as types
  for (const [libName, methods] of Object.entries(STANDARD_LIBRARIES)) {
    types.push({
      name: libName,
      description: `${libName} standard library`,
      extends: null,
      tags: [],
      properties: [],
      methods: methods.map((m) => ({
        name: m.name,
        params: m.params,
        returns: m.returns,
        description: m.description || "",
      })),
      events: [],
    });

    globals.push({
      name: libName,
      type: libName,
      description: `${libName} standard library`,
    });
  }

  const wold = {
    version: 1,
    globals,
    functions: funcs,
    types,
    enums,
    services,
  };

  return wold;
}

function download(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (res) => {
        let data = "";
        res.on("data", (chunk) => (data += chunk));
        res.on("end", () => resolve(data));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function main() {
  console.log("Downloading Roblox API dump...");

  let apiDump;
  try {
    const data = await download(API_DUMP_URL);
    apiDump = JSON.parse(data);
    console.log(
      `Loaded ${apiDump.Classes?.length || 0} classes, ${apiDump.Enums?.length || 0} enums`
    );
  } catch (e) {
    console.error("Failed to download/parse API dump:", e.message);
    process.exit(1);
  }

  const wold = generateWold(apiDump);
  console.log(
    `Generated ${wold.types.length} types, ${wold.enums.length} enums, ${wold.globals.length} globals, ${wold.functions.length} functions`
  );

  const outDir = path.join(__dirname, "..", "generated");
  if (!fs.existsSync(outDir)) {
    fs.mkdirSync(outDir, { recursive: true });
  }

  const outPath = path.join(outDir, "roblox.wold");
  fs.writeFileSync(outPath, JSON.stringify(wold, null, 2), "utf-8");
  console.log(`Wrote ${outPath} (${(fs.statSync(outPath).size / 1024 / 1024).toFixed(1)} MB)`);
}

main().catch(console.error);
