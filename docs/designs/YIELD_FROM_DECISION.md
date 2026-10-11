# Item 5: synchronous generator delegation decision

Status: **design complete; defer `yield from` implementation for 1.9**.
Repository: `kujolang/kujo`, branch `main`, inspected commit
`ecece02d36fafc0d44c63724e2f546cdbb69bd71` (1.8.1 plus unreleased items 2–4).
This is an engineering decision under the roadmap's explicit deferral option,
not a claim of human approval or an implemented language feature.

## Decision and evidence

Keep the supported spelling:

```kujo
func* forward(source) {
    for item in source {
        yield item
    }
}
```

A bounded shorthand is feasible, but the existing spelling already supports lazy
nested forwarding, null values, shared alias progress, error propagation and
capability intersection. New tests verify those properties in both runtimes.
There is no measured throughput, allocation or token benefit justifying a new
execution mechanism. Source brevity alone is not a performance claim.

The acceptance review found an existing boundary disagreement worth resolving
before adding a delegation contract: VM resume depth counts generators separately,
whereas the interpreter charges generator resumes to ordinary function depth.
Both permit 32 generator frames when consumed at top level; a surrounding ordinary
function changes interpreter behavior. The committed
[`yield-from-depth-probe.kujo`](yield-from-depth-probe.kujo) consumes a 32-generator
chain inside `consume` under `try/except`:

| Runtime | Exit | Result |
| --- | --- | --- |
| VM | 0 | `7`, then `false` (no caught error) |
| Interpreter | 4 | `Maximum call stack depth of 32 exceeded while calling <generator>` escapes the enclosing catch |

This is a **failed candidate parity criterion**, not behavior blessed by a golden
assertion. The ordinary-call limit is an existing protection, not something to
remove or raise to admit syntax. No runtime change is made in this design pass.
The exact source, invocation and observations are preserved in
[`../audits/evidence/yield-from-design.json`](../audits/evidence/yield-from-design.json).
The open boundary is tracked in local SignalBox capture
`cap_d8e153bd-7472-4eb5-861a-725fc44ade7f` and signal
`sig_79eb8a70-4ce0-4e2e-b2f7-6dd8e4842fe4`; exact and concept retrieval passed.
Reproduction is offline, uses no host effects and is bounded by the test driver's
10-second process timeout. Depth 4 passes in both; top-level depth 40 fails
catchably in both, including repeated resume of the terminal error.

## Candidate contract for a future implementation

The following is a specification proposal, **not current language syntax**.

| Surface | Bounded proposal |
| --- | --- |
| Grammar | `yield from expression` as a standalone statement inside a synchronous generator or synchronous generator method; reject in ordinary/async functions and value-expression positions. `from` is already a keyword. Preserve ordinary `yield expression`, imports and diagnostics. |
| Source | Accept exactly the inputs supported by `for item in expression`, using its existing normalization and validation. Evaluate once when execution reaches the statement, never at generator creation. Do not add a separate iterable protocol. |
| Values | Forward each yielded item unchanged and in order; `null` is a value, not completion. Empty sources advance immediately. Never collect a generator before yielding. Non-generator inputs retain existing `for` normalization costs. |
| Suspension | One outer resume produces at most one outward item. Preserve the active child, source position and evaluated operands across suspension. Code after delegation waits for child exhaustion. |
| Completion | Child return is completion only; evaluate its return expression for current effects/errors, discard its value. Continue the parent. No completion-value expression, `send`, injected `throw`, or new `{done, value}` result format. |
| Errors | Invalid source and uncaught child errors propagate at the resume that encounters them. A parent handler may recover and yield. An unhandled error terminates and is cached by the parent; the child's own terminal error remains cached. Preserve caller state and original diagnostics, without swallowing or converting errors to exhaustion. |
| Aliases | Child and parent aliases share their respective progress. Another child alias may consume the next item between parent resumes. Reentrant/cyclic resume raises the existing error; do not duplicate or rewind continuations. |
| Captures | Preserve per-instance snapshots and generator-method receiver snapshots. Delegation does not create shared sibling capture cells or look up resumer locals dynamically. |
| Lifetime | Parent retains the child only as part of its active continuation. Completion, terminal failure and releasing all parent aliases release that ownership. An independently retained child survives. No ownership-cycle collector is implied. |
| Early break / abandonment | Stop requesting values; do not drain or close the child. Dropping one parent alias cannot cancel other aliases. No implicit finalizer or guarantee that arbitrary ownership cycles are collected. |
| Cancellation | No new generator cancellation/close API. Running inside a cancellable task remains cooperatively cancellable at existing execution checkpoints; native blocking effects cannot always be interrupted or undone. Cancellation requests do not grant child-close semantics. |
| Capabilities | Each resume intersects creator and current resumer policies; nested delegation consequently cannot exceed child, parent or outer resumer authority. Runtime-owner checks remain in force. |
| Bounds | Retain explicit recursion/resource limits. Resolve and test the ordinary-call/generator-depth interaction above before promising a common boundary. No unbounded stack or automatic limit increase. |

Async generators remain a separate design: scheduler ownership, backpressure,
await/close ordering and cancellation require an independent contract. Shared
capture mutation, general cycle collection and iterator protocol expansion are
also excluded.

## Implementation assessment and review

Inspected implementation, not just documentation:

- `src/parser.rs::parse_primary` parses `Expr::Yield` with an optional expression;
  it has no delegation production. `src/lexer.rs` already reserves `from`.
- `src/compiler.rs` lowers `Stmt::For` using existing iterable normalization and
  lazy generator stepping. A future statement may lower to a hygienic for/yield
  form; synthetic bindings must be unlexable and excluded from user captures.
  The change must cover AST visitors, captures, diagnostics, LSP/docgen and both
  runtimes, not only parser acceptance.
- `src/interpreter/generator_lowering.rs` already preserves expression operands
  with unlexable synthetic bindings. A new statement needs explicit traversal;
  treating it as an ordinary yield expression would be incorrect.
- `src/interpreter/generator.rs::Frame::For` owns the child while suspended;
  `resume_generator` caches failure, restores the caller and intersects policy.
  `Stmt::Return` evaluates its expression and completes without an outward value.
- `src/vm.rs::generator_next` saves/restores execution state, owns suspended
  continuation data, uses weak global ownership, caches failure and bounds nested
  resumes at 32. `Interpreter::with_function_context` instead shares its limit
  of 32 with ordinary calls. The probe demonstrates why those implementations
  cannot be assumed equivalent merely because their constants agree.
- Neither generator step representation exposes a completion value: it returns
  `Some(item)`, `None`, or error. Adding return-value delegation would therefore
  change the protocol and its callers, not merely shorten syntax.

This session performed a source and executable acceptance review, with no
independent or human review claimed. Alternatives considered:

| Alternative | Assessment |
| --- | --- |
| Hygienic shorthand over existing for/yield | Smallest plausible future implementation; defer until the reproduced limit/error boundary is resolved and a concrete workload justifies syntax. |
| Dedicated delegation opcode/state stack now | No measured benefit; adds a second stepping/lifetime path and a larger parity burden. Not justified. |
| Completion-value/send/close protocol | Exceeds the bounded item, affects result shapes and alias/cancellation semantics. Requires a separate proposal. |
| Raise interpreter limits or hide the depth error | Conceals the boundary and changes protection rather than defining semantics. Rejected. |
| Preserve explicit forwarding and record deferral | Meets the current need with tested behavior and no grammar/API change. Selected. |

## Acceptance coverage and reopening criteria

New contracts in `tests/generator_continuation_contracts.rs` cover nested
forwarding, single evaluation, lazy creation, nulls, discarded completion values,
interleaved aliases, terminal errors/recovery, abandoned parent with surviving
child, factory snapshots and all three capability boundaries. Existing tests
cover direct reentry, ownership/root release, receiver snapshots and async
rejection. New bounded subprocess tests in
`tests/concurrency_lifecycle_contracts.rs` cover shallow/deep forwarding and task
cancellation while delegating. These characterize supported explicit forwarding;
they do not claim the proposed syntax exists or freeze the failing probe as correct.

Reopen only with a concrete forwarding workload, a deliberate depth/error policy
and passing cross-runtime boundary tests. Any implementation must additionally
test grammar rejection/ambiguity, hidden-name collisions, import compatibility,
child/parent release via weak ownership probes, source-location diagnostics,
reentry, deeply nested mixed calls, and async rejection. Run the full release gate
and hosted platform checks before shipping. These are implementation prerequisites,
not newly authorized tasks or a 1.9 release blocker.

## Verification and compatibility

Baseline: 22 generator and 17 concurrency contracts passed before edits.
Final verification passed: 27 generator, 19 concurrency, 6 docs-example,
1 README and 123 parity tests (176 total, zero failures). Targeted strict Clippy,
`cargo fmt --check` and `git diff --check` passed. Commands/results are recorded
in the evidence receipt. The full release/hosted matrix was not rerun because
no runtime implementation changed. The existing
supported generator behavior remains unchanged: this pass adds tests and docs.
No runtime source, AST, bytecode, public API, CLI exit code, serialized format,
configuration, environment variable or dependency changes. No new release and no
cross-repository change are required. Roadmap item 5 is complete by explicit
deferral; item 6 still requires a real human first-hour walkthrough and clean-install
evidence. An agent rehearsal cannot satisfy that requirement.
