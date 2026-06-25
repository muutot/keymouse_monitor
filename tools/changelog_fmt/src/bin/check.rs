//! Check CHANGELOG.md formatting rules.
//!
//! Usage:
//!     check-changelog CHANGELOG.md          # read from file
//!     cat CHANGELOG.md | check-changelog    # read from stdin
//!
//! Exits 0 if no violations, 1 otherwise.

use std::io::{Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (path, text) = if args.len() > 1 && args[1] != "-" {
        match std::fs::read_to_string(&args[1]) {
            Ok(t) => (args[1].clone(), t),
            Err(e) => {
                eprintln!("check-changelog: cannot read {}: {}", args[1], e);
                return ExitCode::FAILURE;
            }
        }
    } else {
        let mut buf = String::new();
        if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
            eprintln!("check-changelog: cannot read stdin: {}", e);
            return ExitCode::FAILURE;
        }
        ("<stdin>".to_string(), buf)
    };

    let errs = changelog_fmt::check(&text);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if errs.is_empty() {
        let _ = writeln!(out, "OK: {} passes all checks", path);
        ExitCode::SUCCESS
    } else {
        let _ = writeln!(out, "FAIL: {} violation(s) in {}", errs.len(), path);
        for e in &errs {
            let _ = writeln!(out, "{}", e);
        }
        ExitCode::FAILURE
    }
}
