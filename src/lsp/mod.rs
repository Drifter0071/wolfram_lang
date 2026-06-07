pub mod store;
pub mod bindings;
pub mod handlers;
pub mod server;
pub mod code_actions;
pub mod snippets;
pub mod rename;
pub mod inlay_hints;

pub fn run(bindings_path: Option<&str>) -> Result<(), String> {
    server::run(bindings_path)
}
