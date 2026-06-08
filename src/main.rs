mod cli;
mod watch;

use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
    let is_watch = args.iter().any(|a| a == "--watch" || a == "-w");
    let is_analyze = args.iter().any(|a| a == "--analyze" || a == "-a");
    let is_test = args.iter().any(|a| a == "--test" || a == "-t");
    let is_lsp = args.iter().any(|a| a == "lsp");

    if is_lsp {
        let bindings_path = args
            .iter()
            .position(|a| a == "--bindings")
            .and_then(|i| args.get(i + 1))
            .map(|s| s.as_str());
        if let Err(e) = wolfram::lsp::run(bindings_path) {
            eprintln!("LSP error: {}", e);
        }
        return;
    }

    let pos_args: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();

    if pos_args.is_empty() {
        cli::print_usage();
        return;
    }

    let input = Path::new(pos_args[0]);
    let out_root = Path::new("out");

    if is_watch {
        if !input.is_dir() {
            eprintln!("Error: --watch requires a directory.");
            cli::print_usage();
            return;
        }
        watch::run_watch(input, verbose);
        return;
    }

    if is_analyze {
        if !input.is_file() {
            eprintln!("Error: --analyze requires a file path.");
            cli::print_usage();
            return;
        }
        let source = match std::fs::read_to_string(input) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading '{}': {e}", input.display());
                return;
            }
        };
        let result = wolfram::analyze::analyze(&source, &input.display().to_string());
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
        return;
    }

    if is_test {
        if !input.is_file() {
            eprintln!("Error: --test requires a file path.");
            cli::print_usage();
            return;
        }
        let source = match std::fs::read_to_string(input) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading '{}': {e}", input.display());
                return;
            }
        };
        let result = wolfram::tester::run_test(
            &source,
            &input.display().to_string(),
            &wolfram::tester::TestConfig::default(),
        );
        println!("\n🧪  Edge-Case Tester Results");
        println!("    file: {}", input.display());
        println!("    passed: {}, failed: {}, warnings: {}",
            result.stats.passed, result.stats.failed, result.stats.warnings);
        for e in &result.errors {
            println!("  ✗  ERROR [line {}:{}]: {}", e.line, e.column, e.message);
        }
        for w in &result.warnings {
            println!("  ⚠  WARNING [line {}:{}]: {}", w.line, w.column, w.message);
            if let Some(ref s) = w.suggestion {
                println!("      └─ suggestion: {s}");
            }
        }
        if result.passed {
            println!("  ✓  All checks passed!");
        } else {
            std::process::exit(1);
        }
        return;
    }

    if input.is_dir() {
        cli::transpile_project(input, out_root, verbose);
    } else if input.is_file() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let src_root = cli::resolve_src_root(&cwd);
        cli::transpile_single(input, &src_root, out_root, &cwd, verbose);
    } else {
        eprintln!("Error: '{}' is not a file or directory.", input.display());
        cli::print_usage();
    }
}
