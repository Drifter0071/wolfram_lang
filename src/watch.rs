use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::cli;

pub fn run_watch(input_dir: &Path, verbose: bool) {
    let out_root = Path::new("out");
    let input_dir = input_dir
        .canonicalize()
        .unwrap_or_else(|_| input_dir.to_path_buf());

    // Initial full transpile
    cli::transpile_project(&input_dir, out_root, verbose);
    let src_root = cli::resolve_src_root(&input_dir);

    let (tx, rx) = mpsc::channel();

    let mut watcher = match RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to create file watcher: {e}");
            return;
        }
    };

    if let Err(e) = watcher.watch(&input_dir, RecursiveMode::Recursive) {
        eprintln!("Failed to watch directory '{}': {e}", input_dir.display());
        return;
    }

    println!(
        "  Watching for changes in '{}'... (Ctrl+C to stop)\n",
        input_dir.display()
    );

    let mut last_event: Option<(Instant, PathBuf)> = None;
    let debounce = Duration::from_millis(200);

    loop {
        match rx.recv() {
            Ok(event) => match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) => {
                    for path in &event.paths {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if ext == "wrm"
                                || path.file_name().and_then(|n| n.to_str()) == Some("wolfram.toml")
                            {
                                let now = Instant::now();
                                let should_process = match &last_event {
                                    Some((t, p)) if p == path => now.duration_since(*t) >= debounce,
                                    _ => true,
                                };

                                if should_process {
                                    last_event = Some((now, path.clone()));

                                    if path.file_name().and_then(|n| n.to_str())
                                        == Some("wolfram.toml")
                                    {
                                        println!("  ↻  wolfram.toml changed — full recompile");
                                        cli::transpile_project(&input_dir, out_root, verbose);
                                    } else {
                                        let out = cli::make_out_path(path, &src_root, out_root);
                                        cli::transpile_file(path, &out, verbose);
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            Err(_) => break,
        }
    }
}
