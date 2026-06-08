# Wolfram → Luau Edge-Case Test Plan

## Purpose
Exercise the transpiler against a comprehensive suite of Wolfram inputs and verify the generated Luau output matches expected behavior. Focuses on edge cases where `.` syntax, path resolution, type constructors, control flow, and complex expressions could produce invalid Luau.

---

## Category A: Table Length Access (`.length` → `#`)

### A1: Simple `.length` on variable
**Wolfram Input:**
```
print(products.length)
```
**Expected Luau:**
```lua
print(#products)
```

### A2: `len()` function call
**Wolfram Input:**
```
print(len(items))
```
**Expected Luau:**
```lua
print(#items)
```

### A3: `.length` in conditional
**Wolfram Input:**
```
if (arr.length > 0) {
    print(arr.length)
}
```
**Expected Luau:**
```lua
if #arr > 0 then
    print(#arr)
end
```

### A4: `.length` in assignment
**Wolfram Input:**
```
local count = data.length
```
**Expected Luau:**
```lua
local count = #data
```

### A5: `.length` in function argument
**Wolfram Input:**
```
process(list.length)
```
**Expected Luau:**
```lua
process(#list)
```

### A6: `len()` with expression argument
**Wolfram Input:**
```
print(len(getItems()))
```
**Expected Luau:**
```lua
print(#getItems())
```

### A7: Chained `.length` access (nested)
**Wolfram Input:**
```
print(obj.items.length)
```
**Expected Luau:**
```lua
print(#obj.items)
```

### A8: `.length` inside f-string
**Wolfram Input:**
```wolfram
print(`Count: ${items.length}`)
```
**Expected Luau:**
(Note: f-string interpolation of expressions may vary; the `.length` in the expression should be #)
```lua
print(`Count: ${#items}`)
```

### A9: `.length` on member expression result
**Wolfram Input:**
```
print(players:GetChildren().length)
```
**Expected Luau:**
```lua
print(#players:GetChildren())
```

---

## Category B: Module Path Resolution (suffix stripping)

### B1: Shared module with `.shared` suffix
**Wolfram Input:**
```wolfram
import "src/shared/logger.shared" as Log
```
**Expected Luau** (with config mapping `src/shared` → `ReplicatedStorage.Shared`):
```lua
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Log = require(ReplicatedStorage.Shared.logger)
```

### B2: Server module with `.server` suffix
**Wolfram Input:**
```wolfram
import "src/server/handler.server" as Handler
```
**Expected Luau** (with config mapping `src/server` → `ServerScriptService.Server`):
```lua
local ServerScriptService = game:GetService("ServerScriptService")
local Handler = require(ServerScriptService.Server.handler)
```

### B3: Client module with `.client` suffix
**Wolfram Input:**
```wolfram
import "src/client/gui.client" as GUI
```
**Expected Luau** (with config mapping `src/client` → `StarterPlayer.StarterPlayerScripts.Client`):
```lua
local StarterPlayer = game:GetService("StarterPlayer")
local GUI = require(StarterPlayer.StarterPlayerScripts.Client.gui)
```

### B4: Module without any context suffix
**Wolfram Input:**
```wolfram
import "src/shared/utils" as Utils
```
**Expected Luau:**
```lua
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Utils = require(ReplicatedStorage.Shared.utils)
```

### B5: Module in subdirectory with suffix
**Wolfram Input:**
```wolfram
import "src/shared/helpers/validator.shared" as Valid
```
**Expected Luau:**
```lua
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Valid = require(ReplicatedStorage.Shared.helpers.validator)
```

### B6: Bare import without `.wrm` extension (relative path)
**Wolfram Input:**
```wolfram
import "./sibling" as Sibling
```
**Expected Luau:**
```lua
local Sibling = require(script.Parent.sibling)
```

---

## Category C: Complex Expressions

### C1: Ternary expression
**Wolfram Input:**
```
local result = if (x > 0) then "positive" else "negative"
```
**Expected Luau:**
```lua
local result = (if x > 0 then "positive" else "negative")
```

### C2: Nested method calls
**Wolfram Input:**
```
part:SetPrimaryPartCFrame(CFrame.new(0, 10, 0))
```
**Expected Luau:**
```lua
part:SetPrimaryPartCFrame(CFrame.new(0, 10, 0))
```

### C3: Deep member access chain
**Wolfram Input:**
```
local health = player.Character.Humanoid.Health
```
**Expected Luau:**
```lua
local health = player.Character.Humanoid.Health
```

### C4: Method with colon syntax
**Wolfram Input:**
```
game:GetService("Players")
```
**Expected Luau:**
```lua
game:GetService("Players")
```

### C5: Method with dot syntax (static)
**Wolfram Input:**
```
Vector3.new(1, 2, 3)
```
**Expected Luau:**
```lua
Vector3.new(1, 2, 3)
```

### C6: Logical operators
**Wolfram Input:**
```
local ok = x and y or z
```
**Expected Luau:**
```lua
local ok = x and y or z
```

### C7: Unary operations
**Wolfram Input:**
```
local neg = -value
local flag = not done
```
**Expected Luau:**
```lua
local neg = -value
local flag = not done
```

---

## Category D: Class Definitions

### D1: Basic class with constructor
**Wolfram Input:**
```
class Player {
    local name = ""
    public function init(n) {
        name = n
    }
    public function getName() {
        return name
    }
}
```
**Expected Luau:** Contains `class` boilerplate with constructor, `self.name`, `name` private variable, public `getName` method.

### D2: Class with private fields and methods
**Wolfram Input:**
```
class Counter {
    private local count = 0
    public function init() {}
    private function increment() {
        count = count + 1
    }
    public function next() {
        self:increment()
        return count
    }
}
```
**Expected Luau:** Private `count` stored in `__private_Counter[self].count`, private `increment` method accessible via self, public `next` uses `__private_Counter[self].increment(self)`.

---

## Category E: Control Flow

### E1: For loop with range
**Wolfram Input:**
```
for i in range(0, 10) {
    print(i)
}
```
**Expected Luau:**
```lua
for i = 0, 10 - 1 do
    print(i)
end
```

### E2: For loop with array
**Wolfram Input:**
```
for item in items {
    print(item)
}
```
**Expected Luau:**
```lua
for _, item in ipairs(items) do
    print(item)
end
```

### E3: While loop
**Wolfram Input:**
```
while (x > 0) {
    x = x - 1
}
```
**Expected Luau:**
```lua
while x > 0 do
    x = x - 1
end
```

### E4: If/elif/else chain
**Wolfram Input:**
```
if (x > 10) {
    print("high")
} elif (x > 5) {
    print("mid")
} else {
    print("low")
}
```
**Expected Luau:**
```lua
if x > 10 then
    print("high")
elseif x > 5 then
    print("mid")
else
    print("low")
end
```

### E5: Try/catch
**Wolfram Input:**
```
try {
    riskyCall()
} catch {
    print("failed")
}
```
**Expected Luau:**
```lua
local function __try_0()
    riskyCall()
end
local ok, err = pcall(__try_0)
if not ok then
    print("failed")
end
```

---

## Category F: Data Structures

### F1: Array literal
**Wolfram Input:**
```
local nums = [1, 2, 3, 4]
```
**Expected Luau:**
```lua
local nums = {1, 2, 3, 4}
```

### F2: Table literal with named keys
**Wolfram Input:**
```
local cfg = {name: "test", value: 42}
```
**Expected Luau:**
```lua
local cfg = {name = "test", value = 42}
```

### F3: Mixed table (array + dict)
**Wolfram Input:**
```
local mixed = {"one", "two", key: "val"}
```
**Expected Luau:**
```lua
local mixed = {"one", "two", key = "val"}
```

### F4: Empty structures
**Wolfram Input:**
```
local empty = []
local empty_table = {}
```
**Expected Luau:**
```lua
local empty = {}
local empty_table = {}
```

### F5: Index access with expression
**Wolfram Input:**
```
local val = data[idx + 1]
```
**Expected Luau:**
```lua
local val = data[idx + 1]
```

---

## Category G: Enum and Struct Definitions

### G1: Enum definition
**Wolfram Input:**
```
enum State {
    Lobby, Playing, Ended
}
```
**Expected Luau:**
```lua
local State = table.freeze({Lobby = "Lobby", Playing = "Playing", Ended = "Ended"})
```

### G2: Struct definition
**Wolfram Input:**
```
struct Vec3 {
    x, y, z
}
```
**Expected Luau:**
```lua
local Vec3 = {}
function Vec3.new(x, y, z)
    return {x = x, y = y, z = z}
end
```

---

## Category H: Edge Cases

### H1: Nested function definitions
**Wolfram Input:**
```
function outer() {
    function inner() {
        return 1
    }
    return inner()
}
```
**Expected Luau:**
```lua
local function outer()
    local function inner()
        return 1
    end
    return inner()
end
```

### H2: List comprehension
**Wolfram Input:**
```
local squares = [x * x for x in range(1, 10)]
```
**Expected Luau:** Uses `(function() ... end)()` pattern with `for` loop and `table.insert`.

### H3: List comprehension with condition
**Wolfram Input:**
```
local evens = [x for x in items if x % 2 == 0]
```
**Expected Luau:** Filters via `if` inside generated loop.

### H4: Empty file
**Wolfram Input:**
```
```
**Expected Luau:**
```lua
```

### H5: Only whitespace/comments
**Wolfram Input:**
```
// This is a comment
-- Another comment
```
**Expected Luau:**
```lua
```

### H6: Roblox global access (bare service names)
**Wolfram Input:**
```
local players = game:GetService("Players")
local list = players:GetPlayers()
```
**Expected Luau:**
```lua
local players = game:GetService("Players")
local list = players:GetPlayers()
```

### H7: Multiple imports with same service
**Wolfram Input:**
```wolfram
import "src/shared/logger.shared" as Log
import "src/shared/utils.shared" as Utils
```
**Expected Luau:** Only one `game:GetService("ReplicatedStorage")` call, reused:
```lua
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Log = require(ReplicatedStorage.Shared.logger)
local Utils = require(ReplicatedStorage.Shared.utils)
```

---

## Category I: Type Constructors and API Patterns

### I1: Vector3 constructor
**Wolfram Input:**
```
local pos = Vector3.new(10, 0, 5)
```
**Expected Luau:**
```lua
local pos = Vector3.new(10, 0, 5)
```

### I2: CFrame arithmetic
**Wolfram Input:**
```
local cf = CFrame.new(0, 5, 0) * CFrame.Angles(0, math.rad(90), 0)
```
**Expected Luau:**
```lua
local cf = CFrame.new(0, 5, 0) * CFrame.Angles(0, math.rad(90), 0)
```

### I3: Color3 from RGB
**Wolfram Input:**
```
local col = Color3.fromRGB(255, 0, 0)
```
**Expected Luau:**
```lua
local col = Color3.fromRGB(255, 0, 0)
```

### I4: Instance.new with parent
**Wolfram Input:**
```
local part = Instance.new("Part", workspace)
```
**Expected Luau:**
```lua
local part = Instance.new("Part", workspace)
```

---

## Category J: Null/Nil Patterns

### J1: Nil variable
**Wolfram Input:**
```
local x = nil
```
**Expected Luau:**
```lua
local x = nil
```

### J2: Boolean literals
**Wolfram Input:**
```
local flag = true
local off = false
```
**Expected Luau:**
```lua
local flag = true
local off = false
```

### J3: Equality check (==)
**Wolfram Input:**
```
if (x == nil) {
    print("null")
}
```
**Expected Luau** (note: `== nil` simplified to just variable):
```lua
if x == nil then
    print("null")
end
```

---

## Category K: Transpiler-Specific Edge Cases

### K1: Expression starting with member access (should NOT crash)
**Wolfram Input:**
```
print(someTable.someField.length.something)
```
**Expected behavior:** `.length` in the middle transforms to `#someTable.someField` followed by `.something`. The safe chain logic should wrap this properly:
```lua
print((someTable.someField and #someTable.someField).something)
```

### K2: Same name as Roblox global in local scope
**Wolfram Input:**
```
local game = "hello"
print(game)
```
**Expected Luau:**
```lua
local game = "hello"
print(game)
```

### K3: Public export in non-roblox mode
**Wolfram Input:**
```
public function greet() {
    return "hello"
}
```
**Expected Luau:** Should auto-generate a `return {greet = greet}` at end.

### K4: Decorated statement
**Wolfram Input:**
```
@deprecated
function oldFunc() {
    return nil
}
```
**Expected Luau:** Inner function generated normally, decorator treated as metadata.

---

## Test Implementation Notes

For Rust tests, each test case should:
1. Call `tokenize_and_parse()` to verify the parser doesn't reject it
2. Call `transpile()` to get the generated Luau
3. Assert on key patterns in the output (e.g., `contains("#products")`, `!contains(".length")`)

For module path tests, use `transpile_roblox()` with a mock `RobloxProjectConfig` and `DeploymentEntry`.

For class/enum/struct tests, verify the output contains the expected Luau patterns (e.g., `table.freeze`, `__private_`, `setmetatable`).

### Regression Check Test
A meta-test that runs ALL the above Wolfram inputs through the transpiler and verifies:
1. No panics or crashes
2. Generated Luau doesn't contain `.length` (only `#`)
3. Generated Luau doesn't contain `len(` (only `#`)
4. Require paths don't contain `.shared`, `.server`, `.client` suffixes
5. All braces are balanced
