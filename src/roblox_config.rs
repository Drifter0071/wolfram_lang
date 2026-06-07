use crate::constants::normalize_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WolframConfig {
    #[serde(default)]
    pub roblox: RobloxProjectConfig,
    #[serde(default)]
    pub deployment: HashMap<String, String>,
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

// ─── Deployment Path Model ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DeploymentEntry {
    pub source_dir: String,
    pub service: String,
    pub sub_path: Vec<String>,
}

impl DeploymentEntry {
    pub fn instance_path(&self) -> String {
        let mut parts = vec![self.service.as_str()];
        parts.extend(self.sub_path.iter().map(|s| s.as_str()));
        parts.join(".")
    }
}

#[derive(Debug, Clone)]
pub struct ScriptLocation {
    pub service: String,
    pub instance_segments: Vec<String>,
    pub module_name: String,
}

impl ScriptLocation {
    pub fn instance_prefix(&self) -> Vec<String> {
        let mut v = vec![self.service.as_str()];
        v.extend(self.instance_segments.iter().map(|s| s.as_str()));
        v.into_iter().map(|s| s.to_string()).collect()
    }
}

// ─── Config Normalization ────────────────────────────────────────────

pub fn normalize_deployments(
    deployment_map: &HashMap<String, String>,
) -> Vec<DeploymentEntry> {
    let mut entries: Vec<DeploymentEntry> = Vec::new();
    for (key, val) in deployment_map {
        let norm_key = normalize_path(key);
        let norm_val = val.replace('\\', "/");
        let parts: Vec<&str> = norm_val.split('.').collect();
        if parts.is_empty() {
            continue;
        }
        entries.push(DeploymentEntry {
            source_dir: norm_key,
            service: parts[0].to_string(),
            sub_path: parts[1..].iter().map(|s| s.to_string()).collect(),
        });
    }
    entries
}

// ─── Instance Location Resolution ────────────────────────────────────

pub fn resolve_script_location(
    file_path: &str,
    deployments: &[DeploymentEntry],
    mappings: &[RobloxMapping],
) -> Option<ScriptLocation> {
    let normalized = normalize_path(file_path);

    // Try deployment map first
    for dep in deployments {
        let prefix = if dep.source_dir.ends_with('/') {
            dep.source_dir.clone()
        } else {
            format!("{}/", dep.source_dir)
        };

        if normalized == dep.source_dir
            || normalized.starts_with(&prefix)
        {
            let rel = normalized[dep.source_dir.len()..].trim_start_matches('/');
            let name = rel
                .rsplit('/')
                .next()
                .unwrap_or(rel)
                .strip_suffix(".wrm")
                .unwrap_or(rel)
                .to_string();

            let mut extra_segments: Vec<String> = if let Some(slash) = rel.rfind('/') {
                rel[..slash]
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect()
            } else {
                Vec::new()
            };

            let mut instance_segments = dep.sub_path.clone();
            instance_segments.append(&mut extra_segments);

            return Some(ScriptLocation {
                service: dep.service.clone(),
                instance_segments,
                module_name: name,
            });
        }
    }

    // Fall back to legacy mappings
    if let Some(resolved) = resolve_target_path_legacy(&normalized, mappings) {
        let service = resolved
            .target_instance
            .first()
            .cloned()
            .unwrap_or_default();
        let instance_segments = if resolved.target_instance.len() > 1 {
            resolved.target_instance[1..].to_vec()
        } else {
            Vec::new()
        };
        return Some(ScriptLocation {
            service,
            instance_segments,
            module_name: resolved.target_name,
        });
    }

    None
}

// ─── Path Strategy ───────────────────────────────────────────────────

pub enum RequireStrategy {
    CrossService {
        service_variable: String,
        instance_path: String,
        module_name: String,
    },
    Sibling {
        module_name: String,
    },
    DeepNested {
        service_variable: String,
        instance_path: String,
        module_name: String,
    },
    StarterPlayerRelative {
        chain: String,
    },
}

fn is_starter_player(service: &str) -> bool {
    service == "StarterPlayer" || service == "StarterCharacterScripts" || service == "StarterPlayerScripts"
}

pub fn get_require_path(
    source: &ScriptLocation,
    target: &ScriptLocation,
) -> (String, Option<String>) {
    // Scenario A: Cross-Service — different Roblox services
    if source.service != target.service {
        let mut parts = vec![target.service.as_str()];
        parts.extend(target.instance_segments.iter().map(|s| s.as_str()));
        parts.push(&target.module_name);
        let full = parts.join(".");
        return (full, Some(target.service.clone()));
    }

    // Same service
    let svc = &source.service;

    // Scenario D: StarterPlayer — paths change at runtime, must use script.Parent chain
    if is_starter_player(svc) {
        let relative_parts = build_relative_chain(source, target);
        return (relative_parts, None);
    }

    // Scenario B: Sibling — same parent instance (e.g., both directly under ReplicatedStorage.Shared)
    if source.instance_segments == target.instance_segments {
        let escaped = target.module_name.replace('.', "_");
        return (format!("script.Parent.{}", escaped), None);
    }

    // Scenario C: Deep nested same container — use absolute service variable path
    let mut parts = vec![svc.as_str()];
    parts.extend(target.instance_segments.iter().map(|s| s.as_str()));
    parts.push(&target.module_name);
    let full = parts.join(".");
    (full, Some(svc.clone()))
}

fn build_relative_chain(source: &ScriptLocation, target: &ScriptLocation) -> String {
    let src_len = source.instance_segments.len();
    let common_len = source
        .instance_segments
        .iter()
        .zip(target.instance_segments.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let ups = src_len - common_len;
    let mut chain = String::from("script");
    for _ in 0..=ups {
        chain.push_str(".Parent");
    }

    for seg in &target.instance_segments[common_len..] {
        chain.push('.');
        chain.push_str(seg);
    }

    chain.push('.');
    chain.push_str(&target.module_name.replace('.', "_"));

    chain
}

// ─── Legacy Path Resolution (wolfram.toml [roblox] mappings) ─────────

#[derive(Debug, Clone)]
pub struct ResolvedMapping {
    pub target_instance: Vec<String>,
    pub target_name: String,
}

fn resolve_target_path_legacy(
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
                let name = name.strip_suffix(".shared").unwrap_or(&name).to_string();

                let mut target_parts: Vec<String> =
                    mapping.target.split('.').map(|s| s.to_string()).collect();
                let subdir = if let Some(slash) = rest.rfind('/') {
                    let dir_part = &rest[..slash];
                    if !dir_part.is_empty() {
                        Some(
                            dir_part
                                .split('/')
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>(),
                        )
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
                let target_parts: Vec<String> =
                    mapping.target.split('.').map(|s| s.to_string()).collect();
                let name = normalized
                    .rsplit('/')
                    .next()
                    .unwrap_or(&normalized)
                    .strip_suffix(".wrm")
                    .unwrap_or(&normalized)
                    .to_string();
                let name = name.strip_suffix(".shared").unwrap_or(&name).to_string();
                return Some(ResolvedMapping {
                    target_instance: target_parts,
                    target_name: name,
                });
            }
        }
    }

    None
}

// ─── Import Resolution (combines deployment + legacy + Rojo) ─────────

pub fn resolve_import(
    importing_file: &str,
    import_path: &str,
    config: &RobloxProjectConfig,
    deployments: &[DeploymentEntry],
) -> Option<(String, Option<String>)> {
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
    let with_ext = if !resolved.ends_with(".wrm") {
        format!("{}.wrm", resolved)
    } else {
        resolved.clone()
    };

    let source_loc = resolve_script_location(&normalized_current, deployments, &config.mappings)?;
    let target_loc = resolve_script_location(&with_ext, deployments, &config.mappings)?;

    Some(get_require_path(&source_loc, &target_loc))
}

pub fn resolve_project_import(
    import_path: &str,
    config: &RobloxProjectConfig,
    deployments: &[DeploymentEntry],
) -> Option<(String, Option<String>)> {
    let clean = import_path.trim_start_matches("./").trim_start_matches('/');
    let candidates = if clean.starts_with("src/") {
        vec![
            format!("{}.wrm", clean),
            format!("{}.shared.wrm", clean),
            format!("{}.server.wrm", clean),
            format!("{}.client.wrm", clean),
        ]
    } else {
        vec![
            format!("src/{}.wrm", clean),
            format!("src/{}.shared.wrm", clean),
            format!("src/{}.server.wrm", clean),
            format!("src/{}.client.wrm", clean),
        ]
    };

    for cand in &candidates {
        if let Some(target) = resolve_script_location(cand, deployments, &config.mappings) {
            let mut parts = vec![target.service.clone()];
            parts.extend(target.instance_segments);
            parts.push(target.module_name.clone());
            return Some((parts.join("."), Some(target.service.clone())));
        }
    }
    None
}

pub fn extract_service_name(path: &str) -> &str {
    path.split('.').next().unwrap_or("")
}
