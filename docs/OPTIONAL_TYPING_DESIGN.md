# Optional Typing Policy (v1.0.0)

This document defines Kujo's explicit v1 optional-typing policy.

## v1 Policy Summary

Status: stable v1.0.0 design contract

For v1.0.0, Kujo typing is intentionally split into two categories:

- Supported in v1:
    - type-annotation syntax for function parameters, function returns, and variable/const declarations
    - parser/AST preservation of annotations as metadata
    - incremental adoption in mixed typed/untyped codebases without breaking existing dynamic execution
- Deferred after v1:
    - runtime type enforcement
    - mandatory static type checking gates in `kujo run`
    - typed-JIT specialization guarantees as part of the stable v1 contract

Compatibility contract for v1:

- annotated code must remain runnable under current dynamic semantics
- type annotations must not change runtime behavior by default
- missing annotations keep current dynamic behavior
- current execution-path boundary is explicit:
    - `kujo run --interpreter` emits non-fatal type-checking warnings when mismatches are detected
    - default VM execution (`kujo run`) does not run a static type-check gate before execution

## Goals

- Define a concrete annotation syntax surface for functions, variables, and collections.
- Define explicit v1 support/defer boundaries for runtime checking and optimization.
- Preserve backward compatibility with existing untyped Kujo code.

## Design Status

- **Scope**: v1 policy and compatibility contract
- **Implementation commitment**:
    - annotation syntax and metadata behavior are supported for v1
    - enforcement and optimization tracks remain deferred

## Stage 1: Annotation Surface Proposal

### Supported Type Syntax (proposed)

```text
func calculate(x: int, y: float) -> float {
    return x * y
}

let name: string := "Alice"
let enabled: bool := true
let scores: Array<int> := [95, 87, 92]
let user_scores: Dict<string, int> := {"alice": 95}
```

Current parse-clean equivalent under today's dynamic runtime:

```kujo
func calculate(x, y) {
    return x * y
}

name := "Alice"
enabled := true
scores := [95, 87, 92]
user_scores := {"alice": 95}
```

### Type Forms (proposed)

- Primitive: `int`, `float`, `string`, `bool`, `null`
- Collections: `Array<T>`, `Dict<K, V>`, `Set<T>`
- Function return type: `func f(...) -> T`

### Annotation Semantics (proposed)

- Annotations are optional and additive.
- Untyped code remains valid and unchanged.
- Mixed typed/untyped code is supported.
- Missing annotations imply current dynamic behavior.

### Parser and Type-Checker Impact

- Parser: add annotation nodes on variable declarations and function signatures.
- AST: preserve annotations as metadata without changing runtime semantics by default.
- Type checker: run as optional pass that can emit diagnostics without blocking execution unless explicitly configured.

## Runtime Type Enforcement (Deferred)

Potential opt-in forms remain design candidates and are not part of the v1 stable contract:

```text
@type_check
func calculate(x: int, y: float) -> float {
    return x * y
}
```

Future global opt-in could be provided via CLI and/or config, for example:

- `kujo run script.kujo --type-check`
- project-level `kujo.toml` setting (deferred decision)

### Deferred Behavior Targets

- Default mode: no runtime type enforcement (current v1 behavior).
- Future opt-in mode: enforce argument and return type contracts on annotated functions.
- Future errors should be deterministic and include function name, parameter/return position, expected type, and actual runtime type.

### Future Error Shape Target

Human-facing format (subject to final wording):

- `TypeError: calculate arg#2 expected float, got string`
- `TypeError: calculate return expected float, got int`

Compatibility note:

- Existing error model remains authoritative; this format is a target for a post-v1 implementation.

## Current Type-Checker Inference Boundaries (v1)

- Method-call inference is intentionally partial:
    - known core method surfaces on `string`, `array`, and `dict` return concrete inferred types in the checker.
    - unknown/custom method surfaces intentionally fall back to `Any` instead of claiming unsupported precision.
- Selective imports now resolve exported function signatures from Kujo module files when the module can be discovered on the configured search paths:
    - `kujo run --interpreter` forwards the entry script's search roots into the checker so imported callables do not show up as false `Undefined function` warnings.
    - When a module cannot be analyzed safely, the checker still falls back to permissive imported-callable placeholders instead of rejecting dynamic code.
    - Dotted module names resolve the same way runtime module loading does (`from src.foo import bar` and `from foo.bar import baz` both use the search-path-based module lookup).
- Await/generator/struct-field inference remains conservative in v1 and may return `Any` or unknown where full static shape information is not yet modeled.
- These boundaries are deliberate for additive, non-breaking typing behavior and are tracked in generated TODO triage artifacts under `docs/generated/V1_CODE_TODO_TRIAGE.*`.

### Post-v1 Type-Checker Follow-Up Cluster (V1RR-P1-002)

The following checker precision items are explicit post-v1 work and are not
release blockers for `v1.0.0`:

- Destructuring inference: destructuring patterns remain conservative in v1;
  unsupported static shapes may resolve to `Any` instead of blocking execution.
- Module existence checks: unresolved modules remain diagnostics/lookup
  concerns, not a mandatory static gate before dynamic runtime loading.
- Struct field type lookup: field reads remain conservative when full struct
  shape information is unavailable.
- Promise unwrap typing: `Promise<T>`/await result inference may return `Any`
  until async value shapes are modeled end to end.
- Permissive callable fallback: imported or dynamic callables that cannot be
  analyzed safely continue to use permissive placeholders in v1 so optional
  typing does not reject compatible dynamic code.

Future closure of this cluster must update checker tests, docs, and generated
TODO triage evidence together.

## Typed-JIT Optimization Boundaries (Deferred)

### Future Candidates

- Specialized numeric fast paths for consistently typed arithmetic.
- Reduced boxing/unboxing in tight loops over typed arrays.
- Improved call-site specialization for stable typed signatures.

### Out of Scope for v1.0.0

- Mandatory typing across all code.
- Full Hindley-Milner inference.
- Breaking syntax or runtime behavior changes for untyped code.
- Guarantees of fixed speedup multiples.

## Migration and Compatibility

- Existing untyped scripts continue to run unchanged.
- Teams can adopt annotations incrementally per module/function.
- Typed and untyped modules interoperate through existing runtime value model.
- Any future runtime checks must remain opt-in and backward compatible by default.

## Open Decisions (Post-v1)

- Final syntax for union/optional types (e.g., `A | B` vs helper forms).
- How generic parameter constraints are represented (if at all).
- CLI-only vs config-based type-check mode enabling.
- Whether warnings-only mode should be separate from strict fail mode.

## Non-Goals for v1

- Shipping a full compiler-grade static type system.
- Requiring developers to annotate all public functions.
- Changing default dynamic semantics in v1.0.

## v1 Decision

Kujo v1 adopts **optional, incremental typing metadata** with strict backward compatibility:

- keep dynamic execution as default
- support annotation syntax and metadata preservation
- defer runtime checks to a future opt-in track
- defer typed-JIT specialization guarantees to a future release

This policy keeps v1 semantics stable while leaving room for future typed enforcement and optimization work without breaking existing code.
