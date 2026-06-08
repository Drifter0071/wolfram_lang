use crate::constants::{CLIENT_ONLY_SERVICES, SERVER_ONLY_SERVICES};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptType {
    Server,
    Client,
    Module,
}

impl ScriptType {
    pub fn from_filename(path: &str) -> Self {
        let lower = path.to_lowercase();
        if lower.contains(".server.") || lower.ends_with(".server") {
            ScriptType::Server
        } else if lower.contains(".client.") || lower.ends_with(".client") {
            ScriptType::Client
        } else {
            ScriptType::Module
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ScriptType::Server => "ServerScript",
            ScriptType::Client => "LocalScript",
            ScriptType::Module => "ModuleScript",
        }
    }
}

pub fn check_api_access(script_type: ScriptType, service_name: &str) -> Option<String> {
    match script_type {
        ScriptType::Client => {
            if SERVER_ONLY_SERVICES.contains(&service_name) {
                Some(format!(
                    "{} is server-only. Cannot access from a LocalScript ({}).",
                    service_name,
                    script_type.label()
                ))
            } else {
                None
            }
        }
        ScriptType::Server => {
            if service_name == "Players" {
                // Players.LocalPlayer is not accessible on server
                None
            } else if CLIENT_ONLY_SERVICES.contains(&service_name) {
                Some(format!(
                    "{} is client-only. Server scripts should not depend on client services.",
                    service_name
                ))
            } else {
                None
            }
        }
        ScriptType::Module => None,
    }
}
