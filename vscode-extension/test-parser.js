const { parseSource } = require("./out/lsp/parser");

// Test 1: exponent ^
const r1 = parseSource("local result = 10 ^ 3");
console.log("1. exponent ^:", r1.errors.length === 0 ? "PASS" : "FAIL", r1.errors);

// Test 2: multi-var for
const r2 = parseSource("for k, v in pairs(tbl) {\n    print(k, v)\n}");
console.log("2. multi-var for:", r2.errors.length === 0 ? "PASS" : "FAIL", r2.errors);

// Test 3: multi-var local
const r3 = parseSource("local ok, err = pcall(function() {\n    riskyCall()\n})");
console.log("3. multi-var local:", r3.errors.length === 0 ? "PASS" : "FAIL", r3.errors);
console.log("   names:", r3.ast[0]?.kind === "Local" ? JSON.stringify(r3.ast[0].names) : "n/a");

// Test 4: // comment
const r4 = parseSource("local x = 1 // inline comment\nlocal y = 2");
console.log("4. // comment:", r4.errors.length === 0 ? "PASS" : "FAIL", r4.errors);
