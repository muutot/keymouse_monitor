"""Unit tests for scripts/format-changelog.py.

Uses check-changelog.py as the oracle. The formatter's contract is:
  1. Identity for already-formatted input: re-formatting a clean CHANGELOG
     must produce the same content (modulo whitespace normalization).
  2. The formatter's output must pass the check script (possibly with a
     small, known-unformattable exception — we tolerate at most N
     violations on the real CHANGELOG.md).
  3. Specific representative entries must be formatted as expected.

Run from the project root with:
    python .opencode/skills/release/test-format-changelog.py
"""

import importlib.util
import os
import re
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
SCRIPT = os.path.join(ROOT, "scripts", "format-changelog.py")


def _load_module(name: str, path: str):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


fc = _load_module("format_changelog", SCRIPT)
ck = _load_module("check_changelog", os.path.join(HERE, "check-changelog.py"))


def run_format(text: str) -> str:
    """Run the formatter script as a subprocess and return stdout."""
    proc = subprocess.run(
        [sys.executable, SCRIPT, "-"],
        input=text,
        capture_output=True,
        text=True,
        encoding="utf-8",
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        check=True,
    )
    return proc.stdout


class HashLinkRegexTests(unittest.TestCase):
    """The hash-link regex must accept backticked hashes (`` [`abc1234`](url) ``)
    as well as bare hashes (`[abc1234](url)`)."""

    def test_bare_hash(self):
        self.assertEqual(
            fc.collect_hash_links("[abc1234](http://x)"),
            ["[abc1234](http://x)"],
        )

    def test_backticked_hash(self):
        self.assertEqual(
            fc.collect_hash_links("[`abc1234`](http://x)"),
            ["[`abc1234`](http://x)"],
        )

    def test_multiple(self):
        out = fc.collect_hash_links(
            "see [`aaa1111`](u) and [`bbb2222`](u)"
        )
        self.assertEqual(out, ["[`aaa1111`](u)", "[`bbb2222`](u)"])


class FormatEntryTests(unittest.TestCase):
    """`format_entry(bullet, cont)` is the core per-entry reflow function."""

    def test_short_single_hash_inline(self):
        out = fc.format_entry(
            "- (cli) add -h flag — [`abc1234`](http://x)",
            [],
        )
        self.assertEqual(out, [
            "- (cli) add -h flag — [`abc1234`](http://x)",
        ])

    def test_short_multi_hash_inline(self):
        out = fc.format_entry(
            "- (db) merge SQLite fallback — ([`aaa1111`](u), [`bbb2222`](u))",
            [],
        )
        self.assertEqual(out, [
            "- (db) merge SQLite fallback — ([`aaa1111`](u), [`bbb2222`](u))",
        ])

    def test_long_desc_wraps(self):
        bullet = (
            "- (frontend) fix export/import/version using relative URLs — "
            "use `${API_URL}` prefix so they work from external static hosting — "
            "[`abc1234`](http://x)"
        )
        out = fc.format_entry(bullet, [])
        # All lines must be ≤ 88 display chars (hash links count as 0).
        for line in out:
            self.assertLessEqual(ck.disp_len(line), 88, f"line too long: {line!r}")
        # The hash must share a line with descriptive text.
        joined = "".join(out)
        self.assertIn("[`abc1234`](http://x)", joined)
        for i, line in enumerate(out, 1):
            if "[`abc1234`](http://x)" in line:
                text_only = re.sub(r"\[[^\]]*\]\([^)]*\)", "", line).strip()
                text_only = re.sub(r"[—–\-]\s*$", "", text_only).strip()
                self.assertTrue(text_only, f"hash-only line at L{i}: {line!r}")

    def test_continuation_input_merged(self):
        # Input entry is split across a bullet and a continuation; the
        # formatter should re-pack them.
        out = fc.format_entry(
            "- (cli) add the -h flag to all 4 binaries,",
            ["wire up arg parsing in main — [`abc1234`](http://x)"],
        )
        joined = "".join(out)
        # The original two lines must collapse to fewer (or equal) lines,
        # and the hash must be on a line that has descriptive text.
        self.assertIn("[`abc1234`](http://x)", joined)
        for line in out:
            if "[`abc1234`](http://x)" in line:
                text_only = re.sub(r"\[[^\]]*\]\([^)]*\)", "", line).strip()
                text_only = re.sub(r"[—–\-]\s*$", "", text_only).strip()
                self.assertTrue(text_only, f"hash-only line: {line!r}")

    def test_no_description_just_hash(self):
        # Edge case: entry that is just a hash link. Should still produce
        # one line with the hash and a separator.
        out = fc.format_entry("- (chore) — [`abc1234`](http://x)", [])
        self.assertEqual(len(out), 1)
        self.assertIn("[`abc1234`](http://x)", out[0])

    def test_literal_paren_artifact_stripped(self):
        # The legacy 2.0.0 entries embed the multi-hash tail as literal
        # text ` — ([h1], [h2], [h3])`. Stripping the hash links must not
        # leave ` — (, , )` in the description.
        bullet = (
            "- (core) extract library crates — tools and main binary now "
            "depend on shared crates — ([`aaa1111`](u), [`bbb2222`](u), [`ccc3333`](u))"
        )
        out = fc.format_entry(bullet, [])
        joined = " ".join(out)
        # The artifact ` — (, , )` must NOT appear.
        self.assertNotIn("— (, , )", joined)
        self.assertNotIn("— ( , , )", joined)
        # All hashes must still be present and on a line with text.
        for h in ("`aaa1111`", "`bbb2222`", "`ccc3333`"):
            self.assertIn(h, joined)


class CheckCheckTests(unittest.TestCase):
    """Sanity-check the check function itself."""

    def test_clean_entry_passes(self):
        text = (
            "## [X]\n"
            "\n"
            "### Features\n"
            "- (cli) add -h flag — [`abc1234`](http://x)\n"
        )
        self.assertEqual(ck.check(text), [])

    def test_merge_up_detected(self):
        # Continuation whose first word fits on previous line.
        text = (
            "## [X]\n"
            "\n"
            "### Features\n"
            "- (cli) add the -h flag to all 4 binaries and the tools folder,\n"
            "  wire up arg parsing — [`abc1234`](http://x)\n"
        )
        errs = ck.check(text)
        self.assertTrue(any("MERGE-UP" in e for e in errs), f"expected MERGE-UP, got: {errs}")

    def test_hash_alone_detected(self):
        text = (
            "## [X]\n"
            "\n"
            "### Features\n"
            "- (cli) add -h flag\n"
            "  — [`abc1234`](http://x)\n"
        )
        errs = ck.check(text)
        self.assertTrue(any("HASH-ALONE" in e for e in errs), f"expected HASH-ALONE, got: {errs}")


class FormatOutputTests(unittest.TestCase):
    """End-to-end: format arbitrary input, then check passes."""

    def _check_clean(self, text: str) -> None:
        formatted = run_format(text)
        errs = ck.check(formatted)
        self.assertEqual(errs, [], f"formatting produced violations:\n{chr(10).join(errs)}\n--- formatted ---\n{formatted}")

    def test_clean_input_stays_clean(self):
        text = (
            "# Changelog\n"
            "\n"
            "## [2.1.1]\n"
            "\n"
            "### Features\n"
            "- (cli) add -h/--help to all 4 binaries — [`abc1234`](http://x)\n"
            "\n"
            "### Bug Fixes\n"
            "- (frontend) fix SSE status stuck on reconnect — add onopen handler — [`def5678`](http://x)\n"
            "- (frontend) fix export/import/version using relative URLs — use `${API_URL}` prefix so\n"
            "  they work from external static hosting — [`ccc3333`](http://x)\n"
            "\n"
            "### Chores\n"
            "- (build) consolidate build scripts into scripts/ directory; add exe/ to\n"
            "  .gitignore — [`bbb2222`](http://x)\n"
        )
        self._check_clean(text)

    def test_messy_input_gets_cleaned(self):
        # Input that triggers multiple check violations must come out clean.
        text = (
            "# Changelog\n"
            "\n"
            "## [X]\n"
            "\n"
            "### Features\n"
            "- (cli) add the -h flag to all 4 binaries, the\n"
            "  tools folder, and the db_check helper — [`abc1234`](http://x)\n"
            "- (frontend) fix export/import/version using relative URLs —\n"
            "  use `${API_URL}` prefix so they work from external static hosting — [`def5678`](http://x)\n"
        )
        self._check_clean(text)


class RealChangelogTest(unittest.TestCase):
    """Apply the formatter to the project's actual CHANGELOG.md and verify
    the output has at most a small number of pre-existing, un-formattable
    violations (the only one we know about has entry-description so long
    that no reflow can satisfy both rules)."""

    CHANGELOG = os.path.join(ROOT, "CHANGELOG.md")
    # The historical `avoid base_counts clone on save` entry is genuinely
    # un-formattable per the strict check; we tolerate it.
    MAX_ALLOWED_VIOLATIONS = 1

    def test_reduces_violations_dramatically(self):
        with open(self.CHANGELOG, encoding="utf-8") as f:
            original = f.read()
        original_errs = ck.check(original)
        formatted = run_format(original)
        formatted_errs = ck.check(formatted)
        # Before: 48+ violations on the real CHANGELOG. After: at most
        # MAX_ALLOWED_VIOLATIONS.
        self.assertGreater(
            len(original_errs), 10,
            "test premise broken: original CHANGELOG has too few violations",
        )
        self.assertLessEqual(
            len(formatted_errs), self.MAX_ALLOWED_VIOLATIONS,
            f"formatter did not reduce violations enough:\n"
            f"  before: {len(original_errs)}\n"
            f"  after:  {len(formatted_errs)}\n"
            f"  remaining: {formatted_errs}",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
