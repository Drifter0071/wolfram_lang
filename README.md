# Wolfram

A Python/C#-inspired programming language that transpiles to [Luau](https://luau.org).
Designed for Roblox game development with first-class VS Code support and seamless [Rojo](https://rojo.space/) integration.

## Why Wolfram?

Luau is powerful but verbose. Wolfram adds familiar syntax from Python and C# — bracket blocks, access modifiers, classes, structs, enums — while compiling to clean, idiomatic Luau that Rojo can sync directly into Roblox Studio.

## Language Features

| Feature | Wolfram | Luau |
|---------|---------|------|
| Blocks | `{ }` braces | `then … end` / `do … end` |
| Classes | `class Name { }` + `public`/`private` | Manual metatable setup |
| Structs | `struct Name { x, y, z }` | Manual table constructors |
| Enums | `enum State { Lobby, Playing }` | `table.freeze({…})` |
| Access modifiers | `public` / `private` | Convention only |
| If statements | `if (cond) { }` | `if cond then … end` |
| While loops | `while (cond) { }` | `while cond do … end` |
| For loops | `for x in items { }` | `for _, x in ipairs(items) do … end` |
| Type inference | Auto-detects arrays → `ipairs`, tables → `pairs` | Must manually choose |
| Imports | `import "./lib" as lib` | `local lib = require(…)` |
| Ternary | `cond ? a : b` | `if cond then a else b` (statement only) |
| F-strings | `f"Hello {name}"` | `"Hello " .. name` |
| Semicolons | Optional | Optional |
| Comments | `--` | `--` |

## Compiler

```bash
# Build from source (requires Rust)
cargo build --release

# Transpile a single file
wolfram src/main.wol              # → out/main.luau

# Transpile a project directory
wolfram src/                      # → out/**/*.luau

# Watch mode — auto-transpile on every save
wolfram --watch src/

# Analyze mode — JSON diagnostics + symbols for tooling
wolfram --analyze src/main.wol
```

## VS Code Extension

### Installation

```bash
cd vscode-extension
npm install
npx tsc -p tsconfig.json
```

Then press `F5` in VS Code to launch an Extension Development Host, or run `npx vsce package` to create a `.vsix` for installation.

### Features

- **Syntax highlighting** — TextMate grammar for `.wol` files
- **Intellisense** — Keyword snippets, local symbols, function signatures

- **Roblox API autocomplete** — Full Roblox class/type/enum bindings (849 types, 585 enums)
- **Dot/colon completion** — `part:` shows Instance methods, `game.` shows DataModel properties
- **Type tracking** — Infers variable types from assignments and method return values
- **Diagnostics** — Compiler errors mapped to editor ranges, debounced 500ms
- **Hover info** — Type signatures, parameter lists, documentation
- **Go to definition** — Jump to symbol declarations within the file
- **Document symbols** — Outline view of classes, functions, enums, structs
- **Watch server** — Auto-transpile on every save via status bar toggle
- **Project template** — Scaffolds a Rojo-ready project with client/shared/server structure

### Commands

| Command | Keybinding |
|---------|-----------|
| `Wolfram: New Project` | — |
| `Wolfram: Start Watch Server` | — |
| `Wolfram: Stop Watch Server` | — |
| `Wolfram: Compile Current File` | `F5` |

### Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `wolfram.compilerPath` | `wolfram` | Path to the compiler binary |
| `wolfram.watchOnOpen` | `true` | Auto-start watch server on `.wol` workspace |
| `wolfram.outputDir` | `out` | Output directory for compiled `.luau` files |

## Roblox + Rojo Integration

Wolfram is built for Rojo's file-based workflow. The `src/` directory structure is mirrored identically in `out/`, preserving every subdirectory and filename. Rojo reads `default.project.json` and syncs the compiled `.luau` files directly into Roblox Studio.

```
my-game/
├── wolfram.toml              Compiler + Roblox config
├── default.project.json      Rojo project manifest
├── src/
│   ├── client/
│   │   └── main.client.wol   →  out/client/main.client.luau
│   ├── shared/
│   │   └── utils.shared.wol  →  out/shared/utils.shared.luau
│   └── server/
│       └── main.server.wol   →  out/server/main.server.luau
└── out/                      Auto-generated, gitignored
    ├── client/
    ├── shared/
    └── server/
```

### How it works

1. Write `.wol` files in `src/` under `client/`, `shared/`, or `server/`
2. The watcher transpiles to identically-structured `out/`
3. Rojo reads `default.project.json` and syncs `out/` into Roblox Studio
4. `.client.luau` / `.server.luau` file naming tells Roblox which runtime to use
5. `init.luau` files become folder-named instances (Rojo's `init` convention)
6. Nested subdirectories are fully supported (e.g. `src/client/ui/components/`)

### wolfram.toml

```toml
[[roblox.mappings]]
source = "src/**/*.wol"
target = "out"
```

### default.project.json

```json
{
  "name": "my-game",
  "tree": {
    "$className": "DataModel",
    "ReplicatedStorage": {
      "$className": "ReplicatedStorage",
      "Shared": {
        "$path": "out/shared"
      }
    },
    "ServerScriptService": {
      "$className": "ServerScriptService",
      "Server": {
        "$path": "out/server"
      }
    },
    "StarterPlayer": {
      "$className": "StarterPlayer",
      "StarterPlayerScripts": {
        "$className": "StarterPlayerScripts",
        "Client": {
          "$path": "out/client"
        }
      }
    }
  }
}
```

## Language Reference

### Variables

```
local x = 42
x = 100
local name             -- nil by default
```

### Functions

```
function greet(name) {
    return "Hello, " .. name
}

-- With access modifiers
public function get_player(id) {
    return players[id]
}

private function internal_calc() {
    return 0
}
```

### Control Flow

```
if (score > 100) {
    print("Winner!")
} else if (score > 50) {
    print("Almost")
} else {
    print("Keep trying")
}

while (timer > 0) {
    timer = timer - 1
}

for i in range(1, 10) {
    print(i)
}

for player in players {
    player:Kick()
}
```

### Classes

```
class PlayerData {
    local score = 0
    local name = ""

    public function init(n, s) {
        name = n
        score = s
    }

    public function add_score(points) {
        score = score + points
    }

    private function recalc() {
        -- internal logic
    }
}

-- Usage
local data = PlayerData.new("Player1", 100)
data:add_score(50)
```

### Structs

```
struct Position {
    x, y, z
}

local pos = Position.new(1, 2, 3)
print(pos.x)
```

### Enums

```
enum GameState {
    Lobby,
    Playing,
    Ended
}

local state = GameState.Lobby
```

### Imports

```
import "./player_data" as PlayerData

local data = PlayerData.new("Test", 0)
```

### Ternary

```
local status = score > 50 ? "Pass" : "Fail"
```

### F-Strings

```
local msg = f"Player {name} has {score} points"
```

## Custom API Bindings (`.wold`)

Wolfram supports user-definable type declarations for custom libraries.
Drop a `.wold` file in your project root — the LSP auto-discovers it on workspace open.

```json
{
  "version": 1,
  "types": [
    {
      "name": "MyLibrary",
      "methods": [
        { "name": "do_thing", "params": [{ "name": "x", "type": "number" }], "returns": "string" }
      ]
    }
  ],
  "globals": [
    { "name": "MY_LIB", "type": "MyLibrary" }
  ]
}
```

This adds `MY_LIB` to autocomplete, with `:do_thing(x)` showing the correct parameter and return type.

The built-in Roblox bindings are auto-generated from the [official API dump](https://github.com/CloneTrooper1019/Roblox-Client-Tracker). To regenerate:

```bash
node vscode-extension/generator/generate.js
```

## Project Structure

```
wolfram_lang/
├── Cargo.toml                   Rust crate manifest
├── Cargo.lock                   Dependency lockfile
├── .gitignore
├── src/                         Compiler source
│   ├── main.rs                  CLI entry point (dispatch)
│   ├── lib.rs                   Library API (transpile, analyze)
│   ├── lexer.rs                 Tokenizer (logos-based)
│   ├── parser.rs                Recursive descent parser
│   ├── ast.rs                   AST node definitions with source spans
│   ├── generator.rs             AST → Luau code generator
│   ├── cli.rs                   CLI helpers (transpile_project, collect_files)
│   ├── watch.rs                 File watcher (notify-based)
│   ├── analyze.rs               JSON symbol/diagnostic extractor for LSP
│   └── roblox_config.rs         wolfram.toml parser + import resolver
└── vscode-extension/            VS Code extension
    ├── package.json
    ├── tsconfig.json
    ├── language-configuration.json
    ├── src/
    │   ├── extension.ts         Activation, LSP client, watch server lifecycle
    │   ├── server.ts            LSP server: completion, hover, diagnostics, symbols, go-to-def
    │   ├── bindings.ts          .wold type bindings loader + query engine
    │   ├── types.ts             .wold format TypeScript types
    │   └── syntaxes/
    │       └── wolfram.tmLanguage.json   TextMate grammar
    ├── generator/
    │   └── generate.js          Roblox API dump → .wold converter
    └── templates/
        └── new-project/         Project scaffold (client/shared/server + Rojo config)
```

## License

MIT
