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
9. **Write/append a macro-level entry under `## [Unreleased]` in
   `CHANGELOG.md`** from a code-level perspective. Insert the new entry right
   after the `## [Unreleased]` heading, on a new line (bullet point), before
   any existing entries in that section. Format: markdown bullet list, wrap at
   88 chars, keep it concise.
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
- **88-char wrap** — each line at most 88 display characters;
  `[`hash`](url)` counts as 0 (neither URL nor hash text displayed)
- **Concise** — say what was done and why in as few words as possible
- **No commit hashes** — those are added by release skill
- New entries are inserted at the top of the list (right after `## [Unreleased]`)

## Examples

### Before commit

Staged changes include a database module rewrite:

```markdown
## [Unreleased]

- :sparkles: [database]: add MongoDB fallback to SQLite — when primary write
  fails, automatically retry on local SQLite; on reconnect, sync data back
```

### After several commits

```markdown
## [Unreleased]

- :sparkles: [database]: add MongoDB fallback to SQLite with auto-reconnect
- :bug: [rawinput]: hardcode X1/X2 button number instead of usButtonData
- :recycle: [imports]: group imports and remove fully-qualified std paths
```
