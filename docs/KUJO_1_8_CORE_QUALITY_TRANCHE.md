# Kujo 1.8 core-quality tranche

Status: implementation complete; canonical Kujo release gate passed

Starting source: `531cc223859977f1c203961e166782ea58108786` (`main`)

## Current-source audit and implementation plan

The audit covered the parser and AST, optional checker, module loader, struct and
callable resolution, compiler and optimizer, VM and interpreter closure,
generator and task paths, runtime measurement collector, diagnostics, LSP
surfaces, benchmark suites, ignored fixtures, current roadmap and compatibility
records. The current implementation supersedes the historical Phase-A and
Phase-B plans: owned generator continuations, snapshot captures, eager bounded
tasks, reusable promises and optimizer relocation are completed contracts, not
reopened architecture work.

The bounded plan for this tranche is:

1. Extend only the optional checker model with collection-pattern, struct,
   promise, callable and analyzed-module shapes. Preserve dynamic fallback and
   keep checker findings advisory in interpreter mode.
2. Cache parsed module ASTs because the richer export analysis otherwise repeats
   lexing and parsing for functions, values and structs.
3. Add focused positive, negative and dynamic-fallback checker regressions plus
   VM/interpreter runtime parity seeds.
4. Extend the existing release measurement campaign with representative language,
   retained-closure, nested-generator, reusable-promise, channel-scheduler and
   offline AI-native workloads. Retain only measured performance changes.
5. Re-run the optimizer relocation suite and canonical release gates. Do not
   enable deferred optimizer passes on exception, short-circuit or method-call
   lowering without separate stack-shape proof.

## Implemented inference scope

- Array destructuring now uses literal positions where available, including
  nested patterns and rest values; homogeneous array types remain the fallback.
- Dictionary destructuring propagates known literal values and dictionary value
  types, including rest dictionaries. Known impossible collection shapes produce
  one actionable warning and then widen bound names to `Any` to avoid cascades.
- Known structs retain declared field shapes through construction, ordinary
  bindings, unannotated function returns and nested field reads. Field annotation
  mismatches and missing known fields remain real checker findings. Known struct
  operator overloads are recognized before primitive-operator compatibility is
  considered, avoiding false diagnostics for valid overloads.
- Async functions and async closures produce internal `Promise<T>` shapes. Await
  unwraps known promises, reusable promises and promises passed through arrays;
  awaiting an already-complete value keeps the documented runtime behavior.
- Function and closure aliases retain callable shapes. Calls through field access
  and namespace members use known signatures; unknown or dynamically reassigned
  callables continue to fall back without a false negative.
- Analyzed modules expose exported functions, literal/annotated values and struct
  definitions. Whole-module namespace access and function-local imports use the
  same bounded cache. Missing modules produce an advisory diagnostic when the
  checker has an entry-script search root, while unresolved imported values remain
  dynamic for runtime resolution.

Kujo has no tuple type, destructured function-parameter syntax, import-alias
syntax or optional-import syntax in the stable grammar. This tranche does not add
new syntax to simulate those features.

## Compiler and VM audit

The current optimizer already centralizes relocation for `Jump`, conditional
jumps, `JumpBack`, `ForNext`, `BeginTry`, exception-handler ranges and source-map
entries. The permanent loop/early-return suite exercises before/after execution
and every address-bearing opcode. Compiler lowering already emits specialized
local index and in-place arithmetic/string/map operations. The remaining disabled
optimizer surfaces are disabled for stack-shape reasons; this tranche does not
broaden them without evidence.

## Diagnostics and editor boundary

The CLI checker now supplies bounded help for known module, destructuring, struct
field and callable mistakes and widens recovery values after the primary finding.
The current LSP hover/completion implementation is token- and syntax-index based;
it does not consume `TypeChecker` state. Existing literal hover, imported-symbol
indexing and diagnostic tests therefore remain the supported editor surface. A
new parallel semantic model was not introduced merely to mirror advisory CLI
inference; connecting a shared analyzed-program model remains separate LSP work.

## Measurements

The exact-source, optimized-build campaign is recorded in the
[measurement receipt](../benchmarks/results/kujo-1.8-core-quality-2026-10-02/README.md),
with raw samples and runtime reports retained alongside it. It used two warmups
and eleven interleaved measured samples per runtime mode, plus two warmups and
twenty-one samples per static-analysis binary.

| Area | Baseline median ms | Candidate median ms | Same-round median change |
| --- | ---: | ---: | ---: |
| Generator/capture sustained | 2236.252 | 2244.145 | +0.4% |
| Retained closures | 121.599 | 120.428 | +0.3% |
| Bounded tasks | 1573.989 | 1552.204 | -0.2% |
| Scheduler/channels | 1613.035 | 1591.823 | -0.3% |
| Project static check | 33.981 | 34.094 | +0.3% (independent medians) |

No runtime optimization was retained because the measured long-running deltas
were within 0.6%. The retained analyzed-module AST cache reduces the enriched
function/value/struct export analysis from three parse passes to one (-66.7% by
exact operation count) without a material project-check latency regression.
Runtime shallow counters characterize creation, suspension/resume, termination,
retained captures, task admission/completion/rejection, scheduling, observers and
cancellation. Allocator totals and retained graph bytes are unsupported by the
host instrumentation and are not estimated.

## Validation

- Type-checker unit suite: 38 passed, including positive, negative and dynamic
  fallback cases.
- VM/interpreter parity suite: 119 passed.
- Dual fixture suite: 150/150 runnable fixtures passed; 11 documented skips.
- Explicit interpreter fixture suite: 150/150 runnable fixtures passed; 11
  documented skips.
- Optimizer relocation and loop/early-return regressions: 6 passed.
- Canonical `scripts/release_gate.sh --full`: passed with socket tests, benchmark
  smoke and advisory audit enabled; `cargo-deny` was unavailable and remained the
  gate's documented optional skip.
- Dispatch downstream release gate: passed against the candidate release binary
  with offline fixtures and the repository-pinned Workcell/Eval dependencies;
  the downstream working tree remained clean.

## Compatibility

- Syntax changes: none.
- Runtime behavior changes: none intended; checker results remain advisory.
- Inferred-type changes: more precise destructuring, struct, async and callable
  shapes where source identity is known; unknown values still widen to `Any`.
- Diagnostic changes: new missing-module, impossible-destructuring, missing-field,
  struct-field mismatch, wrong-arity and non-callable messages.
- VM/interpreter behavior: no semantic contract changes; parity is verified by
  runtime regressions and the canonical suites.
- Newly rejected programs: calls that omitted required unannotated user-function
  parameters now receive the same arity diagnostic as annotated parameters. Such
  calls already fail at runtime; this closes a checker false negative. Optional
  builtin trailing parameters remain optional.

## Deferred candidate review

| Candidate | User value | Compatibility / runtime cost | 1.8 recommendation |
| --- | --- | --- | --- |
| Async generators | Natural streamed async producers | New suspended async lifetime and cancellation contract in both engines | Defer pending a design; high value, high semantic risk |
| `yield from` | Less boilerplate for generator composition | New delegation/error/return rules and continuation nesting | Consider only with the async-generator design, not alone |
| Struct generator methods | Completes an existing explicit unsupported corner | Must reconcile receiver capture with generator ownership in both engines | Best bounded follow-up candidate after this tranche |
| Intentional ownership cycles | Enables cyclic object graphs | Requires a collection/lifetime policy and changes current drop reasoning | Defer; risk exceeds current everyday value |
| Atomic shared-state operations | Safer explicit concurrent counters/state | New memory-ordering and cross-runtime behavioral contract | Design for a later 1.8 tranche only if concrete workloads justify it |

## Candidate 1.8 release highlights

### Language improvements

- More useful gradual inference across destructuring, structs, async values and
  callable aliases without making annotations mandatory.

### Runtime performance

- A reproducible core benchmark corpus characterizes continuations, retained
  captures, bounded tasks, scheduling, collections and offline AI-native work.

### Compiler and VM

- Control-flow relocation regressions remain permanent release gates; no unsafe
  speculative optimizer expansion was accepted.

### Developer experience

- Imported values and namespace callables carry useful shapes, and known module,
  field, destructuring and callable mistakes produce concise guidance with
  bounded cascade recovery.
