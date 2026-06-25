---
name: release
description: Bump version, tag, and update CHANGELOG for keymouse-monitor releases — map [Unreleased] entries to commits
license: MIT
compatibility: opencode
metadata:
  audience: maintainers
  workflow: local
---

## Files to update

| File | What to change |
|---|---|
| `version` | Replace current version with new one (plain text, e.g. `2.1.0`) |
| `Cargo.toml` | `[package] version = "X.X.X"` |
| `static/icon/app.rc` | `FILEVERSION X,X,0,0` and `PRODUCTVERSION X,X,0,0` (comma-separated) |
| `CHANGELOG.md` | Replace `## [Unreleased]` with `## [X.X.X]` (keep the entries, regroup by category) |
| `README.md` | Review if new features/breaking changes require doc updates |
| `Cargo.lock` | Updated automatically by `cargo check` |

## [Unreleased] → CHANGELOG Mapping

`[Unreleased]` 是宏观的代码层面描述（无 commit hash），而非 commit message 列表。release 时需要：
1. 理解每条 `[Unreleased]` entry 描述的**功能/修复/架构变化**。
2. 翻阅 commits 的 diff，找出**实际实现了该逻辑的一个或多个 commit**。
3. 一条 `[Unreleased]` 可能对应多个 commits（如一个功能分多次提交完成），也可能一个 commit 贡献了多条 entry 的一部分。

## Steps

1. Find the last version tag: `git describe --tags --abbrev=0`.
2. Determine the new version number (semver: bump major/minor/patch as appropriate).
3. In `CHANGELOG.md`, read the `## [Unreleased]` section (bullet list between
   it and the next `##` heading). Then inspect all commits since last tag: run
   `git log --format="%h %s" <last_tag>..HEAD` for overview, and `git diff
   <last_tag>..HEAD -- <module_path>` for each module area to understand what
   actually changed in code.
4. For each `[Unreleased]` entry, identify the commit(s) whose diff implements
   that logic. The entry is written from code-level understanding, so you need
   to cross-reference the diff with the entry's description.
5. Build CHANGELOG entries: **description first**, then commit hashes appended.
   One `[Unreleased]` entry may map to multiple commits — list all hashes.
   A single commit (especially a squash merge) may contribute to multiple
   entries — it is fine to reference the same commit hash in different
   categories (e.g. Features + Bug Fixes) if its diff spans both.
6. Check `README.md` — if the new release adds features, changes APIs, or
   alters config/CLI behavior, update the relevant sections.
 7. Update `version`, `Cargo.toml`, and `static/icon/app.rc` with the new
   version.
8. Run `cargo check` to regenerate `Cargo.lock`.
9. In `CHANGELOG.md`, **replace the `## [Unreleased]` heading with `##
   [X.X.X]`**, keeping all entries. Group them by category (Features / Bug
   Fixes / Refactoring / Performance / Chores), still description-first with
   commit hashes appended.
10. **Verify** CHANGELOG formatting. Run `python .opencode/skills/release/check-changelog.py`
     to check for two violations: (a) hash link without description on same
     line, (b) continuation line with mergeable word (fits on previous line
     within 88 chars). Fix any violations before committing.
11. Commit all changes with message `:bookmark: bump version to X.X.X`.

## CHANGELOG Entry Format

CHANGELOG entries follow a **description-first** format: the macro-level
summary from `[Unreleased]` section comes first, with commit hashes appended:

```markdown
### Category
- (module) description — [`ab12cd3`](url), [`ef4567`](url)
```

- **88-char fill** — each line should reach ≤ 88 display characters.
  `[`hash`](url)` counts as 0 (neither URL nor hash text displayed).
  For multi-line entries, if a continuation line's first word fits on the
  previous line without exceeding 88 chars, it must be merged up. The check
   in step 10 flags violations.

**Hash placement**: the `— [`hash`](url)` sequence must never be split across
lines, and must never appear on a line by itself. The hash always shares a
line with descriptive text.

- Single hash: inline on the first line of the entry if it fits within 88
  display chars; otherwise on the last continuation line alongside remaining
  description text.
  ```
  ✓ - (db) add MongoDB fallback — [`a1b2c3d`](url)
  ✓ - (frontend) fix SSE reconnect status — add `onopen` handler to reset status on
    connection established — [`e4f5g6h`](url)
  ✗ - (frontend) fix SSE reconnect status — add `onopen` handler to reset status on
    connection established
    — [`e4f5g6h`](url)
  ```
  The last line `  — [`e4f5g6h`](url)` has no descriptive text — the hash
  **must** be on the same line as `onopen` handler to reset...`.

- Multiple hashes: group inside parentheses on the last continuation line,
  preceded by description text
  `  description — ([`hash1`](url), [`hash2`](url), ...)`.
  **All hashes and parentheses must be on the same line** — never split
  across lines. Wrong:
  ```
    description — ([`hash1`](url),
    [`hash2`](url))
  ```

Categories in order: Features, Bug Fixes, Refactoring, Performance, Chores.

## Example

For version `2.1.0`:
- `version` file: `2.1.0`
- `Cargo.toml`: `version = "2.1.0"`
- `static/icon/app.rc`:
  - `FILEVERSION 2,1,0,0`
  - `PRODUCTVERSION 2,1,0,0`
- `CHANGELOG.md`: replace `## [Unreleased]` with `## [2.1.0]`, regroup entries
- Commit: `:bookmark: bump version to 2.1.0`
