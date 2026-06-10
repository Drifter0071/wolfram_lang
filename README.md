# Wolfram

A C#/Python-inspired language compiling to [Luau](https://luau.org) for Roblox game development. First-class VS Code extension, Rojo integration, intelligent service resolution, and private-class encapsulation via shadow-table compilation.

---

## Showcase: PartSpawner (.wrm → .luau)

A fully-functional client-side part spawner — enums, structs, a private class, input handling, tween animations.

<table>
<tr><th>Wolfram (.wrm)</th><th>Luau (.luau)</th></tr>
<tr><td width="50%">

```js
import "../shared/config" as Config

private enum SpawnMode { Single, Grid, Circle, Random }
private enum PartShape  { Ball, Block, Cylinder }

private struct SpawnConfig {
    mode: SpawnMode, shape: PartShape,
    count: number, radius: number, cooldown: number
}

private struct ColorPalette {
    name: string, primary: Color3,
    secondary: Color3, accent: Color3
}

local Players = game:GetService("Players")
local UserInputService = game:GetService("UserInputService")
local TweenService = game:GetService("TweenService")
local player = Players.LocalPlayer
    or Players.PlayerAdded:Wait()
local mouse = player:GetMouse()

private class PartSpawner {
    private local spawnedCount = 0
    private local palette = ColorPalette.new(
        "Default",
        Color3.fromRGB(255, 100, 50),
        Color3.fromRGB(50, 150, 255),
        Color3.fromRGB(100, 255, 100)
    )

    private function init() {
        spawnedCount = 0
        self.config = Config
    }

    private function getStatus() {
        local active = self.getActiveParts()
        return f"Parts: {active.length}/{spawnedCount}"
    }

    private function getActiveParts() {
        return [p for p in workspace:GetChildren()
                if (p:IsA("Part")
                and p:GetAttribute("SpawnerId")
                    == self.id)]
    }

    private function randomColor() {
        return palette.primary
    }

    private function spawnAt(position, color) {
        local part = Instance.new("Part", workspace)
        part.Position = position
        part.Color = color or self.randomColor()
        part.Anchored = true
        part:SetAttribute("SpawnerId", self.id)
        spawnedCount = spawnedCount + 1
        return part
    }

    private function spawnCircle(center, count) {
        for i in range(0, count) {
            local angle = (math.pi * 2)*(i / count)
            local offset = Vector3.new(
                math.cos(angle)*self.config.radius,
                0,
                math.sin(angle)*self.config.radius)
            local pos = center + offset
            local part = self.spawnAt(pos, self.randomColor())
            task.spawn(function()
                local info = TweenInfo.new(0.5,
                    Enum.EasingStyle.Back,
                    Enum.EasingDirection.Out)
                part.Transparency = 1
                local tween = TweenService:Create(
                    part, info, {Transparency = 0})
                tween:Play()
                tween.Completed:Wait()
            })
        }
        self.notify(f"Spawned {count} parts")
    }

    private function spawnGrid(origin, cols, rows) {
        for r in range(0, rows) {
            for c in range(0, cols) {
                local pos = origin + Vector3.new(c*3, 0, r*3)
                game.Debris:AddItem(
                    self.spawnAt(pos, nil), 15)
            }
        }
    }

    private function notify(msg) {
        print(f"[PartSpawner] {msg}")
    }
}

local spawner = PartSpawner.new(
    SpawnConfig.new(SpawnMode.Circle,
        PartShape.Ball, 12, 8, 0.1))

local onMouseClick = function() {
    if (spawner == nil or not mouse) { return }
    local mode = "Single"
    local pos = mouse.Hit.Position
    if (mode == "Circle") {
        spawner:spawnCircle(pos, 8)
    } elif (mode == "Grid") {
        spawner:spawnGrid(pos, 3, 3)
    } else {
        game.Debris:AddItem(
            spawner:spawnAt(pos, nil), 10)
    }
}

UserInputService.InputBegan:Connect(
    function(input, processed) {
        if (processed) { return }
        if (input.UserInputType
            == Enum.UserInputType.MouseButton1) {
            onMouseClick()
        }
    })
```

</td><td width="50%">

```lua
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Config = require(ReplicatedStorage.Shared.config)

local SpawnMode = table.freeze({
    Single="Single",Grid="Grid",Circle="Circle",Random="Random"})
local PartShape = table.freeze({
    Ball="Ball",Block="Block",Cylinder="Cylinder"})

local SpawnConfig = {}
function SpawnConfig.new(mode,shape,count,radius,cooldown)
    return {mode=mode,shape=shape,count=count,
            radius=radius,cooldown=cooldown}
end
local ColorPalette = {}
function ColorPalette.new(name,primary,secondary,accent)
    return {name=name,primary=primary,
            secondary=secondary,accent=accent}
end

Players = game:GetService("Players")
UserInputService = game:GetService("UserInputService")
TweenService = game:GetService("TweenService")
local player = Players.LocalPlayer
    or Players.PlayerAdded:Wait()
local mouse = player:GetMouse()

local __private_PartSpawner = setmetatable({},{__mode="k"})
local PartSpawner = {}
PartSpawner.__index = PartSpawner

function PartSpawner.new(...)
    local self = setmetatable({}, PartSpawner)
    __private_PartSpawner[self] = {}
    __private_PartSpawner[self].spawnedCount = 0
    __private_PartSpawner[self].palette = ColorPalette.new(
        "Default",Color3.fromRGB(255,100,50),
        Color3.fromRGB(50,150,255),Color3.fromRGB(100,255,100))
    __private_PartSpawner[self].init = function()
        __private_PartSpawner[self].spawnedCount = 0
        self.config = Config
    end
    __private_PartSpawner[self].getStatus = function()
        local active = __private_PartSpawner[self].getActiveParts()
        return `Parts: {#active}/{__private_PartSpawner[self].spawnedCount}`
    end
    __private_PartSpawner[self].getActiveParts = function()
        return (function()
    local _result = {}
    for _, p in ipairs(workspace:GetChildren()) do
        if p:IsA("Part") and p:GetAttribute("SpawnerId")==self.id then
            table.insert(_result, p)
        end
    end
    return _result
end)()
    end
    __private_PartSpawner[self].randomColor = function()
        return __private_PartSpawner[self].palette.primary
    end
    __private_PartSpawner[self].spawnAt = function(position, color)
        local part = Instance.new("Part", workspace)
        part.Position = position
        part.Color = color or __private_PartSpawner[self].randomColor()
        part.Anchored = true
        part:SetAttribute("SpawnerId", self.id)
        __private_PartSpawner[self].spawnedCount =
            __private_PartSpawner[self].spawnedCount + 1
        return part
    end
    __private_PartSpawner[self].spawnCircle = function(center, count)
        for i = 0, count - 1 do
            local angle = (math.pi * 2) * (i / count)
            local offset = Vector3.new(
                math.cos(angle) * self.config.radius, 0,
                math.sin(angle) * self.config.radius)
            local pos = center + offset
            local part = __private_PartSpawner[self].spawnAt(pos,
                __private_PartSpawner[self].randomColor())
            task.spawn(function()
    local info = TweenInfo.new(0.5,
        (Enum and Enum.EasingStyle and Enum.EasingStyle.Back),
        (Enum and Enum.EasingDirection and Enum.EasingDirection.Out))
    part.Transparency = 1
    local tween = TweenService:Create(part, info, {Transparency = 0})
    tween:Play()
    tween.Completed:Wait()
end)
        end
        __private_PartSpawner[self].notify(`Spawned {count} parts`)
    end
    __private_PartSpawner[self].spawnGrid = function(origin, cols, rows)
        for r = 0, rows - 1 do
            for c = 0, cols - 1 do
                local pos = origin + Vector3.new(c*3, 0, r*3)
                game.Debris:AddItem(
                    __private_PartSpawner[self].spawnAt(pos, nil), 15)
            end
        end
    end
    __private_PartSpawner[self].notify = function(msg)
        print(`[PartSpawner] {msg}`)
    end
    __private_PartSpawner[self].init(...)
    return self
end

-- Public forwarding stubs for private methods
function PartSpawner:getStatus()
    return __private_PartSpawner[self].getStatus()
end
-- ... (forwarding stubs for all private methods)
function PartSpawner:spawnCircle(center, count)
    return __private_PartSpawner[self].spawnCircle(center, count)
end

local spawner = PartSpawner.new(
    SpawnConfig.new(SpawnMode.Circle, PartShape.Ball, 12, 8, 0.1))

local onMouseClick = function()
    if spawner == nil or not mouse then return end
    local mode = "Single"
    local pos = (mouse and mouse.Hit and mouse.Hit.Position)
    if mode == "Circle" then
        spawner:spawnCircle(pos, 8)
    elseif mode == "Grid" then
        spawner:spawnGrid(pos, 3, 3)
    else
        game.Debris:AddItem(spawner:spawnAt(pos, nil), 10)
    end
end

UserInputService.InputBegan:Connect(
    function(input, processed)
        if processed then return end
        if input.UserInputType
            == (Enum and Enum.UserInputType
                and Enum.UserInputType.MouseButton1) then
            onMouseClick()
        end
    end)
```

</td></tr>
</table>

---

## Quick Start

```bash
cargo build --release                           # Build compiler

wolfram my-project/                             # Transpile project
wolfram src/main.wrm                            # Transpile single file
wolfram --watch src/                            # Watch mode
wolfram --analyze src/main.wrm                  # JSON diagnostics
```

---

## Language Features

### Variables & Assignment

```js
local x = 42
local name               // nil by default
local a, b = 1, 2        // multi-variable
x = 100
x += 1                   // compound: += -= *= /= %=
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

// Arrow functions
local double = (x) -> x * 2

// Anonymous functions
local fn = function(x) { return x + 1 }

// Default parameters
function connect(host = "localhost", port: number = 3000) {
    // body
}
```

### Classes & Access Modifiers

Classes compile to metatable-based OOP with automatic `.__index` wiring. **`private` members** are stored in a per-instance weak-keyed shadow table (`__private_ClassName[instance]`) to maintain encapsulation at the Luau level:

| Modifier | Storage | External access | Internal access |
|----------|---------|----------------|-----------------|
| `public` | Metatable method | `instance:method()` | `instance:method()` |
| `private local` | `__private_Class[self].field` | Blocked at compile time | Resolved to shadow table |
| `private function` | `__private_Class[self].method` + public stub | Via forwarding stub | Direct shadow-table call |

```js
class PlayerData {
    private local score = 0          // shadow table
    private local name = ""          // shadow table

    public function init(n: string, s: number) {
        name = n                     // resolves to __private_PlayerData[self].name
        score = s
    }

    public function addScore(points: number) {
        score += points              // read + write through shadow table
        self:recalculate()           // internal call → direct shadow-table dispatch
    }

    private function recalculate() {
        return score * 1.5
    }
}

local data = PlayerData.new("Player1", 100)
data:addScore(50)                    // public method, normal call
data:recalculate()                   // WORKS — forwarding stub delegates to shadow table
```

**Generated Luau pattern:**

```lua
local __private_PlayerData = setmetatable({}, {__mode = "k"})

function PlayerData.new(...)
    local self = setmetatable({}, PlayerData)
    __private_PlayerData[self] = {}
    __private_PlayerData[self].name = nil
    __private_PlayerData[self].score = 0
    __private_PlayerData[self].recalculate = function()
        return __private_PlayerData[self].score * 1.5
    end
    __private_PlayerData[self].init(...)
    return self
end

-- Forwarding stub (generated automatically for every private method)
function PlayerData:recalculate()
    return __private_PlayerData[self].recalculate()
end
```

**Key behaviors:**
- **Bare identifiers** in method bodies resolve through a strict hierarchy: local scope → private member → module export → global
- **`self:method()` calls** from within sibling private methods use direct `__private_Class[self].method(args)` dispatch (avoiding metatable indirection)
- **External `instance:method()` calls** hit the public forwarding stub which delegates to the shadow table
- **`local` declarations** inside methods shadow private members (lexical scope takes precedence)
- **Assignments** to private members (`score = 5`) write to the shadow table and do NOT create local variables

### Structs

Structs compile to table constructors with `.new()` factory methods:

```js
struct Vec3 { x: number, y: number, z: number }

local pos = Vec3.new(1, 2, 3)
print(pos.x)
```

```lua
local Vec3 = {}
function Vec3.new(x, y, z)
    return {x = x, y = y, z = z}
end
```

### Enums

Enums compile to frozen tables with string values:

```js
enum GameState { Lobby, Playing, Ended }

local state = GameState.Lobby    // "Lobby"
```

```lua
local GameState = table.freeze({Lobby = "Lobby", Playing = "Playing", Ended = "Ended"})
```

### Control Flow

```js
// if/elif/else
if (score > 100) {
    print("Winner!")
} elif (score > 50) {
    print("Almost")
} else {
    print("Try again")
}

// while
while (timer > 0) { timer -= 1 }

// for-range (compiles to numeric for)
for i in range(1, 10)       { print(i) }   // for i = 1, 9 do
for i in range(0, 20, 2)   { print(i) }   // for i = 0, 19, 2 do

// for-in (auto-detects ipairs vs pairs via type inference)
for player in players { player:Kick() }     // ipairs
for item in dataStore  { process(item) }    // pairs (table type inferred)

// ternary expression
local status = score > 50 ? "Pass" : "Fail"

// list comprehension
local active = [p for p in workspace:GetChildren() if p:IsA("Part")]
```

### Imports & Service Injection

Four resolution strategies — the compiler resolves paths from `wolfram.toml` deployment map:

| Strategy | Source → Target | Example output |
|----------|----------------|---------------|
| **Cross-service** | Client → Shared | `require(ReplicatedStorage.Shared.Config)` + auto `game:GetService` |
| **Sibling** | Same service/parent | `require(script.Parent.Utils)` |
| **Deep nested** | Same service, sub-path | `require(ServerScriptService.Modules.Sub.Util)` |
| **StarterPlayer** | Runtime-movable scripts | Always `script.Parent` chains |

```js
import "../shared/Config" as Config    // cross-service resolution
import "./Button" as Button             // sibling
import "src/shared/Utils" as Utils      // project-root-relative
```

### F-Strings & Interpolation

Wolfram f-strings compile to Luau template literals. Expressions inside `{ }` are fully parsed and transpiled:

```js
local msg = f"Player {name} — score: {score * 100}%"
```

```lua
local msg = `Player {name} — score: {score * 100}%`
```

F-strings support nested expressions, ternary, function calls — anything parseable:

```js
local label = f"{player:GetName() or "Unknown"} [{leaderstats.Score}]"
```

### Error Handling (try/catch/finally)

```js
try {
    riskyCall()
} catch err {
    print(f"Failed: {err}")
} finally {
    cleanup()
}
```

Compiles to `pcall`-based Luau:

```lua
local function __try_0()
    riskyCall()
end
local ok, err = pcall(__try_0)
if not ok then
    print(`Failed: {err}`)
end
cleanup()
```

### Type Annotations (IntelliSense only)

Wolfram supports TypeScript-like type annotations. **Annotations are stripped from Luau output**
and exist purely to drive IDE completions and diagnostics.

```js
local player: Player = Players.LocalPlayer
local items: Part[]
local lookup: {[Player]: boolean}

function createBeam(from: Vector3, to: Vector3): Beam
local tween: Tween = TweenService:Create(obj, info, {Value = 1})

for zone: Part in workspace:GetChildren() { zone.Anchored = true }
```

| Syntax | Description |
|--------|-------------|
| `local name: Type` | Simple type annotation |
| `local name: Type[]` | Array type annotation |
| `local name: {[K]: V}` | Table type annotation |
| `function name(param: Type)` | Parameter type annotation |
| `function name(): ReturnType` | Return type annotation |
| `for var: Type in expr` | Loop variable annotation |

### Event + IntelliSense

- **Events** show in dot completions — `player.CharacterAdded:` resolves to `:Wait()`, `:Connect()`, `:Once()`
- **Method returns** — `TweenService:Create()` resolves to `Tween` type for chained completions
- **Service suggestions** — inside `:GetService("string")`, all Roblox services are suggested
- **Enum members** — `Enum.EasingStyle.` shows members; accepting pastes only the member name
- **24 built-in types** — Vector3, CFrame, OverlapParams, RaycastParams, UDim2, …

### Member Chains

Wolfram emits plain chains — no automatic safe-navigation wrapping:

```js
local ui = zone.Display.Canvas.InfoDisplay
```
```lua
local ui = zone.Display.Canvas.InfoDisplay  -- straight chain
```

### Decorators

```js
@deprecated
@experimental
public function oldAPI() { }
```

Decorator metadata is preserved for tooling; the decorated statement compiles normally.

### Script Type Suffixes

- **`.client.wrm`** → `LocalScript` (runs on client)
- **`.server.wrm`** → `ServerScript` (runs on server)
- **`.wrm`** (no suffix) → `ModuleScript` (shared, produces module table)

The LSP warns when `.client.wrm` or `.server.wrm` files contain `public` declarations (scripts don't export module tables).

---

## Compiler Architecture

```
Source (.wrm)
  → Tokenizer (logos) → 40+ token types
  → Parser (recursive descent) → AST with source spans
  → Validation (scope analysis + Luau syntax check)
  → Generator → Luau output (.luau)
```

### Source Layout

```
src/
├── main.rs                  CLI entry point (single, project, watch, analyze)
├── lib.rs                   Library API (transpile, generate, validation)
├── lexer.rs                 Tokenizer (logos-based)
├── parser.rs                Recursive descent parser
├── ast.rs                   AST node definitions with source spans
├── generator.rs             AST → Luau code emitter
│                            (private scope resolution, forwarding stubs,
│                             safe-chain suppression, class generation)
├── roblox_config.rs         wolfram.toml parser + deployment resolver
├── roblox_context.rs        Script type detection (client/server/module)
├── rojo_config.rs           Rojo default.project.json parser
├── constants.rs             Shared globals, service lists
├── errors.rs                TranspilerError hierarchy
├── analyze.rs               JSON diagnostics extractor
├── luau_checker.rs          Luau validation engine
├── tester.rs                Test harness
└── tests.rs                 Unit + integration tests (200 tests)
```

---

## VS Code Extension

### Features

- **Syntax highlighting** — TextMate grammar for `.wrm` files
- **Intellisense** — Keyword snippets, local symbols, function signatures
- **Roblox API autocomplete** — 849 types, 585 enums from official API dump
- **Dot/colon awareness** — `part:` shows Instance methods, `game.` shows DataModel properties; member-access targets are never flagged as undefined
- **Type inference** — Infers types from assignments and method returns
- **Diagnostics** — Compiler errors mapped to editor, plus scope warnings (undefined vars with AST-aware skipping of enum/struct/member contexts)
- **Public-in-script warning** — Flags `public` declarations in `.client.wrm`/`.server.wrm` files
- **Hover info** — Type signatures, parameters, documentation
- **Go to definition** — Jump to symbol declarations
- **Document symbols** — Outline view (classes, functions, enums, structs)
- **Watch mode** — Auto-transpile on every save via status bar toggle
- **Project template** — One-command scaffold with `client/`, `shared/`, `server/` + Rojo config

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
│   ├── client/                  StarterPlayer LocalScripts
│   │   ├── UI.client.wrm
│   │   └── PartSpawner.client.wrm
│   ├── shared/                  ReplicatedStorage ModuleScripts
│   │   ├── Logger.wrm
│   │   └── Config.wrm
│   └── server/                  ServerScriptService Scripts
│       └── ServerMain.server.wrm
└── out/                         Auto-generated Luau (gitignored)
    ├── client/
    ├── shared/
    └── server/
```

### `wolfram.toml` — Deployment Map

```toml
[deployment]
"src/shared"    = "ReplicatedStorage.Shared"
"src/server"    = "ServerScriptService.ServerModules"
"src/client/ui" = "StarterPlayer.StarterPlayerScripts.UI"
```

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

### Pipeline

1. Write `.wrm` files in `src/` under `client/`, `shared/`, or `server/`
2. The watcher/compiler reads `wolfram.toml` to resolve Roblox instance paths
3. Service declarations (`game:GetService`) are auto-injected at the top of each output
4. Transpiled `.luau` goes to identically-structured `out/`
5. Rojo reads `default.project.json` and syncs `out/` into Roblox Studio
6. `.client.wrm` → `LocalScript`, `.server.wrm` → `ServerScript`, `.wrm` → `ModuleScript`

---

## Custom API Bindings (`.wold`)

Define type declarations for custom libraries — the LSP auto-discovers `.wold` files on workspace open.

```json
{
  "version": 1,
  "types": [{
    "name": "MyLibrary",
    "methods": [{
      "name": "doThing",
      "params": [{ "name": "x", "type": "number" }],
      "returns": "string"
    }]
  }],
  "globals": [{ "name": "MY_LIB", "type": "MyLibrary" }]
}
```

Regenerate built-in Roblox bindings from the official API dump:

```bash
node vscode-extension/generator/generate.js
```

---

## License

MIT
