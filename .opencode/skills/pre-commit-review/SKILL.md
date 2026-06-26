---
name: pre-commit-review
description: Use when the user wants to commit changes or asks to review uncommitted changes before committing. Checks staged and unstaged diffs for bugs, optimization opportunities, and missed modifications. Also triggered by keywords like "pre-commit", "review commit", "check before commit".
---

# Pre-Commit Review

When asked to review code before committing, perform the following steps:

## 1. Gather Change Information

1. Run `git status --short` to see all changed files (staged and unstaged).
2. Run `git diff --cached` to review staged changes.
3. If there are unstaged changes, also run `git diff` to review them.
4. For new untracked files, read those files directly.

## 2. Bug Check

Review every changed line for:

- **Undefined variables / name errors**: references to variables, functions, or classes that don't exist or aren't imported.
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

## 3. Optimization Check

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

## 4. Missed Modification Check

- **Incomplete refactoring**: old function/variable names still referenced after renaming.
- **Missing stubs**: new public functions/classes without corresponding `__init__.py` exports.
- **Missing test coverage**: new functionality without corresponding test changes.
- **TODO / FIXME / HACK left in code**: markers left behind that should be addressed.
- **Debug code**: left-in `print()`, `breakpoint()`, or debug assertions.
- **Missing type hints**: public APIs without type annotations when the project convention uses them.
- **Forgotten files**: `__pycache__/`, `.pyc`, `.o`, or other build artifacts that should be in `.gitignore`.
- **Inconsistent naming**: naming that doesn't follow project conventions (snake_case for Python, etc.).
- **Missing copyright headers**: new files without the project's standard copyright header.
- **Skeleton / placeholder code**: `pass`, `raise NotImplementedError`, or `...` left in place unintentionally.

## 5. Re-evaluate Each Finding

Before writing the final report, critically re-examine every finding you listed:

1. **Trace concrete code paths** — mentally execute the code flow (including concurrent branches) and verify the issue actually manifests vs being a theoretical concern.
2. **Check timing assumptions** — for any claimed race, confirm whether the participating code paths can actually overlap in practice (e.g. JS single-threaded EventSource → fetch order, or unique session IDs preventing reuse).
3. **Verify language / library semantics** — confirm your understanding of concurrency primitives (`parking_lot::RwLock` vs `tokio::sync::RwLock`, `watch::Sender` clone semantics, `Sender::send` vs channel drop, etc.).
4. **Consult project conventions** — check whether the code follows existing patterns (e.g. CHANGELOG managed separately via `commit` skill, absence of a `tests/` directory, etc.).
5. **Distinguish real from pseudo** — for each item, decide: is it a genuine bug/optimization/missed mod, or is it a false positive / negligible nit? Discard all pseudo findings and only keep real ones in the final report.

## 6. Report

Summarize findings grouped by category. For each issue, include the file path, line number, and a concrete suggestion. Prioritize by severity: **BUG** > **Optimization** > **Missed Modifications**.

**Language**: Output the report in Chinese (中文) by default. The summary statistics, section headings, BUG / Optimization / Missed Modifications entries, file paths, line numbers, and concrete fix suggestions should all be written in Chinese. Code identifiers, file names, line numbers, and English-only technical terms (e.g. `Linear`, `forward_dw`, `quantize_weight`, `fsdp_group`) may be kept as-is in their original form.
