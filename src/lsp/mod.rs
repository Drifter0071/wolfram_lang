pub mod store;
pub mod bindings;
pub mod handlers;
pub mod server;

pub fn run() -> Result<(), String> {
    server::run()
}
