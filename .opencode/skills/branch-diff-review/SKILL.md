---
name: branch-diff-review
description: Reviews the diff between the current branch and a base ref (branch or tag, e.g. main, v1.2.3) for introduced bugs, optimization opportunities, and missed modifications. Triggered by keywords like "branch review", "diff review", "review branch", "compare branch", "tag review", "diff tag", "compare tag".
---

# Branch / Tag Diff Review

When asked to review the current branch's changes against a base ref (a branch **or** a tag), perform the following steps. Throughout this skill the variable `<base-ref>` refers to whichever ref the user (or auto-detection) picked — branches and tags are interchangeable for the git commands used below.

## 1. Determine Base Ref

1. If the user specified a base ref (e.g. `main`, `master`, `develop`, or a tag like `v1.2.3`, `release/2024-09`), use that directly. Detect tag-vs-branch by running `git show-ref --verify refs/tags/<base-ref>` or `git rev-parse --verify <base-ref>` — both refs work identically with the diff commands in step 2, so no further branching is needed.
2. Otherwise, auto-detect. Try each of the following in order and use the first that resolves:
   - Run `git remote show origin` to find the default branch (`HEAD` branch).
   - Try `main`, then `master`.
   - If the user is on a release/hotfix branch and none of the above match, check `git tag --sort=-version:refname | head -5` and pick the most recent semver tag (skip prereleases) as the base ref. Mention this choice explicitly in the report.
3. Run `git merge-base HEAD <base-ref>` to find the merge base. This works for both branches and tags.
4. If `git merge-base HEAD <base-ref>` fails because `<base-ref>` is an **ancestor** of `HEAD` (i.e. `HEAD` is ahead of `<base-ref>` on the same lineage — common when reviewing against an older tag), the merge base is `<base-ref>` itself. Verify with `git merge-base --is-ancestor <base-ref> HEAD` and use `<base-ref>` directly in that case.

## 2. Gather Change Information

1. Run `git log --oneline <base-ref>..HEAD` to see all commits on the current branch since `<base-ref>`.
2. Run `git diff <base-ref>...HEAD` (triple-dot: diff from merge base to HEAD) to get the full diff.
3. List changed files: `git diff --name-only <base-ref>...HEAD`.
4. For new files, read them directly.

> Note: For tag-vs-tag reviews (e.g. comparing `v1.0.0..v1.1.0`), the same `git diff <tag-a>...<tag-b>` syntax applies — adapt `HEAD` to the second tag.

## 3. Bug Check

Review every changed line for:

- **Undefined variables / name errors**: references to variables, functions, or classes that don't exist or aren't imported. Check both new code and any removed references to existing symbols.
- **Type mismatches**: function calls with wrong argument types or mismatched shapes in PyTorch tensor operations.
- **Missing imports**: any symbol used but not imported; check the existing imports in the file.
- **API misuse**: incorrect usage of PyTorch / transformer_engine APIs (e.g., wrong argument names, missing required arguments, deprecated APIs).
- **Device / dtype issues**: tensors on wrong device or with wrong dtype, especially `.to(device)` or `.cuda()` calls that should be NPU-aware (`npu()`).
- **Async / sync bugs**: missing `.wait()`, `.synchronize()`, or incorrect stream handling.
- **Resource leaks**: opened files, streams, or handles not properly closed.
- **Error handling**: bare `except:`, overly broad `except Exception`, missing `finally` for cleanup.
- **Race conditions**: shared state without synchronization in multi-stream or multi-process code.
- **Off-by-one / boundary errors**: loop bounds, slice indices, tensor shape mismatches.
- **Hardcoded NPU-specific values**: paths, device IDs, or constants that should be configurable.
- **Merge conflicts**: leftover conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) in the diff.

## 4. Optimization Check

Review changed code for:

- **Redundant operations**: unnecessary `.clone()`, `.detach()`, or repeated tensor-to-device transfers.
- **In-place opportunity**: operations that could use in-place variants (`.add_()` vs `.add()`) to reduce memory.
- **Recomputation vs memory tradeoff**: operations that are recomputed but could be cached, or vice versa.
- **Unnecessary Python loops**: loops over tensor elements that could be vectorized with PyTorch operations.
- **Gradient computation**: tensors that don't need gradients but have `requires_grad=True`.
- **Sequential vs fused ops**: multiple sequential operations that could be fused (e.g., `scale + add` instead of separate ops).
- **Unused computation**: variables computed but never used.
- **Import optimization**: unused imports or imports that could be deferred to reduce startup time.
- **NPU-specific optimization**: operations that could use NPU-specific kernels or avoid host-device synchronization.
- **Repeated work**: same computation performed across multiple commits or in a loop that could be hoisted.

## 5. Missed Modification Check

- **Incomplete refactoring**: old function/variable names still referenced after renaming. Compare across all changed files.
- **Missing stubs**: new public functions/classes without corresponding `__init__.py` exports.
- **Missing test coverage**: new functionality without corresponding test changes. Check if test files exist for the changed modules.
- **TODO / FIXME / HACK left in code**: markers left behind that should be addressed.
- **Debug code**: left-in `print()`, `breakpoint()`, or debug assertions.
- **Missing type hints**: public APIs without type annotations when the project convention uses them.
- **Forgotten files**: `__pycache__/`, `.pyc`, `.o`, or other build artifacts that should be in `.gitignore`.
- **Inconsistent naming**: naming that doesn't follow project conventions (snake_case for Python, etc.).
- **Missing copyright headers**: new files without the project's standard copyright header.
- **Skeleton / placeholder code**: `pass`, `raise NotImplementedError`, or `...` left in place unintentionally.
- **Cross-file inconsistencies**: changes in one file that should have corresponding changes in related files (e.g., config changes without schema updates, API changes without caller updates).
- **Commit message issues**: commits that don't follow project conventions (check `git log` output from step 2.1).

## 6. Summary Statistics

Before the detailed report, provide a Chinese summary:

```
## Summary
- Base ref: <base> (branch: <name> / tag: <name>)
- Commits: <count>
- Files changed: <count>
- Insertions: <+, -> deletions: <-, ->
- **BUGS**: <count>
- **Optimizations**: <count>
- **Missed Modifications**: <count>
```

## 7. Report

Summarize findings grouped by category. For each issue, include the file path, line number, and a concrete suggestion. Prioritize by severity: **BUG** > **Optimization** > **Missed Modifications**.

**Language**: Output the report in Chinese (中文) by default. The summary statistics, section headings, BUG / Optimization / Missed Modifications entries, file paths, line numbers, and concrete fix suggestions should all be written in Chinese. Code identifiers, file names, line numbers, and English-only technical terms (e.g. `Linear`, `forward_dw`, `quantize_weight`, `fsdp_group`) may be kept as-is in their original form.
