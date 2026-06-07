#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptType {
    Server,
    Client,
    Shared,
}

impl ScriptType {
    pub fn from_filename(path: &str) -> Self {
        let lower = path.to_lowercase();
        if lower.contains(".server.") || lower.ends_with(".server") {
            ScriptType::Server
        } else if lower.contains(".client.") || lower.ends_with(".client") {
            ScriptType::Client
        } else {
            ScriptType::Shared
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ScriptType::Server => "ServerScript",
            ScriptType::Client => "LocalScript",
            ScriptType::Shared => "ModuleScript",
        }
    }
}

const SERVER_ONLY_SERVICES: &[&str] = &[
    "ServerScriptService", "ServerStorage", "DataStoreService",
    "MessagingService", "PathfindingService",
];

const CLIENT_ONLY_SERVICES: &[&str] = &[
    "UserInputService", "GuiService", "HapticService",
    "ContextActionService", "StarterGui",
];

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
        ScriptType::Shared => None,
    }
}
