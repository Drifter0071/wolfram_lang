use crate::constants::normalize_path;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct RojoProject {
    #[allow(dead_code)]
    name: Option<String>,
    tree: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct RojoPathMapping {
    pub fs_path: String,
    pub instance_path: String,
    pub service: String,
}

pub fn load_rojo_mappings(project_root: &Path) -> Option<Vec<RojoPathMapping>> {
    let rojo_path = project_root.join("default.project.json");
    let raw = fs::read_to_string(&rojo_path).ok()?;
    let project: RojoProject = serde_json::from_str(&raw).ok()?;
    let mut mappings = Vec::new();
    walk_tree(
        &project.tree,
        &mut Vec::new(),
        "",
        project_root,
        &mut mappings,
    );
    if mappings.is_empty() {
        None
    } else {
        Some(mappings)
    }
}

fn walk_tree(
    node: &serde_json::Value,
    parent_keys: &mut Vec<String>,
    instance_prefix: &str,
    project_root: &Path,
    out: &mut Vec<RojoPathMapping>,
) {
    let obj = match node.as_object() {
        Some(o) => o,
        None => return,
    };

    let class_name = obj.get("$className").and_then(|v| v.as_str());
    let fs_path = obj.get("$path").and_then(|v| v.as_str());

    for (key, value) in obj {
        if key.starts_with('$') {
            continue;
        }
        if !value.is_object() {
            continue;
        }

        parent_keys.push(key.clone());
        let child_instance = if instance_prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", instance_prefix, key)
        };

        let is_service = value
            .get("$className")
            .and_then(|v| v.as_str())
            .map(|c| is_roblox_service(c))
            .unwrap_or(false);

        if is_service {
            let service_name = key.clone();
            parent_keys.pop();
            // Walk children with new service context
            walk_children_of_service(&value, &service_name, project_root, out);
            continue;
        }

        if let Some(path) = value.get("$path").and_then(|v| v.as_str()) {
            let instance = if instance_prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", instance_prefix, key)
            };
            let service = extract_service_from_path(&instance);
            let normalized_fs = path.replace('\\', "/");
            out.push(RojoPathMapping {
                fs_path: normalized_fs,
                instance_path: instance,
                service,
            });
        } else {
            walk_tree(value, parent_keys, &child_instance, project_root, out);
        }

        parent_keys.pop();
    }

    // Also check $className at this level for service detection
    let _ = class_name;
    let _ = fs_path;
}

fn walk_children_of_service(
    node: &serde_json::Value,
    service_name: &str,
    _project_root: &Path,
    out: &mut Vec<RojoPathMapping>,
) {
    let obj = match node.as_object() {
        Some(o) => o,
        None => return,
    };

    // Collect all $path entries under this service
    let mut stack: Vec<(String, &serde_json::Value)> = Vec::new();
    for (key, value) in obj {
        if key.starts_with('$') {
            continue;
        }
        if !value.is_object() {
            continue;
        }
        stack.push((key.clone(), value));
    }

    while let Some((key, value)) = stack.pop() {
        if let Some(path) = value.get("$path").and_then(|v| v.as_str()) {
            let instance = format!("{}.{}", service_name, key);
            let normalized_fs = path.replace('\\', "/");
            out.push(RojoPathMapping {
                fs_path: normalized_fs,
                instance_path: instance,
                service: service_name.to_string(),
            });
        } else if let Some(children) = value.as_object() {
            for (ck, cv) in children {
                if ck.starts_with('$') {
                    continue;
                }
                if !cv.is_object() {
                    continue;
                }
                let full_key = format!("{}.{}", key, ck);
                stack.push((full_key, cv));
            }
        }
    }
}

fn is_roblox_service(class_name: &str) -> bool {
    matches!(
        class_name,
        "ReplicatedStorage"
            | "ServerScriptService"
            | "ServerStorage"
            | "StarterPlayer"
            | "StarterGui"
            | "StarterPack"
            | "Lighting"
            | "SoundService"
            | "RunService"
            | "Workspace"
            | "Players"
            | "Teams"
            | "Chat"
            | "HttpService"
            | "TeleportService"
            | "MarketplaceService"
            | "DataStoreService"
            | "MessagingService"
            | "PathfindingService"
            | "PhysicsService"
            | "CollectionService"
            | "TweenService"
            | "UserInputService"
            | "ContextActionService"
            | "LocalizationService"
            | "SocialService"
            | "GroupService"
            | "PolicyService"
            | "AnalyticsService"
            | "AvatarEditorService"
            | "BadgeService"
            | "MemoryStoreService"
            | "TextService"
            | "GuiService"
            | "HapticService"
    )
}

fn extract_service_from_path(instance_path: &str) -> String {
    instance_path.split('.').next().unwrap_or("").to_string()
}

impl RojoPathMapping {
    pub fn resolve_import_to_require(
        mappings: &[RojoPathMapping],
        importing_file: &str,
        import_path: &str,
        out_dir: &str,
    ) -> Option<(String, String)> {
        // Normalize importing file path to be relative to project root
        let normalized_current = importing_file.replace('\\', "/");
        let clean_import = import_path.trim_start_matches("./").trim_start_matches('/');

        // Resolve the import relative to the importing file's directory
        let base_dir = if let Some(slash) = normalized_current.rfind('/') {
            &normalized_current[..slash]
        } else {
            ""
        };

        let resolved = if clean_import.starts_with('/') {
            clean_import.to_string()
        } else if base_dir.is_empty() {
            clean_import.to_string()
        } else {
            format!("{}/{}", base_dir, clean_import)
        };

        // Normalize .. and .
        let resolved = normalize_path(&resolved);

        // Find the mapping for the importing file to get its service context
        let current_fs = normalized_current.trim_start_matches("src/").to_string();
        let _current_out = format!("{}/{}", out_dir, current_fs).replace(".wrm", ".luau");

        // Find mapping for the import target
        let import_out = format!("{}/{}", out_dir, resolved).replace(".wrm", ".luau");

        // Try to find mappings
        let current_mapping = mappings.iter().find(|m| {
            import_out.starts_with(&format!("{}/", m.fs_path)) || import_out == m.fs_path
        });

        let import_mapping = mappings.iter().find(|m| {
            let target_out = format!("{}/{}", out_dir, resolved).replace(".wrm", ".luau");
            target_out.starts_with(&format!("{}/", m.fs_path)) || target_out == m.fs_path
        });

        match (current_mapping, import_mapping) {
            (Some(current), Some(import)) => {
                let module_name = resolved
                    .rsplit('/')
                    .next()
                    .unwrap_or(&resolved)
                    .strip_suffix(".wrm")
                    .unwrap_or(&resolved)
                    .replace('.', "_");

                if current.service == import.service
                    && current.instance_path == import.instance_path.split('.').next().unwrap_or("")
                {
                    // Same service and same root folder — use script.Parent
                    Some((
                        format!("script.Parent.{}", module_name),
                        current.service.clone(),
                    ))
                } else {
                    let path = format!("{}.{}", import.instance_path, module_name);
                    Some((path, import.service.clone()))
                }
            }
            (None, Some(import)) => {
                let module_name = resolved
                    .rsplit('/')
                    .next()
                    .unwrap_or(&resolved)
                    .strip_suffix(".wrm")
                    .unwrap_or(&resolved)
                    .replace('.', "_");
                let path = format!("{}.{}", import.instance_path, module_name);
                Some((path, import.service.clone()))
            }
            _ => None,
        }
    }
}
