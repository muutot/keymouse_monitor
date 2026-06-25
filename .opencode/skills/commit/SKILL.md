---
name: commit
description: Generate a Git emoji commit message from unstaged/staged changes, write macro-level summary under [Unreleased] in CHANGELOG.md, then commit both together
license: MIT
compatibility: opencode
metadata:
  audience: developers
  workflow: local
---

## Workflow

1. Run `git status --short` and `git diff` / `git diff --cached` to inspect all
   pending changes.
2. Categorize the changes by type and pick the corresponding emoji:
   - `:sparkles:` — new feature
   - `:bug:` — bug fix
   - `:recycle:` — refactoring (no behavior change)
   - `:zap:` — performance improvement
   - `:white_check_mark:` — adding/updating tests
   - `:memo:` — documentation / changelog
   - `:art:` — code style / formatting / lint
   - `:wrench:` — tooling, config, build, dependencies, CI, hooks
   - `:bookmark:` — version bump / release
3. Identify the module scope from the changed files and wrap it in `[brackets]`.
4. Write the subject line: `:emoji: [module] description`
5. If the change can be fully described in one line, omit the bullet list.
6. If multiple distinct changes exist, add a blank line after the subject, then
   list each change with `-` bullet points.
7. All message content must be in **English**.
8. Read the actual diff of the staged changes. Analyze what the code
   functionally does — not what files changed, but what capabilities were added,
   what bugs were fixed, how the architecture changed.
9. **Rewrite `## [Unreleased]` from scratch** by analyzing all changes
   since the last release tag:
   - Find last tag: `git describe --tags --abbrev=0`
   - Review all commits since then: `git log --format="%h %s" <last_tag>..HEAD`
   - Read diffs to understand what each commit functionally does
   - **Replace the entire `[Unreleased]` section** with a coherent macro-level
     summary of everything that will ship in the next release. Treat all
     commits since the last tag as one conceptual change set. Cycle-internal
     bugfixes (fixing something introduced in the same cycle) should be
     **omitted entirely** from `[Unreleased]` — they are internal details not
     relevant to users. Only bugfixes targeting **already-released code**
     (not modified in this cycle) get their own `:bug:` entry.
   - Format: markdown bullet list, wrap at 88 chars, concise.
   - Newer/more significant entries at the top.
10. **Stage `CHANGELOG.md` together with the code changes**, so the summary is
    committed as part of this commit.
11. Proceed with committing.

## Message vs Unreleased — 区别

| | Git commit message | CHANGELOG `[Unreleased]` entry |
|---|---|---|
| 依据 | diff 文件列表 → 简要描述改动 | diff 代码逻辑 → 理解功能/修复/架构变化 |
| 粒度 | 原子 commit 级别 | 宏观模块/功能级别 |
| 内容 | `:emoji: [模块] 改了什么` | 这段代码**实现了什么能力**、**修复了什么场景的 bug** |
| commit hash | 有 | 无（release 时映射） |

## CHANGELOG `[Unreleased]` 格式

Section inside `CHANGELOG.md`. Example:

```markdown
## [Unreleased]

- :sparkles: [database]: add MongoDB fallback to SQLite — when primary write
  fails, automatically retry on local SQLite; on reconnect, sync missing data
  back to MongoDB
- :bug: [rawinput]: fix X1/X2 button not registering on certain keyboard
  firmware where usButtonData is always 0
```

- **Markdown** format, bullet list
- Each bullet describes one capability/bugfix/refactor
- **88-char fill** — each line should try to fill 88 display characters;
  `[`hash`](url)` counts as 0 (neither URL nor hash text displayed)
- **Concise** — say what was done and why in as few words as possible
- **No commit hashes** — those are added by release skill

### Hash placement (for release reference)

When the release skill adds commit hashes:

- **Hash placement**: the `— [`hash`](url)` sequence must never be split across
  lines, and must never appear on a line by itself. The hash always shares a
  line with descriptive text.
- Single hash: inline on the first line if it fits within 88 display chars;
  otherwise on the last continuation line alongside remaining description text.
- Multiple hashes: grouped inside parentheses on the last continuation line,
  preceded by description text:
  `  description — ([`hash1`](url), [`hash2`](url), ...)`.

## Examples

### Initial commit

Staged changes include a database module rewrite:

```markdown
## [Unreleased]

- :sparkles: [database]: add MongoDB fallback to SQLite — when primary write
  fails, automatically retry on local SQLite; on reconnect, sync data back
```

### Next commit

Staged: fix X1/X2 button in rawinput.  Commit skill reviews all changes
since last tag (`v1.0.0`), rewrites the entire `[Unreleased]`:

```markdown
## [Unreleased]

- :sparkles: [database]: add MongoDB fallback to SQLite with auto-reconnect
- :bug: [rawinput]: hardcode X1/X2 button number instead of usButtonData
```
