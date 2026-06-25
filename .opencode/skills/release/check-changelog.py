import re, sys

path = "CHANGELOG.md"
lines = open(path, encoding="utf-8").readlines()

WIDTH = 88
errs = []

in_entry = False
in_code_block = False
prev_display = ""

for i, raw in enumerate(lines, 1):
    stripped = raw.rstrip("\n")

    # Track code fence state
    if stripped.startswith("```"):
        in_code_block = not in_code_block

    if in_code_block:
        continue

    # Compute display text: [hash](url) → "" (counts as 0),
    # other [label](url) → "label" (label is real content)
    no_hash_links = re.sub(
        r"\[([0-9a-f]{7,})\]\([^)]*\)", "", stripped
    )
    display_text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", no_hash_links)

    # (a) hash link(s) on a line with no meaningful description
    hashes = re.findall(r"\[([0-9a-f]{7,})\]\([^)]*\)", stripped)
    if hashes:
        text_only = re.sub(r"\[([^\]]*)\]\([^)]*\)", "", stripped).strip()
        # Delete em/en-dash and bullet markers
        text_only = re.sub(r"[—–\-]\s*$", "", text_only).strip()
        if not text_only:
            errs.append(f"  HASH-ALONE L{i}: {stripped!r}")

    # (b) continuation line with mergeable word
    is_bullet = stripped.lstrip().startswith(("- ", "* "))
    is_continuation = stripped.startswith("  ") and not is_bullet
    is_meta = not stripped or stripped.startswith(("#", "```", "|---", ">", "|"))
    if is_continuation and not is_meta and in_entry:
        cont_words = display_text.strip().split()
        if cont_words:
            first_word = cont_words[0]
            # Skip purely punctuation/symbol "words" (e.g. em-dash, +, >)
            if not re.search(r"[a-zA-Z0-9]", first_word):
                pass
            else:
                merged_len = len(prev_display) + 1 + len(first_word)
                if merged_len <= WIDTH:
                    errs.append(
                        f"  MERGE-UP L{i}: '{first_word}' fits on L{i-1} "
                        f"(merged={merged_len} ≤ {WIDTH}): {stripped!r}"
                    )

    # Track entry state for continuation detection
    if is_bullet:
        in_entry = True
    elif is_meta:
        in_entry = False

    if is_bullet or is_continuation:
        prev_display = display_text
    elif not is_meta:
        prev_display = ""

if errs:
    print(f"FAIL: {len(errs)} violation(s) in {path}")
    for e in errs:
        print(e)
    sys.exit(1)
else:
    print(f"OK: {path} passes all checks")
