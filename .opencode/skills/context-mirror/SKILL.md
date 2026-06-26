---
name: context-mirror
description: >
  脉络镜: 深入分析某个模块在项目中的位置、调用链、参数详情、
  类继承关系、前后端依赖，并与 NVTE (NVIDIA Transformer Engine)
  最新实现进行对标，识别实现缺漏和优化机会。触发词包括
  '脉络镜', 'context-mirror', '分析模块', '模块脉络', '对标分析',
  'gap analysis', 'NVTE', 'trace module', 'module analysis'.
---

# 脉络镜 — Module Context Insight

When asked to analyze a module's context in the project, perform the following steps.

## 1. Locate Module Source

1. Take the user-specified module/class name (e.g. `GroupedLinear`, `LayerNormLinear`, `fp8_autocast`).
2. Search the source tree for the definition:
   - `grep -r "class <Name>" --include="*.py" transformer_engine/`
   - `grep -r "def <name>" --include="*.py" transformer_engine/`
3. Read the primary definition file (the one with the class/function body).

## 2. Trace Import Chain

Map how the module reaches the public API:

1. Find all `__init__.py` files that export the symbol:
   - Search for the name in all `__init__.py` files under `transformer_engine/`.
2. Trace step by step from the definition file up to top-level `__init__.py`.
3. List the full import path chain, e.g.:
   ```
   transformer_engine/pytorch/module/linear.py  (definition)
   └→ transformer_engine/pytorch/module/__init__.py  (re-export)
     └→ transformer_engine/pytorch/__init__.py  (public API)
   ```

## 3. Analyze Class Hierarchy

1. Read the `class` definition line to get all base classes.
2. For each base class, find its definition file and list key responsibilities.
3. Present the inheritance chain:
   ```
   GroupedLinear
   └→ TransformerEngineBaseModule  (base.py: config mgmt, FP8 metadata)
     └→ nn.Module  (PyTorch base)
   ```

## 4. Detail Constructor Parameters

1. Read the `__init__` method and extract every parameter with its type annotation and default value.
2. For each parameter, add a concise explanation in Chinese of its purpose (derived from docstrings or code context).
3. Identify which parameters affect forward behavior, memory usage, parallel strategy, etc.
4. Present in a table format:

   | Parameter | Type | Default | Description |
   |-----------|------|---------|-------------|
   | `num_gemms` | `int` | — | 专家/分组数量 |
   | `in_features` | `int` | — | 输入特征维度 |
   | ... | ... | ... | ... |

## 5. Map Key Methods and Call Flow

1. List all public methods on the class (from reading the file).
2. For the `forward` method:
   - Read the full signature.
   - Summarize the forward logic flow (preprocessing → autograd function → postprocessing).
   - Identify the `torch.autograd.Function` subclass that implements the actual forward/backward.
   - Trace what operations are performed (gemm, norm, cast, etc.).
3. For other key methods (`reset_parameters`, `backward_dw`, `make_grouped_weights`, `set_tensor_parallel_group`, etc.):
   - List them with a one-line summary of what they do.

## 6. Identify Related Modules and Dependencies

1. Search for imports of this module across the codebase:
   ```
   grep -r "from.*import.*<Name>" --include="*.py" transformer_engine/
   grep -r "import.*<Name>" --include="*.py" transformer_engine/
   ```
2. Also search in `tests/` to find test files.
3. Identify:
   - **Callers**: which modules use this class.
   - **Dependencies**: what ops/layers this module imports and uses.
   - **Tests**: corresponding test files and key test functions.
   - **Parallel implementations**: alternative code paths (e.g. `performance_grouped_linear_impl.py`).

## 7. Identify Distant Relatives (跨文件/模块关联)

1. Search for the module's key method names being called from other files:
   ```
   grep -r "\.forward_dw\|\.backward_dw\|\.need_backward_dw\|\.make_grouped_weights" --include="*.py" transformer_engine/
   ```
2. Search for configuration/constant references related to this module.
3. Look for any `isinstance` checks or type dispatch that references this class.

## 8. NVTE Reference & Gap Analysis (与 NVTE 对标分析)

This step compares the NPU implementation against NVIDIA's official Transformer Engine (NVTE) to identify missing features, implementation gaps, and optimization opportunities.

### 8.1 Locate NVTE Reference Path

1. Read the module file's docstring / header comments for a `Reference:` line, e.g.:
   ```
   Reference: TransformerEngine/transformer_engine/pytorch/module/grouped_linear.py
   ```
2. If no reference is found, infer the equivalent path:
   - The NPU repo mirrors NVTE's directory structure under `transformer_engine/`.
   - Derive the NVTE path using the same relative path under `NVIDIA/TransformerEngine` on GitHub.

### 8.2 Fetch NVTE Source

1. Fetch the corresponding NVTE source file from NVIDIA's GitHub:
   ```
   https://raw.githubusercontent.com/NVIDIA/TransformerEngine/main/<reference-path>
   ```
   - Try `main` branch first; if that fails (404), try `stable` or the latest release tag.
   - Use `webfetch` to retrieve the raw content.
2. If the exact file doesn't exist at the expected path, search the NVTE repo structure:
   ```
   https://api.github.com/repos/NVIDIA/TransformerEngine/contents/<directory>
   ```
   to find the closest equivalent.
3. If the remote fetch fails (no network, rate limit), note this limitation and skip to section 8.4 (manual comparison based on codebase knowledge).

### 8.3 Structural Comparison

Compare the NPU and NVTE versions across these dimensions:

1. **Class hierarchy**: Same base classes? Different MRO?
2. **Constructor parameters**: Compare every `__init__` param side by side:
   - Parameters present in NVTE but missing in NPU → **feature gap**
   - Parameters with different defaults → **behavior divergence**
   - Parameters with different types → **porting issue**
3. **Public methods**: List NVTE methods not present in NPU → **missing implementation**
4. **Forward signature**: Same input/output contract? Different return types?
5. **FP8 / quantization support**: NVTE may have newer quantization modes (e.g. block-wise FP8, MXFP4, MXFP8) not yet ported.
6. **Autograd function**: Compare the inner `torch.autograd.Function` subclass — the forward/backward logic differences reveal the core porting delta.

### 8.4 Categorize Deltas

Classify each difference into:

| Category | Label | Description |
|----------|-------|-------------|
| **Missing Feature** | 🔴 GAP | NVTE has a feature/param/method that NPU lacks entirely |
| **Behavior Divergence** | 🟡 DIVERGE | Same interface but different logic or defaults |
| **NPU-Specific Adaptation** | 🔵 ADAPT | Intentional change for NPU hardware (e.g. MindSpeed ops vs cuBLAS) |
| **Optimization Opportunity** | 🟢 OPT | NVTE uses a more efficient approach that could be adopted |
| **Deprecated / Removed** | ⚪ CLEANUP | NVTE removed something NPU still carries |

### 8.5 Optimization Suggestions

Based on the comparison, recommend specific actions:

1. **Port missing features**: List each 🔴 GAP with the effort estimate and priority.
2. **Align divergent behavior**: For each 🟡 DIVERGE, suggest whether to align with NVTE or keep the NPU adaptation.
3. **Adopt newer NVTE patterns**: For each 🟢 OPT, describe the approach (fused kernels, delayed scaling, tensor parallelism improvements, etc.).
4. **Remove dead code**: For each ⚪ CLEANUP, point to NVTE commits that removed the feature.

## 9. Output Report

Present findings in the following structure in Chinese (code identifiers and technical terms remain in English):

```
## 脉络镜: <ModuleName>

### 1. 源文件定位
<file_path>

### 2. 导入链
<full import chain>

### 3. 类继承关系
<inheritance tree>

### 4. 构造参数详情
| 参数 | 类型 | 默认值 | 说明 |
| ... | ... | ... | ... |

### 5. 核心方法 & 调用流程
#### forward
<signature and flow description>

#### 其他关键方法
- `<method>`: <description>

### 6. 关联模块 & 依赖
- **调用方**: <files that import this module>
- **内部依赖**: <what ops/layers this module uses>
- **测试文件**: <corresponding test files>
- **备选实现**: <alternative code paths if any>

### 7. 跨文件关联
<references to this module's methods from other parts of the codebase>

### 8. NVTE 对标分析
#### 8.1 NVTE 参考源
<NVTE source URL or path>

#### 8.2 结构对比

| 对比维度 | NPU | NVTE | 差异 |
|----------|-----|------|------|
| 基类 | ... | ... | ... |
| 构造参数数 | ... | ... | ... |
| 公开方法数 | ... | ... | ... |
| FP8 支持 | ... | ... | ... |
| ... | ... | ... | ... |

#### 8.3 参数差异明细
| 参数 | NPU 默认值 | NVTE 默认值 | 差异类型 | 说明 |
|------|-----------|-------------|----------|------|
| ... | ... | ... | 🔴 GAP / 🟡 DIVERGE / ... | ... |

#### 8.4 方法差异明细
| 方法 | NPU | NVTE | 差异类型 | 说明 |
|------|-----|------|----------|------|
| ... | 有 | 有 | — | 实现不同 |
| ... | 无 | 有 | 🔴 GAP | 缺失 |
| ... | 有 | 无 | ⚪ CLEANUP | 可清理 |

#### 8.5 优化建议
- [🔴] <priority>: <suggestion>
- [🟢] <priority>: <suggestion>

### 9. 总结
<brief summary of the module's role in the project and the key gaps vs NVTE>
```
