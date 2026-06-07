pub mod bindings;
pub mod code_actions;
pub mod handlers;
pub mod inlay_hints;
pub mod rename;
pub mod semantic_tokens;
pub mod server;
pub mod snippets;
pub mod store;
pub mod symbols;

pub fn run(bindings_path: Option<&str>) -> Result<(), String> {
    server::run(bindings_path)
}
