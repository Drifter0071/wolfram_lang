use std::path::{Path, PathBuf};

/// Transpile one file, write output to out_path, print status.
pub fn transpile_file(src_path: &Path, out_path: &Path, verbose: bool) {
    let src_str = src_path.display().to_string();
    let out_str = out_path.display().to_string();

    let source_code = match std::fs::read_to_string(src_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  ✗  {src_str}  →  {e}");
            return;
        }
    };

    match wolfram::transpile(&source_code, &src_str) {
        Ok(luau) => {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            match std::fs::write(out_path, &luau) {
                Ok(_) => {
                    println!("  ✓  {src_str}  →  {out_str}");
                    if verbose {
                        println!("\n--- SOURCE ---\n{source_code}");
                        println!("--- LUAU ---\n{luau}\n");
                    }
                }
                Err(e) => eprintln!("  ✗  Write failed for '{out_str}': {e}"),
            }
        }
        Err(e) => eprintln!("  ✗  {e}"),
    }
}

/// Recursively collect all .wol files under a directory.
pub fn collect_wolfram_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_wolfram_files(&path));
            } else if path.extension().and_then(|e| e.to_str()) == Some("wol") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Build the output path for a source file:
///   src_root/a/b/Foo.wol  →  out_root/a/b/Foo.luau
pub fn make_out_path(src_path: &Path, src_root: &Path, out_root: &Path) -> PathBuf {
    let rel = src_path.strip_prefix(src_root).unwrap_or(src_path);
    let mut out = out_root.join(rel);
    out.set_extension("luau");
    out
}

/// Transpile all .wol files under a directory into out_root.
/// Returns (ok_count, fail_count).
pub fn transpile_project(input: &Path, out_root: &Path, verbose: bool) -> (usize, usize) {
    println!("\n🔨  Wolfram Transpiler  ──  project mode");
    println!("    source : {}", input.display());
    println!("    output : {}/\n", out_root.display());

    let files = collect_wolfram_files(input);
    if files.is_empty() {
        println!("  No .wol files found under '{}'.", input.display());
        return (0, 0);
    }

    let mut ok = 0usize;
    let mut fail = 0usize;
    for src in &files {
        let out = make_out_path(src, input, out_root);
        let src_str = src.display().to_string();
        let source_code = std::fs::read_to_string(src).unwrap_or_default();
        match wolfram::transpile(&source_code, &src_str) {
            Ok(luau) => {
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                if std::fs::write(&out, &luau).is_ok() {
                    println!("  ✓  {}  →  {}", src_str, out.display());
                    if verbose {
                        println!("\n--- SOURCE ({src_str}) ---\n{source_code}");
                        println!("--- LUAU ---\n{luau}\n");
                    }
                    ok += 1;
                } else {
                    eprintln!("  ✗  Write failed: {}", out.display());
                    fail += 1;
                }
            }
            Err(e) => {
                eprintln!("  ✗  {e}");
                fail += 1;
            }
        }
    }
    println!("\n  Done — {} succeeded, {} failed.", ok, fail);
    (ok, fail)
}

/// Transpile a single file into out_root.
pub fn transpile_single(input: &Path, out_root: &Path, verbose: bool) {
    println!("\n🔨  Wolfram Transpiler  ──  single file mode");
    let file_name = input.file_name().unwrap_or_default();
    let out = out_root.join(file_name).with_extension("luau");
    println!("    source : {}", input.display());
    println!("    output : {}\n", out.display());
    transpile_file(input, &out, verbose);
}

pub fn print_usage() {
    println!("Wolfram Transpiler — Python-like syntax → Luau");
    println!();
    println!("USAGE:");
    println!("  wolfram <file.wol>              Transpile a single file  → out/<file>.luau");
    println!("  wolfram <project_dir/>          Transpile all .wol        → out/**/**.luau");
    println!("  wolfram --watch <dir>           Watch dir for .wol changes, auto-transpile");
    println!("  wolfram --analyze <file>        Output JSON AST + diagnostics to stdout");
    println!("  wolfram <path> --verbose        Also print source + generated code");
}
