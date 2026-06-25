"""Check CHANGELOG.md formatting rules.

Two rules enforced (both use the same display-length measurement, where
`[hash](url)` counts as 0):

  (a) A line containing hash link(s) must also contain descriptive text
      (no hash-only lines).
  (b) A continuation line whose first word would fit on the previous line
      (after the 88-char display-width rule) is illegal — it must be merged
      up.

Public API:
  - `check(text) -> list[str]`: returns a list of human-readable error
    messages (empty list means OK).
  - As a CLI script: reads `CHANGELOG.md` from the working directory and
    prints OK / FAIL with all violations.
"""

import re
import sys

WIDTH = 88

HASH_LINK_RE = re.compile(r"\[`?([0-9a-f]{7,})`?\]\([^)]*\)")
ANY_LINK_RE = re.compile(r"\[([^\]]*)\]\([^)]*\)")
HEADING_RE = re.compile(r"^#{1,6}\s")
TABLE_SEP_RE = re.compile(r"^\|---")


def disp_len(line: str) -> int:
    s = HASH_LINK_RE.sub("", line)
    s = ANY_LINK_RE.sub(r"\1", s)
    return len(s)


def looks_like_bullet(stripped: str) -> bool:
    return stripped.lstrip().startswith(("- ", "* "))


def is_continuation(stripped: str) -> bool:
    return stripped.startswith("  ") and not looks_like_bullet(stripped)


def is_meta(stripped: str) -> bool:
    if not stripped:
        return True
    if HEADING_RE.match(stripped):
        return True
    if stripped.startswith("```"):
        return True
    if TABLE_SEP_RE.match(stripped):
        return True
    if stripped.startswith(">"):
        return True
    if stripped.startswith("|"):
        return True
    return False


def check(text: str) -> list[str]:
    """Return a list of formatting violations. Empty list means OK."""
    lines = text.splitlines()
    errs: list[str] = []
    in_entry = False
    prev_display = ""
    in_code_block = False

    for i, raw in enumerate(lines, 1):
        stripped = raw.rstrip("\n")

        if stripped.startswith("```"):
            in_code_block = not in_code_block
        if in_code_block:
            continue

        no_hash_links = HASH_LINK_RE.sub("", stripped)
        display_text = ANY_LINK_RE.sub(r"\1", no_hash_links)

        # (a) hash link on a line with no descriptive text
        hashes = HASH_LINK_RE.findall(stripped)
        if hashes:
            text_only = ANY_LINK_RE.sub("", stripped).strip()
            text_only = re.sub(r"[—–\-]\s*$", "", text_only).strip()
            if not text_only:
                errs.append(f"  HASH-ALONE L{i}: {stripped!r}")

        # (b) continuation line with mergeable word
        is_bullet = looks_like_bullet(stripped)
        is_cont = is_continuation(stripped)
        is_m = is_meta(stripped)
        if is_cont and not is_m and in_entry:
            cont_words = display_text.strip().split()
            if cont_words:
                first_word = cont_words[0]
                if not re.search(r"[a-zA-Z0-9]", first_word):
                    pass
                else:
                    merged_len = len(prev_display) + 1 + len(first_word)
                    if merged_len <= WIDTH:
                        errs.append(
                            f"  MERGE-UP L{i}: '{first_word}' fits on L{i-1} "
                            f"(merged={merged_len} ≤ {WIDTH}): {stripped!r}"
                        )

        if is_bullet:
            in_entry = True
        elif is_m:
            in_entry = False

        if is_bullet or is_cont:
            prev_display = display_text
        elif not is_m:
            prev_display = ""

    return errs


def main() -> int:
    path = sys.argv[1] if len(sys.argv) > 1 else "CHANGELOG.md"
    with open(path, encoding="utf-8") as f:
        text = f.read()
    errs = check(text)
    if errs:
        print(f"FAIL: {len(errs)} violation(s) in {path}")
        for e in errs:
            print(e)
        return 1
    print(f"OK: {path} passes all checks")
    return 0


if __name__ == "__main__":
    sys.exit(main())
