pub mod store;
pub mod bindings;
pub mod handlers;
pub mod server;

pub fn run(bindings_path: Option<&str>) -> Result<(), String> {
    server::run(bindings_path)
}
