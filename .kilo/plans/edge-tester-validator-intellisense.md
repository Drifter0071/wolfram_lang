# Plan: Edge-Case Tester, Robust Luau Validator, and Roblox-Studio-Level IntelliSense

## Overview

Three integrated enhancements to the Wolfram language toolchain:

| # | Feature | Scope | Complexity |
|---|---------|-------|------------|
| 1 | Edge-Case Tester (`--test` mode) | New Rust module | Medium |
| 2 | Roblox-Tailored Luau Validator | Enhance `luau_checker.rs` + `roblox_api.rs` | High |
| 3 | Studio-Level IntelliSense | Enhance TS LSP + `roblox.wold` + `wolfram.toml` | High |

---

## Part 1: Edge-Case Tester (`--test` Mode)

### 1.1 Purpose
A transpiler mode that compiles Wolfram → Luau and then runs a battery of static and dynamic checks to catch edge cases that wouldn't be obvious to developers. Not a unit test framework — a **validation harness** embedded in the compiler.

### 1.2 Architecture

```
wolfram --test <file.wrm>
          │
          ▼
    ┌─────────────┐
    │ Parse + AST  │
    │ (tokenize)   │
    └──────┬──────┘
           ▼
    ┌─────────────┐
    │ Transpile    │
    │ Wrm → Luau   │
    └──────┬──────┘
           ▼
    ┌─────────────┐
    │ Luau Syntax  │────▶ Syntax error? → FAIL
    │ Validation   │     (via lexer on generated output)
    └──────┬──────┘
           ▼
    ┌─────────────┐
    │ Static       │────▶ Warnings/errors → REPORT
    │ Analysis     │     (dead code, nil deref, type mismatch, unused vars)
    └──────┬──────┘
           ▼
    ┌─────────────┐
    │ Roblox API   │────▶ Invalid property/method → FAIL
    │ Conformance  │     (checks all Member/MethodCall against API DB)
    └──────┬──────┘
           ▼
    ┌─────────────┐
    │ Pattern      │────▶ Warnings → REPORT
    │ Detector     │     (`.length` usage, bare `require()` calls, missing `:GetService()`,
    │              │      global assignment, non-idiomatic patterns)
    └──────┬──────┘
           ▼
    ┌─────────────┐
    │ Report       │
    │ (console/    │
    │  JSON)       │
    └─────────────┘
```

### 1.3 Implementation Plan

#### New file: `src/tester.rs`
- Struct `TesterConfig`: `{ strict: bool, fail_fast: bool, patterns: Vec<String> }`
- Struct `TestResult`: `{ passed: bool, errors: Vec<Diagnostic>, warnings: Vec<Diagnostic>, stats: TestStats }`
- `fn run_test(source: &str, config: &TestConfig) -> TestResult`

#### Checks implemented:

| Check | Type | Description |
|-------|------|-------------|
| **Luau Syntax Valid** | Error | Re-parse generated Luau to ensure valid syntax |
| **Nil Safety** | Error | Detect `.` access on possibly-nil values without nil guards |
| **Dead Code** | Warning | `return` followed by unreachable statements |
| **Unused Variables** | Warning | Declared but never referenced (`_` prefix exempt) |
| **Unused Imports** | Warning | Module imported but alias never used |
| **API Property/Method Exists** | Error | Every `obj.Property` / `obj:Method()` validated against `roblox.wold` |
| **API Parameter Count** | Warning | Wrong number of arguments to known Roblox methods |
| **API Parameter Type** | Warning | Obviously wrong argument types (string where Vector3 expected, etc.) |
| **Server/Client Boundary** | Error | Accessing server-only APIs from client scripts and vice versa |
| **Conditional Nil Safe** | Warning | `.length` used on dictionary tables instead of arrays |
| **Yield Safety** | Warning | `task.wait()` or coroutine calls in places that shouldn't yield |
| **Pattern Detector** | Info | Custom regex-based patterns configured in `wolfram.toml` |

#### CLI Integration
Add `wolfram --test <file.wrm>` and `wolfram --test <project_dir/>`.

#### `wolfram.toml` Extension
```toml
[test]
strict = true                    # Fail on warnings too
fail_fast = true                 # Stop at first error
check_nil_safety = true
check_dead_code = true
check_api_conformance = true
allowed_patterns = [             # Allow known safe patterns
    "script.Parent.Parent",      # Suppress deep Parent chain warnings
]
forbidden_patterns = [           # Flag custom anti-patterns
    "while true do"              # Users can add their own
]
```

---

## Part 2: Stronger Roblox-Tailored Luau Validator

### 2.1 Current State
- `luau_checker.rs` has 4 phases but Phase 3 (`check_property_method_existence`) only checks for deprecated functions — it does NOT actually validate that `part:Destroy()` exists on `BasePart` or that `player.Foo` is invalid.
- `roblox_api.rs` has a hardcoded subset of Roblox classes (~30 classes, ~200 properties/methods). The `roblox.wold` file is much more comprehensive but is only used by the LSP, not the validator.

### 2.2 Goal
Make the validator a **Roblox-specific semantic checker** that:
- Validates every property/method access against the full API database
- Tracks type flow through assignments (basic type inference)
- Checks Roblox-specific rules (server/client boundaries, service access patterns, event wiring)

### 2.3 Implementation Plan

#### A. Load `roblox.wold` in Rust validator
- **New file: `src/api_db.rs`** — Load `roblox.wold` JSON at compile time via `include_str!()` or at runtime via `serde_json`.
- Structs: `ApiDatabase` with `types: HashMap<String, ApiType>`, `globals: HashMap<String, ApiGlobal>`
- Methods: `get_class(name) -> Option<ApiType>`, `property_exists(class, prop) -> bool`, `method_exists(class, method) -> bool`
- This is a Rust-side counterpart to `vscode-extension/src/lsp/bindings.ts` but with faster lookup (string-keyed HashMap).

#### B. Enhance Phase 3: API Conformance
Replace the current stub validation with real checks:

| Check | Implementation |
|-------|---------------|
| Property exists on class | Walk the `Member` chain from root → check each step against `api_db` |
| Method exists on class | Same for `MethodCall` nodes |
| Parameter count matches | Compare `args.len()` to expected `params.len()` |
| Type resolution through chains | `player.Character.Humanoid.Health` — resolve `Character` returns `Model?`, then `Model.Humanoid` returns `Humanoid`, etc. |
| Server-only API in client | `FireServer` in client script → error |
| Client-only API in server | `LocalPlayer` in server script → warning |
| Deprecated API detection | Map known deprecated methods (e.g., `FindFirstChild` → `FindFirstChild`) |

#### C. Enhance Phase 2: Semantic Checks
- **Flow-sensitive nil safety**: If a variable is assigned `game:GetService("Players")`, tag it as non-nil. If accessed through a nullable property chain, require nil guard (`if obj then obj.field end`).
- **Table shape checking**: If a table is used as a dictionary, warn about `#table` (which is unreliable on dictionaries).
- **Immutable assignment detection**: Detect `Enum.Name = 5` (assigning to enum) or `CFrame.new().X = 10` (can't modify built-in type properties directly — need `CFrame.new()` call).

#### D. Enhance Phase 4: Architecture Patterns
- **Event wiring check**: `RemoteEvent.OnServerEvent:Connect(handler)` — verify handler signature matches event params
- **Module return check**: ModuleScripts should have explicit return values
- **Circular dependency detection**: Already implemented but uses file-path-based graph. Enhance to support cross-service cycle detection.

---

## Part 3: Studio-Level IntelliSense

### 3.1 Current State
The TS LSP provides basic completion/hover from `roblox.wold`. It handles:
- Globals (`game`, `workspace`, `print`)
- Classes (`Instance.Destroy`, `Part.Position`)
- Enums (`Enum.KeyCode`)
- Keywords with snippets

**What's missing** for Roblox Studio-like experience:
- Can't chain through types: typing `game.Players.LocalPlayer.` doesn't suggest `Character`, `UserId`, etc.
- No parameter hints when typing method arguments
- No `signatureHelp` for known Roblox functions
- Doesn't flag wrong argument counts or types
- No code actions (quick fixes for common issues like `.length` → `#`)
- No integration with `wolfram.toml` for path completions, deployment awareness

### 3.2 Implementation Plan

#### A. Type-Chaining Completions
When the user types `player.Character.Humanoid.`, the LSP needs to:
1. Resolve `player` → type: `Player` (from local scope or API globals)
2. `Player.Character` → property type: `Model?`
3. `Model.Humanoid` → property type: `Humanoid` (traverse inheritance)
4. Show `Humanoid` properties and methods

**Implementation in `completion.ts`:**
```typescript
function resolveChainedType(prefix: string, bindings: Bindings, scope: Map<string, string>): string | undefined {
    const parts = prefix.split('.');
    const root = parts[0];
    // Check local scope first
    if (scope.has(root)) {
        let current = scope.get(root)!;
        // If scope type is a known class, traverse
        for (let i = 1; i < parts.length; i++) {
            const props = bindings.getAllProperties(current);
            const p = props.find(p => p.name.toLowerCase() === parts[i].toLowerCase());
            if (p) { current = p.type; continue; }
            // Check methods too
            const methods = bindings.getAllMethods(current);
            const m = methods.find(m => m.name.toLowerCase() === parts[i].toLowerCase());
            if (m) { current = m.returns; continue; }
            return 'Instance'; // Fallback
        }
        return current;
    }
    // Check API globals
    const g = bindings.getGlobal(root);
    if (g) { /* similar traversal */ }
    return undefined;
}
```

#### B. Signature Help (Parameter Hints)
Enhance `signatureHelp` in `server.ts`:
- For known Roblox methods, show parameter labels with types
- Highlight active parameter as user types commas
- Support method syntax `obj:Method(...)` and static syntax `ClassName.new(...)`

#### C. Enhanced Hover
Show richer hover information for Roblox API:
- Method: shows all parameters with types, return type, description
- Property: shows type, read/write status, description
- Event: shows callback signature
- Include links to Roblox documentation URLs where available

#### D. Code Actions (Quick Fixes)
Add code actions via `codeActionProvider`:
| Trigger | Action |
|---------|--------|
| `obj.length` | Replace with `#obj` |
| `len(x)` | Replace with `#x` |
| `import "foo"` without `as alias` | Add-as alias |
| `game.GetService("Foo")` (bare) | Suggest `:GetService` (colon syntax) |
| Unused import | Remove import |
| `local x = game:GetService("Players")` | Type-annotate: `-- type: Players` |

#### E. Wolfram.toml Integration in LSP
The LSP should load and use `wolfram.toml` for:
- **Path completions**: When typing `import "src/|"`, show completions filtered by deployment config
- **Deployment-aware suggestions**: Know which services are available in which context
- **Module path resolution in hover**: Show the resolved `require()` path for imports
- **Config validation**: Warn if `wolfram.toml` has invalid service names or missing directories

New `wolfram.toml` features to add:
```toml
[lsp]
diagnostic_level = "strict"       # "strict" | "standard" | "minimal"
enable_api_docs = true             # Show Roblox API docs in hover
max_completions = 100              # Limit completion items
path_hints = true                  # Show resolved require() paths in code lenses

[test]                             # New section for edge-case tester (see Part 1)
strict = true
...

[patterns]                         # New section for custom code patterns
warn_on = [".length", "while true do"]
require_import_as = true           # Require `as` keyword in imports
```

---

## Implementation Order

| Step | Task | Time | Depends On |
|------|------|------|------------|
| 1 | Build `api_db.rs` — load `roblox.wold` in Rust | 1-2 hrs | — |
| 2 | Enhance `luau_checker.rs` Phase 3 — real API validation | 2-3 hrs | Step 1 |
| 3 | Enhance `luau_checker.rs` Phase 2 — flow-sensitive nil + table shape | 1-2 hrs | — |
| 4 | Build `tester.rs` — edge-case test mode | 2-3 hrs | Steps 1-3 |
| 5 | CLI integration for `--test` flag | 0.5 hr | Step 4 |
| 6 | Enhance TS LSP completion — type chaining | 2-3 hrs | — |
| 7 | Enhance TS LSP signatureHelp | 1-2 hrs | — |
| 8 | Enhance TS LSP hover — rich docs | 1 hr | — |
| 9 | Add code actions | 1-2 hrs | — |
| 10 | `wolfram.toml` LSP integration | 1-2 hrs | — |
| 11 | New `wolfram.toml` features | 1 hr | — |
| 12 | Tests for all new features | 2-3 hrs | All above |

---

## Key Design Decisions

1. **Single source of truth**: The `roblox.wold` JSON file should be the canonical Roblox API database, used by both Rust validator AND TS LSP. No duplication.

2. **Graceful degradation**: If `roblox.wold` is missing, the validator falls back to the hardcoded `roblox_api.rs` subset. The LSP falls back to basic completion.

3. **Performance**: The Rust validator caches API lookups. The TS LSP caches parsed documents and bindings. Both use incremental recomputation.

4. **Config-driven flexibility**: `wolfram.toml` is the central configuration point. Both compiler and LSP read it. Diagnostics severity, pattern detection, and path mappings are all controlled from one file.

5. **Strict by default**: The `--test` mode is strict by default. The LSP diagnostic level can be tuned in `wolfram.toml`.
