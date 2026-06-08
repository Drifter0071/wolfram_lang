# Wolfram

A Python/C#-inspired language that compiles to [Luau](https://luau.org) — purpose-built for Roblox game development with first-class VS Code support, Rojo integration, and intelligent Roblox instance-path resolution.

---

## Quick Start

```bash
cargo build --release                           # Build compiler

wolfram my-project/                             # Transpile project
wolfram src/main.wrm                            # Transpile single file
wolfram --watch src/                            # Watch mode
wolfram --analyze src/main.wrm                  # JSON diagnostics
```

## Why Wolfram?

Wolfram gives Roblox developers the syntax they know from Python and C# — braces, classes, access modifiers, f-strings — while compiling to **clean, idiomatic Luau** that runs natively in Roblox Studio. No runtime overhead, no foreign patterns.

### Before → After

<table>
<tr><th>Wolfram</th><th>Luau</th></tr>
<tr>
<td>

```js
import "../shared/HealthSystem"
  as HealthSystemClass

local health = HealthSystemClass.new(100)

local disconnect = health
  .HealthChanged:Connect(
    function(current, max) {
      print(f"Health: {current}/{max}")
      if (current <= 0) {
        print("Player died!")
        disconnect()
      }
    }
  )

health:takeDamage(20)
```

</td>
<td>

```lua
local RepStorage = game:GetService("ReplicatedStorage")
local HealthSystemClass = require(RepStorage.Shared.HealthSystem)

local health = HealthSystemClass.new(100)

local disconnect = health.HealthChanged:Connect(function(current, max)
    print(`Health: {current}/{max}`)
    if current <= 0 then
        print("Player died!")
        disconnect()
    end
end)

health:takeDamage(20)
```

</td>
</tr>
</table>

---

## Roblox Instance Path Resolution

Wolfram understands Roblox's object hierarchy. No more hand-writing `game:GetService()` chains — the compiler resolves everything from your `wolfram.toml` deployment map.

### `wolfram.toml` — Deployment Map

```toml
[deployment]
"src/shared"    = "ReplicatedStorage.Shared"
"src/server"    = "ServerScriptService.ServerModules"
"src/client/ui" = "StarterPlayer.StarterPlayerScripts.UI"
```

### Four Resolution Strategies

| Strategy | When | Example Output |
|----------|------|---------------|
| **Cross-Service** | Source & target in different Roblox services | `require(ReplicatedStorage.Shared.Utils)` with auto-generated `game:GetService("ReplicatedStorage")` |
| **Sibling** | Same service, same parent instance | `require(script.Parent.Utils)` |
| **Deep Nested** | Same service, different sub-paths | `require(ServerScriptService.Modules.Sub.Util)` |
| **StarterPlayer** | Scripts that move at runtime | Always `script.Parent` chain (runtime-safe) |

### Resolution Examples

**Server requiring Shared** (cross-service):
```js
// src/server/GameLoop.wrm
import "../shared/Config" as Config
```
```lua
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Config = require(ReplicatedStorage.Shared.Config)
```

**Client UI sibling** (StarterPlayer):
```js
// src/client/ui/MainMenu.wrm
import "./Button" as Button
```
```lua
local Button = require(script.Parent.Button)
```

**Shared sibling** (same container):
```js
// src/shared/GameModule.wrm
import "./MathUtils" as MathUtils
```
```lua
local MathUtils = require(script.Parent.MathUtils)
```

---

## Language Features

| Feature | Wolfram | Compiles to |
|---------|---------|-------------|
| **Blocks** | `{ }` braces | `then … end` / `do … end` |
| **Classes** | `class Name { }` with `public`/`private` | Metatable-based OOP |
| **Structs** | `struct Vec3 { x, y, z }` | Table constructor factories |
| **Enums** | `enum State { Lobby, Playing }` | `table.freeze({…})` |
| **Access mods** | `public` / `private` keywords | Private storage via `__private_*` tables |
| **If/elif/else** | `if (cond) { }` | `if cond then … end` |
| **While** | `while (cond) { }` | `while cond do … end` |
| **For-in** | `for x in items { }` | Auto-detects `ipairs` vs `pairs` |
| **For-range** | `for i in range(0, 10, 2)` | `for i = 0, 10 - 1, 2 do` |
| **Ternary** | `cond ? a : b` | `if cond then a else b` (expression) |
| **F-strings** | `f"Hello {name}"` | `` `Hello {name}` `` (Luau template literals) |
| **Imports** | `import "./lib" as lib` | `local lib = require(…)` with path resolution |
| **Type annotations** | `function f(x: number)` | Preserved in output for Luau type checker |
| **Comments** | `-- comment` | `-- comment` |
| **Decorators** | `@export variable` | Metadata for tooling |
| **Try/catch** | `try { } catch(err) { }` | `pcall`-based error handling |
| **List comprehensions** | `[x for x in items if x > 0]` | `table.insert` loop |
| **Logical operators** | `and` / `or` / `not` | Native Lua operators |

---

## VS Code Extension

### Features

- **Syntax highlighting** — TextMate grammar for `.wrm` files
- **Intellisense** — Keyword snippets, local symbols, function signatures
- **Roblox API autocomplete** — 849 types, 585 enums from official API dump
- **Dot/colon awareness** — `part:` shows Instance methods, `game.` shows DataModel properties
- **Type inference** — Infers types from assignments and method returns
- **Diagnostics** — Compiler errors mapped to editor, debounced 500ms
- **Hover info** — Type signatures, parameters, documentation
- **Go to definition** — Jump to symbol declarations
- **Document symbols** — Outline view (classes, functions, enums, structs)
- **Watch mode** — Auto-transpile on every save via status bar toggle
- **Project template** — One-command scaffold with client/shared/server + Rojo config

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
| `wolfram.compilerPath` | `""` | Path to `wolfram.exe` |
| `wolfram.watchOnOpen` | `true` | Auto-start watcher on `.wrm` workspace |
| `wolfram.outputDir` | `"out"` | Output directory for `.luau` files |

---

## Project Structure

```
my-game/
├── wolfram.toml                 Compiler config + deployment map
├── default.project.json         Rojo project manifest
├── src/
│   ├── client/                  StarterPlayer scripts
│   │   └── ui/
│   │       └── MainMenu.wrm
│   ├── shared/                  ReplicatedStorage modules
│   │   ├── Logger.wrm
│   │   └── MathUtils.wrm
│   └── server/                  ServerScriptService scripts
│       └── ServerMain.wrm
└── out/                         Auto-generated Luau (gitignored)
    ├── client/
    ├── shared/
    └── server/
```

### How it works

1. Write `.wrm` files in `src/` under `client/`, `shared/`, or `server/`
2. The watcher/compiler reads `wolfram.toml` to resolve Roblox instance paths
3. Service declarations (`game:GetService`) are auto-injected at the top of each output
4. Transpiled `.luau` goes to identically-structured `out/`
5. Rojo reads `default.project.json` and syncs `out/` into Roblox Studio
6. `.client.wrm` / `.server.wrm` suffixes control script type (LocalScript vs Script)
7. Nested subdirectories are fully supported

### `default.project.json`

```json
{
  "name": "my-game",
  "tree": {
    "$className": "DataModel",
    "ReplicatedStorage": {
      "$className": "ReplicatedStorage",
      "Shared": { "$path": "out/shared" }
    },
    "ServerScriptService": {
      "$className": "ServerScriptService",
      "Server": { "$path": "out/server" }
    },
    "StarterPlayer": {
      "$className": "StarterPlayer",
      "StarterPlayerScripts": {
        "$className": "StarterPlayerScripts",
        "Client": { "$path": "out/client" }
      }
    }
  }
}
```

---

## Language Reference

### Variables & Assignment

```js
local x = 42
local name             // nil by default
x = 100
x += 1                 // compound assignment
```

### Functions

```js
function greet(name: string): string {
    return f"Hello, {name}"
}

public function getPlayer(id: number) {
    return players[id]
}

private function internalCalc() {
    return 0
}
```

### Classes

```js
class PlayerData {
    local score = 0
    local name = ""

    public function init(n: string, s: number) {
        name = n
        score = s
    }

    public function addScore(points: number) {
        score += points
    }

    private function recalculate() {
        // internal
    }
}

local data = PlayerData.new("Player1", 100)
data:addScore(50)
```

### Structs

```js
struct Vec3 {
    x: number, y: number, z: number
}

local pos = Vec3.new(1, 2, 3)
print(pos.x)
```

### Enums

```js
enum GameState {
    Lobby,
    Playing,
    Ended
}

local state = GameState.Lobby
```

### Control Flow

```js
if (score > 100) {
    print("Winner!")
} elif (score > 50) {
    print("Almost")
} else {
    print("Try again")
}

while (timer > 0) {
    timer -= 1
}

for i in range(1, 10) {
    print(i)
}

for player in players {             // auto-detects ipairs(pairs)
    player:Kick()
}
```

### Imports

```js
import "../shared/Config" as Config       // relative (deployment-resolved)
import "src/shared/Utils" as Utils       // project-root-relative
import "./MathUtils" as MathUtils        // same directory
```

### Ternary & F-Strings

```js
local status = score > 50 ? "Pass" : "Fail"
local msg = f"Player {name} has {score} points"
```

### Error Handling

```js
try {
    riskyCall()
} catch (err) {
    print(f"Failed: {err}")
} finally {
    cleanup()
}
```

---

## Custom API Bindings (`.wold`)

Define type declarations for custom libraries — the LSP auto-discovers `.wold` files on workspace open.

```json
{
  "version": 1,
  "types": [
    {
      "name": "MyLibrary",
      "methods": [
        {
          "name": "doThing",
          "params": [{ "name": "x", "type": "number" }],
          "returns": "string"
        }
      ]
    }
  ],
  "globals": [
    { "name": "MY_LIB", "type": "MyLibrary" }
  ]
}
```

Regenerate built-in Roblox bindings from the official API dump:

```bash
node vscode-extension/generator/generate.js
```

---

## Compiler Architecture

```
Source (.wrm)
  → Tokenizer (logos) → 40+ token types
  → Parser (recursive descent) → AST with source spans
  → Validation (scope + luau-check)
  → Generator → Luau output (.luau)
```

### Source Layout

```
src/
├── main.rs                  CLI entry point (single file, project, watch, analyze)
├── lib.rs                   Library API (transpile, generate, validation, luau_check)
├── lexer.rs                 Tokenizer (logos-based)
├── parser.rs                Recursive descent parser
├── ast.rs                   AST node definitions with source spans
├── generator.rs             AST → Luau code emitter
├── roblox_config.rs         wolfram.toml parser + deployment resolver
├── roblox_context.rs        Script type detection (client/server/shared)
├── rojo_config.rs           Rojo default.project.json parser
├── constants.rs             Shared globals, service lists, path utils
├── errors.rs                TranspilerError hierarchy
├── analyze.rs               JSON diagnostics extractor
└── tests.rs                 Unit + integration tests (120 tests)
```

---

## License

MIT
