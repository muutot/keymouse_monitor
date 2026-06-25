//! Reformat CHANGELOG.md entries to 88-char fill.
//!
//! Usage:
//!     format-changelog CHANGELOG.md        # read from file
//!     cat CHANGELOG.md | format-changelog  # read from stdin
//!
//! Prints the reformatted CHANGELOG to stdout.

use std::io::{Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let text = if args.len() > 1 && args[1] != "-" {
        match std::fs::read_to_string(&args[1]) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("format-changelog: cannot read {}: {}", args[1], e);
                return ExitCode::FAILURE;
            }
        }
    } else {
        let mut buf = String::new();
        if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
            eprintln!("format-changelog: cannot read stdin: {}", e);
            return ExitCode::FAILURE;
        }
        buf
    };
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if let Err(e) = out.write_all(changelog_fmt::format_text(&text).as_bytes()) {
        eprintln!("format-changelog: write failed: {}", e);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
