---
name: release
description: Bump version, tag, and update CHANGELOG for keymouse-monitor releases — map Unreleased entries to commits
license: MIT
compatibility: opencode
metadata:
  audience: maintainers
  workflow: github
---

## Files to update

| File | What to change |
|---|---|
| `version` | Replace current version with new one (plain text, e.g. `2.1.0`) |
| `Cargo.toml` | `[package] version = "X.X.X"` |
| `static/icon/app.rc` | `FILEVERSION X,X,0,0` and `PRODUCTVERSION X,X,0,0` (comma-separated) |
| `CHANGELOG.md` | Add new `## [X.X.X]` section with grouped entries, each linked to its commit hash; add new `## [Unreleased]` section above it |
| `Unreleased.md` | Delete this file after processing |
| `Cargo.lock` | Updated automatically by `cargo check` |

## Unreleased → CHANGELOG Mapping

`Unreleased` 是宏观的代码层面描述（无 commit hash），而非 commit message 列表。release 时需要：
1. 理解每条 `Unreleased` entry 描述的**功能/修复/架构变化**。
2. 翻阅 commits 的 diff，找出**实际实现了该逻辑的一个或多个 commit**。
3. 一条 `Unreleased` 可能对应多个 commits（如一个功能分多次提交完成），也可能一个 commit 贡献了多条 entry 的一部分。

## Steps

1. Find the last version tag: `git describe --tags --abbrev=0`.
2. Determine the new version number (semver: bump major/minor/patch as appropriate).
3. Read each `Unreleased` entry. Then inspect all commits since last tag: run `git log --format="%h %s" <last_tag>..HEAD` for overview, and `git diff <last_tag>..HEAD -- <module_path>` for each module area to understand what actually changed in code.
4. For each `Unreleased` entry, identify the commit(s) whose diff implements that logic. The entry is written from code-level understanding, so you need to cross-reference the diff with the entry's description.
5. Build CHANGELOG entries with the actual commit hashes. One `Unreleased` entry may map to multiple commits — list them all under the same section/category.
6. Update `version`, `Cargo.toml`, and `static/icon/app.rc` with the new version.
7. Run `cargo check` to regenerate `Cargo.lock`.
8. In `CHANGELOG.md`, insert a new `## [X.X.X]` section above the
   `[Unreleased]` section with the mapped entries grouped by category (Features
   / Bug Fixes / Refactoring / Performance / Chores). Each entry must include
   the full commit link.
9. Delete the `Unreleased.md` file.
10. Commit all changes with message `:bookmark: bump version to X.X.X`.

## CHANGELOG Entry Format

```
### Category
- [`ab12cd3`](https://github.com/muutot/keymouse_monitor/commit/ab12cd3) (module) description
```

Categories in order: Features, Bug Fixes, Refactoring, Performance, Chores.

## Example

For version `2.1.0`:
- `version` file: `2.1.0`
- `Cargo.toml`: `version = "2.1.0"`
- `static/icon/app.rc`:
  - `FILEVERSION 2,1,0,0`
  - `PRODUCTVERSION 2,1,0,0`
- `CHANGELOG.md`: add `## [2.1.0]` section with entries, delete `Unreleased` file
- Commit: `:bookmark: bump version to 2.1.0`
