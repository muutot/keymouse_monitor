#!/usr/bin/env python3
"""Reformat CHANGELOG.md entries to 88-char fill.

Rules (mirrored in check-changelog.py):
  - `[hash](url)` counts as 0 display chars.
  - Each bullet entry gets reflowed greedily onto as few lines as possible.
  - The hash link must share a line with descriptive text. For a single hash,
    it sits on the same line as the last chunk of description text; for
    multiple hashes, they are grouped as `([h1], [h2], ...)` on the same line.
  - A continuation line whose first word fits on the previous line (within
    88 display chars) is illegal; the formatter avoids producing that by
    packing lines greedily.

Reads CHANGELOG.md, prints the reformatted version to stdout.
"""

import re
import sys

WIDTH = 88
HASH_LINK_RE = re.compile(r"\[`?([0-9a-f]{7,})`?\]\([^)]*\)")
HEADING_RE = re.compile(r"^#{1,6}\s")
TABLE_SEP_RE = re.compile(r"^\|---")
EM_DASH = " — "


def disp_len(line: str) -> int:
    """Length of `line` for the 88-char rule, with hash links counted as 0."""
    s = HASH_LINK_RE.sub("", line)
    s = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", s)
    return len(s)


def collect_hash_links(body: str) -> list[str]:
    return [m.group(0) for m in HASH_LINK_RE.finditer(body)]


def strip_hash_links(body: str) -> str:
    s = HASH_LINK_RE.sub("", body)
    # When the original entry has the form `desc — ([h1], [h2], [h3])` with
    # the parens being literal text (as the legacy 2.0.0/1.3.0 entries do),
    # stripping the hash links leaves `desc — (, , )`. Drop that literal
    # pattern from the end of the desc as well.
    s = re.sub(r"\s*—\s*\(\s*(?:,\s*)*\)\s*$", "", s)
    s = re.sub(r"\s*—\s*$", "", s)
    return s.strip()


def split_prefix(bullet: str) -> tuple[str, str]:
    m = re.match(r"^(- \([^)]*\)\s+)", bullet)
    if m:
        return m.group(1), bullet[m.end():]
    return "- ", bullet[2:]


def build_tail(hashes: list[str]) -> str:
    """Build the literal hash tail (em-dash + hash link(s)). Hash links are
    appended to a string whose display width counts only the literal chars."""
    if not hashes:
        return ""
    if len(hashes) == 1:
        return EM_DASH + hashes[0]
    return f"{EM_DASH}({', '.join(hashes)})"


def tail_disp(tail: str) -> int:
    """Display width of the tail, counting hash links as 0."""
    if not tail:
        return 0
    s = HASH_LINK_RE.sub("", tail)
    return len(s)


def format_entry(bullet: str, cont_texts: list[str]) -> list[str]:
    prefix, rest = split_prefix(bullet)
    body = " ".join([rest] + cont_texts).strip()
    hashes = collect_hash_links(body)
    desc = strip_hash_links(body)

    tail = build_tail(hashes)
    t_w = tail_disp(tail)
    prefix_w = disp_len(prefix)
    cont_indent = 2  # for continuation lines

    words = desc.split()
    if not words:
        # No description text — put the tail on the prefix line.
        if tail:
            line = prefix.rstrip()
            if line:
                return [line + tail]
            return [tail.lstrip()]
        return [prefix.rstrip()]

    # Greedy word-fill. The last word of the entry is reserved to share a
    # line with the tail — so when we consider placing word `w`, we treat
    # `tail_w` as already consuming the line budget iff `w` is the very
    # last word AND no more words follow.
    n = len(words)
    lines: list[str] = []
    cur: list[str] = []
    cur_w = 0
    for idx, w in enumerate(words):
        is_last = (idx == n - 1)
        if not cur:
            # Start a new line. We don't reserve tail width here; the
            # post-loop pass handles tail fitting.
            cur = [w]
            cur_w = len(w)
            continue
        margin = prefix_w if len(lines) == 0 else cont_indent
        # When placing the final word, the tail will be appended later;
        # account for it now to avoid an overflow that requires moving
        # the word down.
        reserved = t_w if is_last else 0
        if cur_w + 1 + len(w) + reserved <= WIDTH - margin:
            cur.append(w)
            cur_w += 1 + len(w)
        else:
            lines.append(" ".join(cur))
            cur = [w]
            cur_w = len(w)
    if cur:
        lines.append(" ".join(cur))

    # Append the tail to the last line.
    if tail and lines:
        lines[-1] = lines[-1] + tail

    # Emit with prefix on the first line and a 2-space indent on continuations.
    out: list[str] = []
    for i, desc_line in enumerate(lines):
        if i == 0:
            out.append(prefix + desc_line)
        else:
            out.append("  " + desc_line)
    return out


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


def collect_entries(lines: list[str]) -> list[tuple[int, int, list[str]]]:
    entries: list[tuple[int, int, list[str]]] = []
    cur_start: int | None = None
    cur_end: int | None = None
    cur_texts: list[str] = []
    for i, raw in enumerate(lines):
        stripped = raw.rstrip("\n")
        if looks_like_bullet(stripped):
            if cur_start is not None:
                entries.append((cur_start, cur_end, cur_texts))
            cur_start = i
            cur_end = i
            cur_texts = [stripped]
        elif is_continuation(stripped) and cur_start is not None:
            cur_end = i
            cur_texts.append(stripped)
        else:
            if cur_start is not None:
                entries.append((cur_start, cur_end, cur_texts))
                cur_start = None
                cur_texts = []
    if cur_start is not None:
        entries.append((cur_start, cur_end, cur_texts))
    return entries


def main() -> int:
    if len(sys.argv) > 1 and sys.argv[1] != "-":
        path = sys.argv[1]
        with open(path, encoding="utf-8") as f:
            text = f.read()
    else:
        text = sys.stdin.read()
    lines = text.splitlines(keepends=False)
    entries = collect_entries(lines)
    new_lines = list(lines)
    for start, end, texts in sorted(entries, key=lambda e: e[0], reverse=True):
        bullet = texts[0]
        cont = [t[2:] if t.startswith("  ") else t for t in texts[1:]]
        reformatted = format_entry(bullet, cont)
        replacement = [ln + "\n" for ln in reformatted]
        new_lines[start:end + 1] = replacement
    sys.stdout.write("".join(new_lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
