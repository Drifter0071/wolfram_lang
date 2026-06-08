const { parseSource } = require("./out/lsp/parser");
const code = `local x = 5
local y = 10
function greet(name: string) {
    print(f"Hello {name}")
}
if (x > 0) {
    print("positive")
}
class Player {
    public function init(self) {
        self.score = 0
    }
}
import "../shared/Config" as Config
for i in range(0, 10) {
    print(i)
}
`;
const result = parseSource(code);
console.log("errors:", result.errors.length);
if (result.errors.length > 0) result.errors.forEach(e => console.log("  -", e));
console.log("symbols:", result.symbols.map(s => s.name + ":" + s.kind));
console.log("scope keys:", Array.from(result.scope.keys()));
