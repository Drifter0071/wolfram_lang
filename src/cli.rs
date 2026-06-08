use std::path::{Path, PathBuf};
use wolfram::roblox_config::{normalize_deployments, DeploymentEntry};
use wolfram::roblox_context::ScriptType;

/// If `project_root/src` exists, use it as the source root for output paths.
/// Otherwise return the project_root itself.
pub fn resolve_src_root(project_root: &Path) -> PathBuf {
    let src_dir = project_root.join("src");
    if src_dir.is_dir() {
        src_dir
    } else {
        project_root.to_path_buf()
    }
}

pub struct ProjectConfig {
    pub wolfram_toml: Option<wolfram::roblox_config::RobloxProjectConfig>,
    pub rojo_mappings: Option<Vec<wolfram::rojo_config::RojoPathMapping>>,
    pub deployments: Vec<DeploymentEntry>,
}

pub fn load_project_config(project_root: &Path) -> ProjectConfig {
    let mut config = ProjectConfig {
        wolfram_toml: None,
        rojo_mappings: None,
        deployments: Vec::new(),
    };

    let toml_path = project_root.join("wolfram.toml");
    if let Ok(raw) = std::fs::read_to_string(&toml_path) {
        if let Ok(full) = toml::from_str::<wolfram::roblox_config::WolframConfig>(&raw) {
            config.wolfram_toml = Some(full.roblox);
            config.deployments = normalize_deployments(&full.deployment);
        }
    }

    config.rojo_mappings = wolfram::rojo_config::load_rojo_mappings(project_root);
    config
}

/// Transpile one file simple wrapper, uses wolfram::transpile.
pub fn transpile_file(src_path: &Path, out_path: &Path, verbose: bool) {
    let rel = src_path.display().to_string();
    transpile_file_with_fn(src_path, out_path, &rel, verbose, |src, path| {
        wolfram::transpile(src, path)
    });
}

/// Recursively collect all .wrm files under a directory.
pub fn collect_wolfram_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_wolfram_files(&path));
            } else if path.extension().and_then(|e| e.to_str()) == Some("wrm") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Build the output path for a source file:
///   src_root/a/b/Foo.wrm  →  out_root/a/b/Foo.luau
pub fn make_out_path(src_path: &Path, src_root: &Path, out_root: &Path) -> PathBuf {
    let rel = src_path.strip_prefix(src_root).unwrap_or(src_path);
    let stem = rel.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let base = stem.strip_suffix(".shared").unwrap_or(stem);
    let parent = rel.parent().unwrap_or(Path::new(""));
    out_root.join(parent).join(format!("{}.luau", base))
}

/// Transpile all .wrm files under a directory into out_root.
/// Returns (ok_count, fail_count).
/// Delete .luau files in out_dir that don't have a corresponding .wrm in src_dir.
fn clean_orphaned_luau(src_dir: &Path, out_dir: &Path) -> usize {
    let mut cleaned = 0usize;
    let _ = walk_luau_files(out_dir, &mut |luau_path| {
        let rel = luau_path.strip_prefix(out_dir).unwrap_or(luau_path);
        let bare_str = rel.with_extension("").display().to_string();
        let ext_check = |ext: &str| -> bool {
            let candidate = format!("{}{}", bare_str, ext);
            src_dir.join(&candidate).exists()
        };
        let exists = ext_check(".wrm") || ext_check(".shared.wrm") || ext_check(".server.wrm") || ext_check(".client.wrm");
        if !exists {
            if std::fs::remove_file(luau_path).is_ok() {
                cleaned += 1;
            }
        }
    });
    cleaned
}

fn walk_luau_files(dir: &Path, f: &mut dyn FnMut(&Path)) -> std::io::Result<()> {
    if !dir.exists() { return Ok(()); }
    for entry in std::fs::read_dir(dir)? {
        let e = entry?;
        let path = e.path();
        if path.is_dir() { walk_luau_files(&path, f)?; }
        else if path.extension().map_or(false, |ext| ext == "luau") { f(&path); }
    }
    Ok(())
}

pub fn transpile_project(input: &Path, out_root: &Path, verbose: bool) -> (usize, usize) {
    println!("\n🔨  Wolfram Transpiler  ──  project mode");
    println!("    source : {}", input.display());
    println!("    output : {}/\n", out_root.display());

    // Clean orphaned .luau files
    let cleaned = clean_orphaned_luau(input, out_root);

    let files = collect_wolfram_files(input);
    if files.is_empty() {
        if cleaned > 0 {
            println!("  Cleaned {} orphaned .luau file(s).", cleaned);
        }
        println!("  No .wrm files found under '{}'.", input.display());
        return (0, 0);
    }

    let src_root = resolve_src_root(input);
    let project_config = load_project_config(input);
    let wcfg = project_config.wolfram_toml.as_ref();
    let rojo = project_config.rojo_mappings.as_deref();
    let deps = &project_config.deployments;
    let out_str = out_root.display().to_string();
    let mut ok = 0usize;
    let mut fail = 0usize;
    for src in &files {
        let out = make_out_path(src, &src_root, out_root);
        let src_str = src.display().to_string();
        let source_code = std::fs::read_to_string(src).unwrap_or_default();
        let rel = src.strip_prefix(input).unwrap_or(src).display().to_string();
        let script_type = ScriptType::from_filename(&rel);
        let result = if wcfg.is_some() || rojo.is_some() || !deps.is_empty() {
            wolfram::transpile_roblox(&source_code, &rel, wcfg, &rel, rojo, deps, &out_str)
        } else {
            wolfram::transpile(&source_code, &rel)
        };
        match result {
            Ok(luau) => {
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                if std::fs::write(&out, &luau).is_ok() {
                    println!(
                        "  ✓  {}  →  {}  [{}]",
                        src_str,
                        out.display(),
                        script_type.label()
                    );
                    let type_issues = wolfram::check_types(&source_code);
                    for msg in &type_issues {
                        println!("      {}", msg);
                    }
                    if verbose {
                        let warnings = wolfram::check_scope(&source_code, &rel);
                        for w in &warnings {
                            println!("  ⚠  {}", w);
                        }
                        if verbose && !warnings.is_empty() {
                            println!("\n--- SOURCE ({src_str}) ---\n{source_code}");
                            println!("--- LUAU ---\n{luau}\n");
                        }
                    }
                    ok += 1;
                } else {
                    println!("  ✗  Write failed: {} — {}", src_str, out.display());
                    fail += 1;
                }
            }
            Err(e) => {
                let first_line = e.lines().next().unwrap_or(&e);
                println!("  ✗  {} — {}", src_str, first_line);
                for extra in e.lines().skip(1) {
                    println!("      {}", extra);
                }
                fail += 1;
            }
        }
    }
    println!("\n  Done — {} succeeded, {} failed.", ok, fail);
    (ok, fail)
}

/// Transpile a single file into out_root, preserving directory structure relative to src_root.
pub fn transpile_single(
    input: &Path,
    src_root: &Path,
    out_root: &Path,
    project_root: &Path,
    verbose: bool,
) {
    println!("\n🔨  Wolfram Transpiler  ──  single file mode");
    let out = make_out_path(input, src_root, out_root);
    println!("    source : {}", input.display());
    println!("    output : {}\n", out.display());
    let cfg = load_project_config(project_root);
    let wcfg = cfg.wolfram_toml.as_ref();
    let rojo = cfg.rojo_mappings.as_deref();
    let deps = &cfg.deployments;
    let out_str = out_root.display().to_string();
    let rel = input
        .strip_prefix(project_root)
        .unwrap_or(input)
        .display()
        .to_string();
    if wcfg.is_some() || rojo.is_some() || !deps.is_empty() {
        transpile_file_with_roblox(input, &out, &rel, verbose, wcfg, rojo, deps, &out_str);
    } else {
        transpile_file_with_fn(input, &out, &rel, verbose, |src, path| {
            wolfram::transpile(src, path)
        });
    }
}

fn transpile_file_with_fn<F>(src_path: &Path, out_path: &Path, rel: &str, verbose: bool, f: F)
where
    F: Fn(&str, &str) -> Result<String, String>,
{
    let src_str = src_path.display().to_string();
    let out_str = out_path.display().to_string();
    let source_code = match std::fs::read_to_string(src_path) {
        Ok(s) => s,
        Err(e) => {
            println!("  ✗  {src_str}  →  {e}");
            return;
        }
    };
    match f(&source_code, rel) {
        Ok(luau) => {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            match std::fs::write(out_path, &luau) {
                Ok(_) => {
                    println!("  ✓  {src_str}  →  {out_str}");
                    let type_issues = wolfram::check_types(&source_code);
                    for msg in &type_issues {
                        println!("      {}", msg);
                    }
                    if verbose {
                        println!("\n--- SOURCE ---\n{source_code}");
                        println!("--- LUAU ---\n{luau}\n");
                    }
                }
                Err(e) => println!("  ✗  {src_str}  →  Write failed: {e}"),
            }
        }
        Err(e) => {
            let first_line = e.lines().next().unwrap_or(&e);
            println!("  ✗  {src_str}  →  {}", first_line);
            for extra in e.lines().skip(1) {
                println!("      {}", extra);
            }
        }
    }
}

fn transpile_file_with_roblox(
    src_path: &Path,
    out_path: &Path,
    rel: &str,
    verbose: bool,
    config: Option<&wolfram::roblox_config::RobloxProjectConfig>,
    rojo_mappings: Option<&[wolfram::rojo_config::RojoPathMapping]>,
    deployments: &[DeploymentEntry],
    out_dir: &str,
) {
    let src_str = src_path.display().to_string();
    let out_str = out_path.display().to_string();
    let source_code = match std::fs::read_to_string(src_path) {
        Ok(s) => s,
        Err(e) => {
            println!("  ✗  {src_str}  →  {e}");
            return;
        }
    };
    let result = wolfram::transpile_roblox(
        &source_code, rel, config, rel, rojo_mappings, deployments, out_dir,
    );
    match result {
        Ok(luau) => {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            match std::fs::write(out_path, &luau) {
                Ok(_) => {
                    println!("  ✓  {src_str}  →  {out_str}");
                    let type_issues = wolfram::check_types(&source_code);
                    for msg in &type_issues {
                        println!("      {}", msg);
                    }
                    if verbose {
                        println!("\n--- SOURCE ---\n{source_code}");
                        println!("--- LUAU ---\n{luau}\n");
                    }
                }
                Err(e) => println!("  ✗  {src_str}  →  Write failed: {e}"),
            }
        }
        Err(e) => {
            let first_line = e.lines().next().unwrap_or(&e);
            println!("  ✗  {src_str}  →  {}", first_line);
            for extra in e.lines().skip(1) {
                println!("      {}", extra);
            }
        }
    }
}

pub fn print_usage() {
    println!("Wolfram Transpiler — Python-like syntax → Luau");
    println!();
    println!("USAGE:");
    println!("  wolfram <file.wrm>              Transpile a single file  → out/<file>.luau");
    println!("  wolfram <project_dir/>          Transpile all .wrm        → out/**/**.luau");
    println!("  wolfram --watch <dir>           Watch dir for .wrm changes, auto-transpile");
    println!("  wolfram --analyze <file>        Output JSON AST + diagnostics to stdout");
    println!("  wolfram --test <file>           Run edge-case tests on a .wrm file");
}
