//! Format and check CHANGELOG.md entries to 88-char fill.
//!
//! Two rules enforced (both use the same display-length measurement, where
//! `[hash](url)` counts as 0):
//!
//!   (a) A line containing hash link(s) must also contain descriptive text
//!       (no hash-only lines).
//!   (b) A continuation line whose first word would fit on the previous line
//!       (after the 88-char display-width rule) is illegal — it must be merged
//!       up.

use regex::Regex;
use std::sync::OnceLock;

pub const WIDTH: usize = 88;
pub const EM_DASH: &str = " \u{2014} ";

static HASH_LINK_RE: OnceLock<Regex> = OnceLock::new();
static ANY_LINK_RE: OnceLock<Regex> = OnceLock::new();
static HEADING_RE: OnceLock<Regex> = OnceLock::new();
static TABLE_SEP_RE: OnceLock<Regex> = OnceLock::new();
static HASH_TAIL_PARENS_RE: OnceLock<Regex> = OnceLock::new();
static TRAILING_DASH_RE: OnceLock<Regex> = OnceLock::new();
static HASH_LONE_STRIP_RE: OnceLock<Regex> = OnceLock::new();
static WORD_CHAR_RE: OnceLock<Regex> = OnceLock::new();
static BULLET_PREFIX_RE: OnceLock<Regex> = OnceLock::new();

fn hash_link_re() -> &'static Regex {
    HASH_LINK_RE.get_or_init(|| Regex::new(r"\[`?([0-9a-f]{7,})`?\]\([^)]*\)").unwrap())
}

fn any_link_re() -> &'static Regex {
    ANY_LINK_RE.get_or_init(|| Regex::new(r"\[([^\]]*)\]\([^)]*\)").unwrap())
}

fn heading_re() -> &'static Regex {
    HEADING_RE.get_or_init(|| Regex::new(r"^#{1,6}\s").unwrap())
}

fn table_sep_re() -> &'static Regex {
    TABLE_SEP_RE.get_or_init(|| Regex::new(r"^\|---").unwrap())
}

/// Strips ` — (, , , , )` style artifacts left by removing hash links from
/// a legacy ` — ([h1], [h2], [h3])` literal tail.
fn hash_tail_parens_re() -> &'static Regex {
    HASH_TAIL_PARENS_RE
        .get_or_init(|| Regex::new(r"\s*—\s*\(\s*(?:,\s*)*\)\s*$").unwrap())
}

fn trailing_dash_re() -> &'static Regex {
    TRAILING_DASH_RE.get_or_init(|| Regex::new(r"\s*—\s*$").unwrap())
}

fn hash_lone_strip_re() -> &'static Regex {
    HASH_LONE_STRIP_RE
        .get_or_init(|| Regex::new(r"[—–\-]\s*$").unwrap())
}

fn word_char_re() -> &'static Regex {
    WORD_CHAR_RE.get_or_init(|| Regex::new(r"[a-zA-Z0-9]").unwrap())
}

fn bullet_prefix_re() -> &'static Regex {
    BULLET_PREFIX_RE.get_or_init(|| Regex::new(r"^(- \([^)]*\)\s+)").unwrap())
}

/// Length of `line` for the 88-char rule, with hash links counted as 0.
pub fn disp_len(line: &str) -> usize {
    let s = hash_link_re().replace_all(line, "");
    let s = any_link_re().replace_all(&s, |caps: &regex::Captures| caps[1].to_string());
    s.chars().count()
}

fn looks_like_bullet(stripped: &str) -> bool {
    stripped.trim_start().starts_with("- ") || stripped.trim_start().starts_with("* ")
}

fn is_continuation(stripped: &str) -> bool {
    stripped.starts_with("  ") && !looks_like_bullet(stripped)
}

fn is_meta(stripped: &str) -> bool {
    if stripped.is_empty() {
        return true;
    }
    if heading_re().is_match(stripped) {
        return true;
    }
    if stripped.starts_with("```") {
        return true;
    }
    if table_sep_re().is_match(stripped) {
        return true;
    }
    if stripped.starts_with('>') {
        return true;
    }
    if stripped.starts_with('|') {
        return true;
    }
    false
}

/// Check a CHANGELOG for the two formatting rules. Returns human-readable
/// error messages; empty list means OK.
pub fn check(text: &str) -> Vec<String> {
    let mut errs = Vec::new();
    let mut in_entry = false;
    let mut prev_display = String::new();
    let mut in_code_block = false;

    for (i, raw) in text.lines().enumerate() {
        let stripped = raw.trim_end_matches('\n');

        if stripped.starts_with("```") {
            in_code_block = !in_code_block;
        }
        if in_code_block {
            continue;
        }

        let no_hash = hash_link_re().replace_all(stripped, "");
        let display_text = any_link_re().replace_all(&no_hash, |caps: &regex::Captures| {
            caps[1].to_string()
        });

        // (a) hash link on a line with no descriptive text
        let hashes: Vec<_> = hash_link_re().find_iter(stripped).collect();
        if !hashes.is_empty() {
            let text_only = any_link_re().replace_all(stripped, "").into_owned();
            let text_only = text_only.trim().to_string();
            let text_only = hash_lone_strip_re()
                .replace(text_only.as_str(), "")
                .trim()
                .to_string();
            if text_only.is_empty() {
                errs.push(format!("  HASH-ALONE L{}: {:?}", i + 1, stripped));
            }
        }

        // (b) continuation line with mergeable word
        let is_b = looks_like_bullet(stripped);
        let is_c = is_continuation(stripped);
        let is_m = is_meta(stripped);
        if is_c && !is_m && in_entry {
            let cont_words: Vec<&str> = display_text.trim().split_whitespace().collect();
            if let Some(first_word) = cont_words.first() {
                if word_char_re().is_match(first_word) {
                    let merged_len = prev_display.chars().count() + 1 + first_word.chars().count();
                    if merged_len <= WIDTH {
                        errs.push(format!(
                            "  MERGE-UP L{}: {:?} fits on L{} (merged={} <= {}): {:?}",
                            i + 1,
                            first_word,
                            i,
                            merged_len,
                            WIDTH,
                            stripped
                        ));
                    }
                }
            }
        }

        if is_b {
            in_entry = true;
        } else if is_m {
            in_entry = false;
        }

        if is_b || is_c {
            prev_display = display_text.to_string();
        } else if !is_m {
            prev_display.clear();
        }
    }

    errs
}

fn collect_hash_links(body: &str) -> Vec<String> {
    hash_link_re()
        .find_iter(body)
        .map(|m| m.as_str().to_string())
        .collect()
}

fn strip_hash_links(body: &str) -> String {
    let s = hash_link_re().replace_all(body, "");
    let s = hash_tail_parens_re().replace(&s, "");
    let s = trailing_dash_re().replace(&s, "");
    s.trim().to_string()
}

fn split_prefix(bullet: &str) -> (String, String) {
    if let Some(caps) = bullet_prefix_re().captures(bullet) {
        let prefix = caps.get(1).unwrap().as_str().to_string();
        let rest = bullet[prefix.len()..].to_string();
        (prefix, rest)
    } else {
        ("- ".to_string(), bullet[2..].to_string())
    }
}

fn build_tail(hashes: &[String]) -> String {
    if hashes.is_empty() {
        return String::new();
    }
    if hashes.len() == 1 {
        return format!("{}{}", EM_DASH, hashes[0]);
    }
    format!("{}({})", EM_DASH, hashes.join(", "))
}

fn tail_disp(tail: &str) -> usize {
    if tail.is_empty() {
        return 0;
    }
    let s = hash_link_re().replace_all(tail, "");
    s.chars().count()
}

/// Reflow a single entry. Returns formatted lines (no trailing newline).
pub fn format_entry(bullet: &str, cont_texts: &[&str]) -> Vec<String> {
    let (prefix, rest) = split_prefix(bullet);
    let body_full = std::iter::once(rest.as_str())
        .chain(cont_texts.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let body = body_full.trim();
    let hashes = collect_hash_links(body);
    let desc = strip_hash_links(body);

    let tail = build_tail(&hashes);
    let t_w = tail_disp(&tail);
    let prefix_w = disp_len(&prefix);
    let cont_indent = 2usize;

    let words: Vec<&str> = desc.split_whitespace().collect();

    if words.is_empty() {
        if !tail.is_empty() {
            let line = prefix.trim_end();
            if !line.is_empty() {
                return vec![format!("{}{}", line, tail)];
            }
            return vec![tail.trim_start().to_string()];
        }
        return vec![prefix.trim_end().to_string()];
    }

    // Greedy word-fill.
    let n = words.len();
    let mut lines: Vec<String> = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    let mut cur_w: usize = 0;
    for (idx, w) in words.iter().enumerate() {
        let is_last = idx == n - 1;
        if cur.is_empty() {
            // Start a new line.
            cur.push(w);
            cur_w = w.chars().count();
            continue;
        }
        let margin = if lines.is_empty() { prefix_w } else { cont_indent };
        let reserved = if is_last { t_w } else { 0 };
        // cur_w + 1 + len(w) + reserved <= WIDTH - margin
        let needed = cur_w + 1 + w.chars().count() + reserved;
        if needed <= WIDTH - margin {
            cur.push(w);
            cur_w += 1 + w.chars().count();
        } else {
            lines.push(cur.join(" "));
            cur = vec![w];
            cur_w = w.chars().count();
        }
    }
    if !cur.is_empty() {
        lines.push(cur.join(" "));
    }

    // Append the tail to the last line.
    if !tail.is_empty() && !lines.is_empty() {
        if let Some(last) = lines.last_mut() {
            last.push_str(&tail);
        }
    }

    // Emit with prefix on first line and 2-space indent on continuations.
    let mut out: Vec<String> = Vec::new();
    for (i, desc_line) in lines.iter().enumerate() {
        if i == 0 {
            out.push(format!("{}{}", prefix, desc_line));
        } else {
            out.push(format!("  {}", desc_line));
        }
    }
    out
}

/// Format an entire CHANGELOG (read as text). Returns the reformatted text.
///
/// Preserves the original line terminators (LF or CRLF) for every non-entry
/// line. Each reformatted entry line is emitted with `\n`; callers that need
/// CRLF output should pre-convert the input.
pub fn format_text(text: &str) -> String {
    // Walk the original text and record (line_content, line_terminator) for
    // each line. This lets us preserve CRLF vs LF for non-entry lines.
    let mut splits: Vec<(&str, &str)> = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(idx) = rest.find('\n') {
            let (line, after) = rest.split_at(idx);
            let term = if after.starts_with("\r\n") {
                "\r\n"
            } else {
                "\n"
            };
            let after_term = if term.len() == 2 { &after[2..] } else { &after[1..] };
            splits.push((line, term));
            rest = after_term;
        } else {
            splits.push((rest, ""));
            rest = "";
        }
    }
    let line_strs: Vec<String> = splits.iter().map(|(l, _)| l.to_string()).collect();
    let entries = collect_entries(&line_strs);
    let mut new_lines: Vec<String> = splits
        .iter()
        .map(|(l, t)| format!("{}{}", l, t))
        .collect();
    // Replace in reverse so indices don't shift.
    let mut sorted: Vec<&(usize, usize, Vec<String>)> = entries.iter().collect();
    sorted.sort_by_key(|e| std::cmp::Reverse(e.0));
    for (start, end, texts) in sorted {
        let bullet = texts[0].clone();
        let cont: Vec<&str> = texts[1..]
            .iter()
            .map(|t| {
                if let Some(rest) = t.strip_prefix("  ") {
                    rest
                } else {
                    t.as_str()
                }
            })
            .collect();
        let reformatted = format_entry(&bullet, &cont);
        let replacement: Vec<String> = reformatted
            .into_iter()
            .map(|ln| format!("{}\n", ln))
            .collect();
        new_lines.splice(*start..=*end, replacement);
    }
    new_lines.join("")
}

fn collect_entries(lines: &[String]) -> Vec<(usize, usize, Vec<String>)> {
    let mut entries: Vec<(usize, usize, Vec<String>)> = Vec::new();
    let mut cur_start: Option<usize> = None;
    let mut cur_end: Option<usize> = None;
    let mut cur_texts: Vec<String> = Vec::new();
    for (i, raw) in lines.iter().enumerate() {
        let stripped = raw.trim_end_matches('\n');
        if looks_like_bullet(stripped) {
            if let Some(s) = cur_start {
                entries.push((s, cur_end.unwrap(), std::mem::take(&mut cur_texts)));
            }
            cur_start = Some(i);
            cur_end = Some(i);
            cur_texts = vec![stripped.to_string()];
        } else if is_continuation(stripped) && cur_start.is_some() {
            cur_end = Some(i);
            cur_texts.push(stripped.to_string());
        } else {
            if let Some(s) = cur_start {
                entries.push((s, cur_end.unwrap(), std::mem::take(&mut cur_texts)));
                cur_start = None;
                cur_texts.clear();
            }
        }
    }
    if let Some(s) = cur_start {
        entries.push((s, cur_end.unwrap(), cur_texts));
    }
    entries
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ----- hash-link regex -----

    #[test]
    fn hash_link_bare() {
        let out = collect_hash_links("[abc1234](http://x)");
        assert_eq!(out, vec!["[abc1234](http://x)"]);
    }

    #[test]
    fn hash_link_backticked() {
        let out = collect_hash_links("[`abc1234`](http://x)");
        assert_eq!(out, vec!["[`abc1234`](http://x)"]);
    }

    #[test]
    fn hash_link_multiple() {
        let out = collect_hash_links("see [`aaa1111`](u) and [`bbb2222`](u)");
        assert_eq!(out, vec!["[`aaa1111`](u)", "[`bbb2222`](u)"]);
    }

    // ----- format_entry -----

    #[test]
    fn format_short_single_hash_inline() {
        let out = format_entry("- (cli) add -h flag — [`abc1234`](http://x)", &[]);
        assert_eq!(out, vec!["- (cli) add -h flag — [`abc1234`](http://x)"]);
    }

    #[test]
    fn format_short_multi_hash_inline() {
        let out = format_entry(
            "- (db) merge SQLite fallback — ([`aaa1111`](u), [`bbb2222`](u))",
            &[],
        );
        assert_eq!(
            out,
            vec!["- (db) merge SQLite fallback — ([`aaa1111`](u), [`bbb2222`](u))"]
        );
    }

    #[test]
    fn format_long_desc_wraps() {
        let bullet = "- (frontend) fix export/import/version using relative URLs — \
                       use `${API_URL}` prefix so they work from external static hosting — \
                       [`abc1234`](http://x)";
        let out = format_entry(bullet, &[]);
        // All lines <= 88 display chars.
        for line in &out {
            assert!(
                disp_len(line) <= 88,
                "line too long: {:?} ({} chars)",
                line,
                disp_len(line)
            );
        }
        let joined = out.join("");
        assert!(joined.contains("[`abc1234`](http://x)"));
        for (i, line) in out.iter().enumerate() {
            if line.contains("[`abc1234`](http://x)") {
                let text_only = any_link_re().replace_all(line, "").trim().to_string();
                let text_only = hash_lone_strip_re().replace(text_only.as_str(), "").trim().to_string();
                assert!(!text_only.is_empty(), "hash-only line at L{}: {:?}", i + 1, line);
            }
        }
    }

    #[test]
    fn format_continuation_input_merged() {
        let out = format_entry(
            "- (cli) add the -h flag to all 4 binaries,",
            &["wire up arg parsing in main — [`abc1234`](http://x)"],
        );
        let joined = out.join("");
        assert!(joined.contains("[`abc1234`](http://x)"));
        for line in &out {
            if line.contains("[`abc1234`](http://x)") {
                let text_only = any_link_re().replace_all(line, "").trim().to_string();
                let text_only = hash_lone_strip_re().replace(text_only.as_str(), "").trim().to_string();
                assert!(!text_only.is_empty(), "hash-only line: {:?}", line);
            }
        }
    }

    #[test]
    fn format_no_description_just_hash() {
        let out = format_entry("- (chore) — [`abc1234`](http://x)", &[]);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("[`abc1234`](http://x)"));
    }

    #[test]
    fn format_literal_paren_artifact_stripped() {
        let bullet = "- (core) extract library crates — tools and main binary now \
                       depend on shared crates — ([`aaa1111`](u), [`bbb2222`](u), [`ccc3333`](u))";
        let out = format_entry(bullet, &[]);
        let joined = out.join(" ");
        assert!(!joined.contains("— (, , )"), "artifact remains: {}", joined);
        assert!(!joined.contains("— ( , , )"), "artifact remains: {}", joined);
        for h in ["`aaa1111`", "`bbb2222`", "`ccc3333`"] {
            assert!(joined.contains(h), "missing hash {} in: {}", h, joined);
        }
    }

    // ----- check -----

    #[test]
    fn check_clean_entry_passes() {
        let text = "## [X]\n\n### Features\n- (cli) add -h flag — [`abc1234`](http://x)\n";
        assert!(check(text).is_empty());
    }

    #[test]
    fn check_merge_up_detected() {
        let text = "## [X]\n\n### Features\n\
                    - (cli) add the -h flag to all 4 binaries and the tools folder,\n  \
                    wire up arg parsing — [`abc1234`](http://x)\n";
        let errs = check(text);
        assert!(
            errs.iter().any(|e| e.contains("MERGE-UP")),
            "expected MERGE-UP, got: {:?}",
            errs
        );
    }

    #[test]
    fn check_hash_alone_detected() {
        let text = "## [X]\n\n### Features\n\
                    - (cli) add -h flag\n  \
                    — [`abc1234`](http://x)\n";
        let errs = check(text);
        assert!(
            errs.iter().any(|e| e.contains("HASH-ALONE")),
            "expected HASH-ALONE, got: {:?}",
            errs
        );
    }

    // ----- end-to-end -----

    #[test]
    fn format_clean_input_stays_clean() {
        let text = "\
# Changelog\n\
\n\
## [2.1.1]\n\
\n\
### Features\n\
- (cli) add -h/--help to all 4 binaries — [`abc1234`](http://x)\n\
\n\
### Bug Fixes\n\
- (frontend) fix SSE status stuck on reconnect — add onopen handler — [`def5678`](http://x)\n\
- (frontend) fix export/import/version using relative URLs — use `${API_URL}` prefix so\n\
  they work from external static hosting — [`ccc3333`](http://x)\n\
\n\
### Chores\n\
- (build) consolidate build scripts into scripts/ directory; add exe/ to\n\
  .gitignore — [`bbb2222`](http://x)\n";
        let formatted = format_text(text);
        let errs = check(&formatted);
        assert!(
            errs.is_empty(),
            "formatting produced violations:\n{}\n--- formatted ---\n{}",
            errs.join("\n"),
            formatted
        );
    }

    #[test]
    fn format_messy_input_gets_cleaned() {
        let text = "\
# Changelog\n\
\n\
## [X]\n\
\n\
### Features\n\
- (cli) add the -h flag to all 4 binaries, the\n\
  tools folder, and the db_check helper — [`abc1234`](http://x)\n\
- (frontend) fix export/import/version using relative URLs —\n\
  use `${API_URL}` prefix so they work from external static hosting — [`def5678`](http://x)\n";
        let formatted = format_text(text);
        let errs = check(&formatted);
        assert!(
            errs.is_empty(),
            "formatting produced violations:\n{}\n--- formatted ---\n{}",
            errs.join("\n"),
            formatted
        );
    }

    #[test]
    fn format_real_changelog() {
        // Real CHANGELOG.md should not gain formatting violations.  Keep this
        // test resilient to the current checked-in formatting state.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("CHANGELOG.md");
        let original = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("read {:?} failed", path));
        let original_errs = check(&original);
        let formatted = format_text(&original);
        let formatted_errs = check(&formatted);
        assert!(
            formatted_errs.len() <= original_errs.len(),
            "formatter did not reduce enough:\n  before: {}\n  after:  {}\n  remaining: {}",
            original_errs.len(),
            formatted_errs.len(),
            formatted_errs.join("\n")
        );
    }
}
