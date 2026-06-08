# Plan: Native TypeScript LSP Rewrite

## Goal
Delete all Rust LSP code (`src/lsp/`) and build a complete TypeScript LSP in `vscode-extension/src/lsp/` that provides full intellisense — autocomplete, hover, diagnostics, go-to-def, rename, semantic tokens, inlay hints, signature help, document/workspace symbols, code actions, and snippets.

## Status Quo

### What the Rust LSP currently provides (must be replaced)
| Feature | Rust File | TS File (to create/expand) |
|---|---|---|
| Lexer/Parser/AST | `lexer.rs`, `parser.rs`, `ast.rs` | `parser.ts` (partially exists, needs rewrite) |
| Document Store | `store.rs` | `store.ts` (new) |
| Bindings (.wold loader) | `bindings.rs` | `bindings.ts` (already exists, solid) |
| Completion | `handlers.rs` | `completion.ts` (exists but scope inference weak) |
| Hover | `handlers.rs` | `hover.ts` (exists) |
| Go-to-Definition | `handlers.rs` | inline in `server.ts` (basic, needs cross-file) |
| Signature Help | `handlers.rs` | inline in `server.ts` (basic) |
| Document Symbols | `symbols.rs` | inline in `server.ts` (basic) |
| Workspace Symbols | `symbols.rs` | **missing** |
| Code Actions | `code_actions.rs` | inline in `server.ts` (basic) |
| Rename (prepare + execute) | `rename.rs` | **missing** |
| Inlay Hints | `inlay_hints.rs` | **missing** |
| Semantic Tokens | `semantic_tokens.rs` | **missing** |
| Diagnostics | `handlers.rs` | `diagnostics.ts` (exists) |
| Snippets | `snippets.rs` | **missing** |
| Server main loop | `server.rs` | `server.ts` (exists, needs expansion) |
| Type checking | `typeck.rs` | `diagnostics.ts` (needs expansion) |
| Scope analysis | `scope.rs` | inline in `parser.ts` (basic) |

### What the TS LSP already has (in `vscode-extension/src/lsp/`)
- `server.ts` — LSP wire-up with onCompletion, onHover, onDefinition, onDocumentSymbol, onSignatureHelp, onCodeAction
- `completion.ts` — context-aware completions (keywords, locals, globals, roblox API, enum, dot/colon, import paths)
- `hover.ts` — hover info for globals/functions/symbols
- `parser.ts` — hand-written tokenizer + basic recursive-descent parser producing symbols/imports/scope
- `bindings.ts` — `.wold` file loader for Roblox API (849 types, 585 enums)
- `diagnostics.ts` — basic Luau compatibility diagnostics
- `utils.ts` — helpers (line prefix, word extraction, string detection)

## Implementation Plan

### Phase 1: Full TypeScript Parser (replace `parser.ts`)
**Goal:** Parse Wolfram source into a proper AST with spans, producing symbols, imports, scope, and errors.

Create `ast.ts` — AST node type definitions:
- `Span` (start, end)
- `Expr` variants: Number, Str, FString, Bool, Nil, Ident, SelfExpr, Call, MethodCall, Member, Index, Binary, Logical, UnaryMinus, Not, Grouping, Ternary, Array, Table, Function, ListComp, AwaitExpr
- `Stmt` variants: Local, Assign, Return, If, While, For, FuncDef, ClassDef, EnumDef, StructDef, Import, Break, Continue, TryCatch, ExprStmt, DecoratedStmt
- `TableField` (Pair/Value)
- `ListCompGenerator` (var, iter, condition)

Rewrite `parser.ts` as proper parser module with:
- `Lexer` class: tokenize into Token stream (keyword, ident, string, fstring, number, operator, dot_colon)
- `Parser` class: recursive descent producing `Stmt[]`
- Export: `parseSource(text: string): ParseResult { ast, symbols, imports, scope, errors }`
- Full scope tracking (locals, function params, for-loop vars, class members)

### Phase 2: Document Store (`store.ts`)
Create `store.ts`:
- `DocumentStore` class with open/update/close/get/reparseIfDirty
- `DocumentState` with uri, source, ast, symbols, imports, scope, dirty flag
- Track recent parse results for on-the-fly queries

### Phase 3: Diagnostics (`diagnostics.ts` — expand)
Enhance with:
- Parse error extraction with line/column
- Undefined variable detection
- Type mismatch warnings (basic)
- Deprecated API warnings (wait/spawn/delay → task.*)
- ModuleScript missing return warning
- Server-only service in client warning

### Phase 4: Completion (`completion.ts` — enhance)
Already strong. Enhancements:
- Use proper parsed AST for scope instead of regex-based scope (already partially done with enriched scope)
- Snippet completions for boilerplate (mod-new, class-oop, event-connect, etc.)
- Better type resolution through method call chains (`obj:Method():OtherMethod()`)

### Phase 5: Semantic Tokens (`semanticTokens.ts` — new)
Port from `semantic_tokens.rs`:
- Walk AST and emit token types: namespace(0), type(1), class(2), function(3), property(4), method(5), variable(6), parameter(7), keyword(8), string(9), number(10), comment(11), operator(12), decorator(13)
- Delta-encode for LSP protocol

### Phase 6: Rename (`rename.ts` — new)
Port from `rename.rs`:
- `handlePrepareRename` — check symbol exists, return word range
- `handleRename` — find all occurrences in current file + workspace (for public symbols)
- Word-boundary-aware search

### Phase 7: Inlay Hints (`inlayHints.ts` — new)
Port from `inlay_hints.rs`:
- Type annotation hints for local variables (`local x = Vector3.new(...)` → `local x: Vector3`)
- Parameter name hints (future)

### Phase 8: Workspace Symbols (`workspaceSymbols.ts` — new)
Port from `symbols.rs::handle_workspace_symbols`:
- Search all open documents for symbols matching query
- Filter by function/class/enum/struct/variable
- Return `SymbolInformation[]` with URI locations

### Phase 9: Go-to-Definition (expand server.ts)
Enhance from current basic implementation:
- Cross-file go-to-def for imports (alias.Member → jump to target file at member declaration)
- Better symbol location accuracy using parsed AST

### Phase 10: Wire up `server.ts`
Add all new handlers:
- `onRenameRequest` + `onPrepareRename`
- `onSemanticTokensFull` + `onSemanticTokensRange`
- `onWorkspaceSymbol`
- `connection.languages.inlayHint.on(...)` (if supported) or `onInlayHint`
- Wire up DocumentStore for unified state
- Full server capabilities declaration

### Phase 11: Delete Rust LSP
Remove:
- `src/lsp/` directory entirely
- Remove `lsp-server`, `lsp-types` from `Cargo.toml` dependencies
- Remove `LspConfig` from `cli.rs` if it references LSP startup
- Remove LSP-related code from `lib.rs`
- Remove LSP references from `main.rs`

### Phase 12: Update VS Code Extension Client
- `extension.ts` already spawns `node out/lsp/server.js --stdio` — verify this works
- Ensure `package.json` scripts build `src/lsp/` files
- Test end-to-end

## File Structure After Migration

```
vscode-extension/
├── src/
│   ├── extension.ts          # VS Code extension client (unchanged)
│   ├── completions.ts         # Legacy? (review if still needed)
│   └── lsp/
│       ├── server.ts          # Main LSP server entry point
│       ├── ast.ts             # AST type definitions (new)
│       ├── parser.ts          # Lexer + Parser (rewritten)
│       ├── store.ts           # Document store (new)
│       ├── bindings.ts        # .wold loader (exists, solid)
│       ├── completion.ts      # Completion handler (enhanced)
│       ├── hover.ts           # Hover handler (exists)
│       ├── diagnostics.ts     # Diagnostics (enhanced)
│       ├── semanticTokens.ts  # Semantic tokens (new)
│       ├── rename.ts          # Rename handler (new)
│       ├── inlayHints.ts      # Inlay hints (new)
│       ├── workspaceSymbols.ts # Workspace symbols (new)
│       └── utils.ts           # Shared utilities (exists)
├── generated/
│   └── roblox.wold           # Roblox API bindings (exists)
└── package.json
```

The Rust `src/lsp/` directory is deleted entirely. Non-LSP Rust code (lexer, parser, generator, ast, etc.) remains for the CLI compiler.

## Execution Order

1. Create `ast.ts` — type definitions
2. Rewrite `parser.ts` — full lexer + recursive descent parser
3. Create `store.ts` — document store
4. Enhance `diagnostics.ts`
5. Create `semanticTokens.ts`
6. Create `rename.ts`
7. Create `inlayHints.ts`
8. Create `workspaceSymbols.ts`
9. Rewrite `server.ts` — wire everything, declare full capabilities
10. Enhance `completion.ts`
11. Delete `src/lsp/` (Rust)
12. Update `Cargo.toml` to remove lsp deps
13. Build & test
