---
name: commit-message-gen
description: >
  Generates a structured Git commit message from uncommitted
  (staged and unstaged) changes. Analyzes diffs and produces a message
  in English following the format 'type(module): concise summary' with
  optional bullet points. Triggered by keywords like 'commit message',
  'generate commit', 'write commit message'.
---

# Commit Message Generation

When asked to generate a commit message from uncommitted changes, perform the following steps.

## 1. Gather Change Information

1. Run `git status --short` to see all changed files (staged and unstaged).
2. Run `git diff --cached` to review staged changes.
3. If there are unstaged changes, also run `git diff` to review them.
4. For new untracked files, read those files directly.

## 2. Determine Commit Type

Pick the most appropriate type based on the diff content:

| Type       | When to use |
|------------|-------------|
| `feat`     | New feature / new file with new functionality |
| `fix`      | Bug fix |
| `refactor` | Code restructuring without behavior change |
| `perf`     | Performance improvement |
| `test`     | Adding or modifying tests |
| `docs`     | Documentation changes |
| `chore`    | Build / CI / tooling / config changes |
| `style`    | Formatting, linting, whitespace |
| `revert`   | Reverting a previous change |

## 3. Determine Module Name

Identify the primary module/area affected:

- Use the top-level directory or subsystem name (e.g. `grouped_linear`, `layernorm`, `fused_attn`, `distributed`, `fp8`, `tests`).
- If changes span multiple modules, pick the one with most changes, or use a broader scope like `pytorch`.

## 4. Generate Message

Format:
```
<type>(<module>): <concise summary in English>

- <bullet point 1>
- <bullet point 2>
```

Rules:
- **One-liner preferred**: if the changes can be summarized in a single sentence, omit bullet points entirely.
- **English only**: all content in English.
- **Concise total**: aim for under 72 characters in the first line.
- **Bullet points**: use only when multiple distinct changes exist that cannot be summarized in one sentence.
- **Imperative mood**: use "fix", "add", "remove", "update", "refactor" etc. (not "fixed", "added", "fixing").
- **No period** at the end of the first line.
- **Blank line** between the title line and bullet points (if present).
- **No Chinese** in the message body.
- **No issue/PR references** unless the user explicitly asks.

### Examples

Good (one-liner):
```
fix(grouped_linear): mark packed weight/bias with skip_backward_post_hook for delayed wgrad
```

Good (with bullet points):
```
feat(grouped_linear): add FP8 support for packed grouped weights

- Add FP8 quantization path for single_grouped_weight mode
- Register fp8_meta for packed weight tensors
- Update backward_dw to handle quantized packed weights
```

Bad (too long first line, Chinese, past tense):
```
fix(grouped_linear): 修复了延迟 wgrad 计算中 packed weight 没有被标记 skip_backward_post_hook 的问题
```

## 5. Output

1. Present the generated commit message to the user.
2. Ask the user if they want to:
   - Use the message and commit directly (`git commit -m "..."`)
   - Edit the message first
   - Stage unstaged changes first (if any) and then commit
   - Cancel

## 6. Commit (if requested)

If the user approves, stage any unstaged changes as needed and commit using:
```
git commit -m "<message>"
```
