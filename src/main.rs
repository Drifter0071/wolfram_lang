mod cli;
mod watch;

use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
    let is_watch = args.iter().any(|a| a == "--watch" || a == "-w");
    let is_analyze = args.iter().any(|a| a == "--analyze" || a == "-a");

    let pos_args: Vec<&String> = args.iter().skip(1).filter(|a| !a.starts_with('-')).collect();

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

    if input.is_dir() {
        cli::transpile_project(input, out_root, verbose);
    } else if input.is_file() {
        cli::transpile_single(input, out_root, verbose);
    } else {
        eprintln!("Error: '{}' is not a file or directory.", input.display());
        cli::print_usage();
    }
}
