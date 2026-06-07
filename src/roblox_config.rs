use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WolframConfig {
    #[serde(default)]
    pub roblox: RobloxProjectConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RobloxProjectConfig {
    #[serde(default)]
    pub mappings: Vec<RobloxMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobloxMapping {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedMapping {
    pub target_instance: Vec<String>,
    pub target_name: String,
}

pub fn resolve_target_path(
    file_path: &str,
    mappings: &[RobloxMapping],
) -> Option<ResolvedMapping> {
    let normalized = file_path.replace('\\', "/");

    for mapping in mappings {
        let source = mapping.source.replace('\\', "/");

        if source.contains('*') {
            let prefix = match source.find('*') {
                Some(0) => String::new(),
                Some(pos) => source[..pos].to_string(),
                None => source.clone(),
            };

            if normalized.starts_with(&prefix) {
                let rest = &normalized[prefix.len()..];
                let rest = rest.trim_start_matches('/');

                let name = if let Some(slash) = rest.rfind('/') {
                    rest[slash + 1..].to_string()
                } else {
                    rest.to_string()
                };
                let name = name.strip_suffix(".wrm").unwrap_or(&name).to_string();

                let mut target_parts: Vec<String> = mapping.target.split('.').map(|s| s.to_string()).collect();
                let subdir = if let Some(slash) = rest.rfind('/') {
                    let dir_part = &rest[..slash];
                    if !dir_part.is_empty() {
                        Some(dir_part.split('/').map(|s| s.to_string()).collect::<Vec<_>>())
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(sub) = subdir {
                    for part in sub {
                        target_parts.push(part);
                    }
                }

                return Some(ResolvedMapping {
                    target_instance: target_parts.clone(),
                    target_name: name,
                });
            }
        } else {
            if normalized == source || normalized == format!("{}.wrm", source) {
                let target_parts: Vec<String> = mapping.target.split('.').map(|s| s.to_string()).collect();
                let name = normalized.rsplit('/').next().unwrap_or(&normalized)
                    .strip_suffix(".wrm").unwrap_or(&normalized).to_string();
                return Some(ResolvedMapping {
                    target_instance: target_parts,
                    target_name: name,
                });
            }
        }
    }

    None
}

fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => continue,
            ".." => { parts.pop(); }
            _ => parts.push(seg),
        }
    }
    parts.join("/")
}

pub fn resolve_import(
    importing_file: &str,
    import_path: &str,
    mappings: &[RobloxMapping],
) -> Option<String> {
    let normalized_current = normalize_path(importing_file);
    let clean_path = import_path.trim_start_matches("./").trim_start_matches('/');

    let base_dir = if let Some(slash) = normalized_current.rfind('/') {
        &normalized_current[..slash]
    } else {
        ""
    };

    let resolved = if clean_path.starts_with('/') {
        clean_path.to_string()
    } else if base_dir.is_empty() {
        clean_path.to_string()
    } else {
        format!("{}/{}", base_dir, clean_path)
    };

    let resolved = normalize_path(&resolved);

    if !resolved.ends_with(".wrm") {
        let with_ext = format!("{}.wrm", resolved);
        if let (Some(current_target), Some(import_target)) = (
            resolve_target_path(&normalized_current, mappings),
            resolve_target_path(&with_ext, mappings),
        ) {
            return Some(build_require_path(&current_target.target_instance, &import_target));
        }
    }

    let (current_target, import_target) = (
        resolve_target_path(&normalized_current, mappings)?,
        resolve_target_path(&resolved, mappings)?,
    );
    Some(build_require_path(&current_target.target_instance, &import_target))
}

fn build_require_path(current: &[String], imported: &ResolvedMapping) -> String {
    let instance_path = imported.target_instance.join(".");
    let escaped_name = imported.target_name.replace('.', "_");

    if current == imported.target_instance.as_slice() {
        format!("require(script.Parent.{})", escaped_name)
    } else {
        format!("require(game.{}.{})", instance_path, escaped_name)
    }
}
