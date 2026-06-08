const { computeDiagnostics } = require("./out/lsp/diagnostics");
const code = `local Players = game:GetService("Players")
local burger = { name = "Cheeseburger", price = 5 }
local total = burger.price * 2
-- rojo sync config
-- Menu module
print(Players)
print(total)
`;
const diags = computeDiagnostics(code);
diags.forEach(d => console.log(`[${d.severity === 1 ? "ERROR" : "WARN"}] ${d.source}: ${d.message}`));
console.log("total:", diags.length, "diagnostics");
