---
name: commit
description: Generate a Git emoji commit message from unstaged/staged changes, then log the summary to the Unreleased file
license: MIT
compatibility: opencode
metadata:
  audience: developers
  workflow: local
---

## Workflow

1. Run `git status --short` and `git diff` / `git diff --cached` to inspect all pending changes.
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
5. If the change can be fully described in one line, **omit the bullet list**.
6. If multiple distinct changes exist, add a blank line after the subject, then list each change with `-` bullet points.
7. All message content must be in **English**.
8. After the commit is made, read the actual diff (`git diff HEAD~1..HEAD` or the staged changes before commit). Analyze what the code functionally does — not the commit message, but what capabilities were added, what bugs were fixed, how the architecture changed. Then **write or append a macro-level entry to the `Unreleased` file** summarizing the change from a code-architecture perspective.
9. If `Unreleased` file does not exist, create it. Keep entries deduplicated and grouped by logical area.

## Message vs Unreleased — 区别

| | Git commit message | Unreleased entry |
|---|---|---|
| 依据 | diff 文件列表 → 简要描述改动 | diff 代码逻辑 → 理解功能/修复/架构变化 |
| 粒度 | 原子 commit 级别 | 宏观模块/功能级别 |
| 内容 | `:emoji: [模块] 改了什么` | 这段代码**实现了什么能力**、**修复了什么场景的 bug**、**如何重构了架构** |
| commit hash | 有 | 无（release 时映射） |

## Unreleased 文件格式

Plain text, repository root. 每条从代码层面描述一个可发布的能力变更：

```
:sparkles: [database]: add MongoDB fallback to SQLite — when primary write fails, automatically retry on local SQLite; on reconnect, sync missing data back to MongoDB
:bug: [rawinput]: fix X1/X2 button not registering on certain keyboard firmware where usButtonData is always 0 — fallback to hardcoded button number instead
```

每条应当是一段完整的自然语言，说清楚**做了什么、为什么、什么场景下生效**，而非「修复 bug」这种简短描述。发布时 release skill 据此匹配到实际 commits。

## Examples

| Change | Message |
|---|---|
| New feature in database module | `:sparkles: [database]: add MongoDB fallback to SQLite with auto-reconnect sync` |
| Bug fix in rawinput | `:bug: [rawinput]: hardcode X1/X2 button number instead of usButtonData` |
| Refactor imports | `:recycle: [imports]: group imports and remove fully-qualified std paths` |
| Update hooks | `:wrench: [hooks]: drop amend in post-commit, stage only` |
| Version bump | `:bookmark: bump version to 2.1.1` |
| Changelog update | `:memo: [changelog]: add commit links and remove date from version title` |
