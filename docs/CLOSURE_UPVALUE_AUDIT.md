# Closure/upvalue completion audit

Audit date: 2026-09-25. Base: `719d6f7`. Branch:
`runtime/full-upvalue-closures`.

Status: **snapshot-compatible VM capture mechanism implemented**. The user selected
the non-breaking contract on 2026-09-25: preserve successful v1 snapshot behavior.
The audit below records the pre-change implementation and baseline evidence.
The [implementation report](CLOSURE_UPVALUE_IMPLEMENTATION.md) records final
evidence, known validation failures and the bounded closure-deferral completion.

## Resolved compatibility decision

The requested shared-binding design conflicts with existing successful program
results in **both** engines. This is not just an incomplete VM implementation.

```kujo
func make_pair() {
    mut count := 0
    func increment() { count += 1; return count }
    func current() { return count }
    return [increment, current]
}
let pair := make_pair()
print(pair[0]())
print(pair[1]())
print(pair[0]())
print(pair[1]())
```

Both engines print `1, 0, 2, 0` (one value per line). Shared lexical cells would
print `1, 1, 2, 2`. A reader created before its parent's assignment also retains
the earlier value. Two aliases of the **same** closure do share its capture state.
Separately created closures snapshot it again, including transitive captures.

The assignment's stop condition forbids silently breaking stable runtime
semantics. `docs/RELEASE_PROCESS.md` requires explicit compatibility/migration
handling; `docs/V1_SCOPE.md` requires version gating for intentional breaking
language/runtime changes. The original audit offered two choices:

1. Preserve v1 per-closure snapshots and complete explicit indexed lexical
   resolution under that contract. This needs an explicit exception to the
   assignment's instruction against copying captures; it cannot be described as
   shared parent/sibling binding identity.
2. Authorize shared lexical binding semantics in both engines, with an explicit
   release/version and migration plan. This expands the interpreter work and
   requires a cycle-lifetime design before promotion to shared cells.

Option 1 was selected by the user. Separate closures retain separate snapshots;
aliases and repeat calls of the same closure share its cells. Compiler binding
resolution and capture access may change without adopting shared sibling cells.
The implementation must preserve this contract, fix lexical scope defects, and
retain the separate interpreter and generator limitations explicitly.

## Snapshot-compatible implementation

The compiler now discovers captures during lexical resolution and records
definition-site local slots, parent capture indices, scoped script bindings and
runtime-created named bindings explicitly. Intermediate functions forward only
the captures needed by descendants. `LoadCapture` / `StoreCapture` use a lazy
frame-local indexed view of the existing callback-shared cells. Each new closure
still snapshots values; no shared parent/sibling binding semantics were adopted.

Nested named functions occupy lexical slots instead of globals. A self binding
is installed only for calls that need it, without placing the function inside
its own captured environment. Function replacement retains earlier function
values. Loop captures respect shadowing and evaluate the iterable before binding
the iteration variable. Script block captures survive environment-scope exit.

`tests/closure_capture_contracts.rs` checks compiler sources, indexed access,
transitive forwarding, non-retention of unrelated locals, capture release,
malformed operands, VM recursion and JIT cache/fallback behavior. The differential
audit suite retains the snapshot contracts and adds runtime-created bindings,
script scopes, loop shadowing and named-function replacement. The two original
VM defect probes now pass. The interpreter's pre-existing returned-recursion
limitation remains explicitly ignored and is not claimed fixed.

The implementation is described in [the architecture](ARCHITECTURE.md). The
following pipeline and validation sections preserve the **pre-change audit**;
final implementation validation is recorded in the linked implementation report.

## Baseline source-to-runtime pipeline (`719d6f7`)

1. `src/parser.rs::parse_func_with_async` and `parse_func_expr_with_async` build
   `Stmt::FuncDef` / `Expr::Function` in `src/ast.rs`; there are no capture lists.
   Parser expression/block limits bound parsed source nesting.
2. `Compiler::add_local` gives explicit local declarations and parameters
   monotonically allocated slots, names and binding kinds. `resolve_local_slot`
   searches active lexical locals in reverse; scope exit truncates that list.
   The root compiler uses environment bindings instead of frame slots.
3. `find_free_variables` computes a sorted used-minus-defined **name** set over
   a body. `_parent_locals` is ignored. Its traversal is not a full lexical
   resolver: block definitions enter a common set, while nested bodies add uses.
   `upvalue_names` is membership in this candidate set, not resolved identities.
4. Named, anonymous and struct-method compilation copy that list to
   `BytecodeChunk.upvalues`. Captured references lower to `LoadVar`/`StoreVar`,
   ordinary explicit locals to slot instructions. Imports use import native
   helpers and runtime binding; global/undefined candidates are not separately
   resolved capture descriptors. Bare assignment can create named locals.
5. `MakeClosure` searches the current frame by name: existing captured map,
   last matching name in the chunk's entire local-slot metadata, then named
   locals. It clones the found **value** into a new `Arc<Mutex<Value>>` and
   copies mutability metadata. An unfound name is omitted for runtime lookup.
   This is not promotion of the defining local into shared storage.
6. `Value::BytecodeFunction` owns a chunk and two maps (capture cells and binding
   kinds). `LoadVar` searches captures, named locals, then globals. `StoreVar`
   enforces captured binding kind and updates the cell. Frame setup clones the
   cell Arcs, so repeated calls and aliases retain the same closure state.
7. Return and exception unwind pop frames and truncate the operand stack.
   Captured values are already heap-owned and have no stack pointer to close.
   Grandchildren copy intermediate captured values into new cells; they do not
   share their intermediate closure's binding cell.
8. `CaptureUpvalue` / `LoadUpvalue` / `StoreUpvalue` / `CloseUpvalues` are a
   separate VM-wide vector mechanism that the compiler does not emit. Its
   captures are immediately closed and closing is a no-op. Its stores lack
   binding-kind enforcement. Connecting it unchanged would be incorrect.

## Interpreter comparison

`Environment` stores values and binding kinds in parallel scope maps.
`Stmt::FuncDef` in nested scopes and anonymous functions clone the environment
into a new `Arc<Mutex<Environment>>`. `enter_captured_environment` clones that
snapshot on invocation; normal completion writes the updated captured
environment back. This retains unrelated environment values, unlike the VM's
selected capture map. Value cloning is type-specific: collections may share
copy-on-write storage and callable values retain shared callable state.

Neither environment retention nor existing `Arc<Mutex<Value>>` proves shared
lexical binding semantics. The latter shares state across invocation/aliasing,
not across independent `MakeClosure` operations.

## Baseline behavioral matrix

`existing` means a named repository test covers the row; it is not a claim that
this audit exhaustively validated every variation. `probe` means the new
characterization suite. The decision entries below preserve the original audit
checkpoint; the user subsequently selected snapshot preservation. Current
implementation coverage and remaining limits are in the implementation report.

| Scenario | Interpreter / current VM evidence | Intended VM |
| --- | --- | --- |
| Read immutable / mutable capture after return | copied value survives; existing nearest-binding and counter tests | preserve values/lifetime |
| Mutate mutable capture repeatedly | own closure state persists; probe alias test | preserve |
| Mutate immutable capture | scalar/array/map writes rejected for captured let and const; dedicated audit test | preserve |
| Parent writes after closure creation | closure retains old value; probe parent test | snapshot vs shared decision |
| Separately created siblings | independent snapshots; probe siblings test | snapshot vs shared decision |
| Aliases of same closure | shared closure state; probe alias test | preserve |
| Independent factories | independent state; probe alias test | preserve |
| Nested/grandparent capture | values propagate through intermediate snapshots; probe transitive test | snapshot vs shared decision |
| Shadowed variable | nearest-binding parity test exists; whole-chunk name search is insufficient to prove all scopes | explicit lexical identity |
| `for` body capture | existing loop snapshot test returns 1,2,3 | preserve per-iteration observations |
| `while` / `loop` captures | dedicated audit test retains per-iteration snapshots 0,1,2 in both engines | snapshot vs shared decision |
| Conditional creation / scope exit | dedicated conditional/early-return test retains capture after block exit | explicit lexical identity/lifetime |
| Recursive local / mutual functions | returned named recursion works on VM but fails in interpreter; ignored regression records missing self-binding; mutual patterns remain unaudited | must specify self/forward references |
| Callback | existing higher-order/callable parity tests | preserve |
| Imported callback crossing runtime | existing `imported_vm_callback` suite | preserve cell identity and policy |
| Captured collection | dedicated array/map reads and repeated writes pass; alias combinations not exhaustive | preserve value semantics |
| Captured struct / enum | dedicated factory-return test reads a struct field and compares captured enum values in both engines | preserve value semantics |
| Captured callable | alias probe and factory-return callable invocation pass; no exhaustive recursive graph proof | preserve callable identity |
| Throw after captured mutation, caught outside | both retain mutation; probe throw test | preserve |
| Escape before parent error / reset | closure published before a throw survives frame unwind in both engines; execution reset not yet probed | preserve validated escape behavior; audit reset |
| Arity | existing closure/function/async/generator parity tests | preserve diagnostics |
| Deep nesting | deterministic generated anonymous closure chains at depths 1–12 retain grandparent parameter values in both engines | preserve bounded generated coverage |

## Representation and lifecycle design constraints

A shared-cell implementation cannot just replace the clone in `MakeClosure`
with `Arc::clone`: defining slots still contain ordinary values, and sibling
closures each allocate their own cell. Compile-time descriptors must distinguish
local slot, enclosing capture and global/runtime binding; lexical shadowing
must take precedence over a name-only free-variable set. Descriptor propagation
must cross intermediate functions and handle local named functions, imports,
destructuring and exception/match bindings.

Keep ordinary slots as values. Promote captured slots only if the chosen
contract requires parent/sibling shared identity; preserve binding kind and
make all slot read/write and in-place fast paths observe the promoted cell.
Frame snapshots, generator restoration, callback children and JIT helpers must
not bypass it. Heap cells can avoid stack relocation entirely, but lifetime
ownership still needs proof: `cell -> callable -> cell` is a possible cycle if
shared capture permits storing a closure into its own binding. This audit found
no tracing cycle collector in the inspected value/VM paths; it does not claim
that every existing graph is leak-free. Weak references alone would break valid
escaping closures. Do not silently introduce a collector or unbounded cycles.

For snapshot-compatible completion, indexed captures can remain heap-owned at
creation; no open stack reference is necessary. That alternative must be named
accurately and approved against the shared-binding target.

## Bridge, JIT, security and adjacent work

- `VM::call_interpreter_callback` creates a child VM with the caller's globals,
  policy and cloned callable/cell identities. `CallbackBridgeGuard` bounds
  synchronous nesting. Tests cover results, mutation, errors, arity, restricted
  filesystem denial and recursion. Preserve these; do not transfer authority in
  a capture descriptor.
- JIT opcode eligibility rejects closure creation and the dedicated upvalue
  opcodes, but accepts name/local loads and stores. Direct-call/cache paths also
  need review before any slot representation changes. No new JIT safety claim.
- Normal VM locals use `Vec<Value>`; current captured accesses add hash lookup,
  mutex lock and value clone. Closure creation copies a chunk and allocates
  cells/maps. Interpreter creation retains a whole environment. No before/after
  performance claim is possible without an implementation.
- Checked local/upvalue access returns errors on invalid indices in inspected
  handlers; `MakeClosure` directly indexes constants. This is not a complete
  malformed-bytecode validator or a claim of sandboxing. No unsafe code added.
- Async named captures already have mutation/isolation tests. Generator state
  stores capture maps and slots; any future representation must update those
  fields together with its owner. Spawn lowering currently creates/discards a
  closure without a capture-analysis pass. Generator/async/spawn redesign is
  outside this checkpoint. Do not infer closure coverage from their broad
  existing surface tests.

## Additional unresolved observations

Nested named functions are stored with `StoreGlobal`. A parent defining
`middle`, followed by top-level `let middle := outer()`, produces a VM duplicate
binding error while the interpreter succeeds:

```kujo
func outer() {
    func middle() { return 1 }
    return middle
}
let middle := outer()
print(middle())
```

The desired-behavior regression
`nested_function_must_not_define_a_global_binding` is explicitly ignored until
repaired; merely changing cell storage will not fix it. It is separate from the capture-identity decision.

The whole-chunk slot search also selects bindings that are not yet in scope:

```kujo
func factory() {
    let value := 1
    let reader := func() { return value }
    if true { let value := 2 }
    return reader
}
let reader := factory()
print(reader())
```

The VM prints `null`; the interpreter prints `1`. The desired-behavior test
`closure_must_capture_the_binding_visible_at_its_definition` is explicitly
ignored until repaired. Neither VM defect probe is counted as passing evidence.
Both are compiler lexical-resolution defects, independent of the identity
policy decision. No defect is claimed fixed by this audit.

A returned named recursive closure also exposes an interpreter limitation:

```kujo
func factory(offset) {
    func fold_capture(n) {
        if n == 0 { return offset }
        return n + fold_capture(n - 1)
    }
    return fold_capture
}
let recursive := factory(7)
print(recursive(3))
```

The VM prints `13` (exit 0); the interpreter reports
`Undefined variable: fold_capture` (exit 4). Its snapshot is created before its
own name is bound. The ignored desired-behavior regression
`a_recursive_named_closure_keeps_its_capture_after_parent_return` records this
unresolved limitation. This is a third defect probe, not a passing parity case.
No interpreter self-binding or reference-cycle policy was changed to mask it.

## Baseline validation (before runtime changes)


No runtime source was modified. Default-feature baseline `cargo check` passed
in 8m 13s; formatting and diff whitespace checks passed. The initial full
`cargo test` was interrupted during dependency compilation to prioritize
bounded closure evidence while other repository builds were active on this
16 GB host. It is **not** a passing full-suite result.

| Command | Actual result |
| --- | --- |
| `cargo fmt --check` | passed |
| `cargo check` | passed, default features |
| `cargo clippy --all-targets --all-features -- -D warnings` | passed: initial baseline 3m 58s, final expanded test source recheck 3.62s |
| `cargo test` | default-feature continuation failed at `http_route_concurrency::process_shutdown_terminates_in_flight_handlers`: one-second shutdown deadline assertion; no runtime source changes |
| `cargo test --test closure_capture_audit --test vm_interpreter_parity_surfaces --test imported_vm_callback` | passed with default features: 12 characterization, 115 parity, 8 callback tests; three ignored defect probes |
| `cargo run -- test --runtime vm` | passed: 154/154 runnable fixtures, six skipped; reports `vm_primary=149` |
| `cargo run -- test --runtime dual` | passed: 154/154 runnable fixtures, six skipped; `vm_primary=154`, `interpreter_fallback=0` |
| `bash scripts/release_gate.sh --full` | not rerun: its full-test step has a known failing baseline, and implementation remains blocked |
| `CARGO_BUILD_JOBS=2 cargo test --no-default-features --test closure_capture_audit --test vm_interpreter_parity_surfaces --test imported_vm_callback` | passed: 5 characterization, 115 parity, 8 callback tests; 2 explicitly ignored defect probes |
| `CARGO_BUILD_JOBS=2 cargo test --no-default-features --test closure_capture_audit -- --ignored` | expected failure confirmed: both defect probes fail on the VM after their interpreter assertions pass |
| `git diff --check` | passed |

Continuation expanded `closure_capture_audit.rs` to 12 passing cases and three
ignored defect probes. Direct `rustc --test` execution against the existing
reduced-feature library passed all 12; explicitly running the ignored tests
confirmed all three failures. This additionally covers scalar/compound values,
captured collection mutation, conditional and early-return paths, while/loop
snapshots, generated anonymous nesting depths 1–12, let/const mutation rejection,
and a closure published before its parent throws. The recursive named-closure
probe fails only on the interpreter. No runtime source changed.

The reduced-feature run excludes JIT and optional database/image/PDF surfaces;
it does not substitute for the full requested release gates. Its ordinary
pass count is 128, with the two known defects separately confirmed, not passed.
An initial test-helper compile error (using Rust equality on `Value`) was
corrected to variant matching before the successful run.

The later default-feature run uses `CARGO_BUILD_JOBS=2`,
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, and
`CARGO_INCREMENTAL=0` to bound build resources on the shared host. Optional
runtime features and debug assertions remain enabled. The explicit targeted
run passes 135 tests; the full suite is not claimed passing.
An isolated rerun of `process_shutdown_terminates_in_flight_handlers` also
failed the same deadline assertion (70.74s total, versus 70.56s for its full
suite). Host-load sensitivity and root cause remain unclassified; this is
outside the closure assignment and was not changed or silently waived.

## Original audit checkpoint handoff (`447f5ad`, superseded)

- Architecture implemented: none; this branch adds audit evidence and tests.
- Files: `tests/closure_capture_audit.rs` characterizes capture identity and
  carries three ignored desired-behavior regressions; this document records the
  pipeline, matrix, constraints and results. `V1_SCOPE.md` retains the deferral
  and links the decision; the parity matrix names the evidence and defects;
  `CHANGELOG.md` records the documentation-only impact.
- Performance: no before/after benchmark, because no runtime implementation was
  changed. Source-level costs and the ordinary-slot constraint are recorded above.
- Security diff review: only literal-program tests and documentation changed;
  no capability handling, runtime ownership, bytecode indices or unsafe blocks
  changed. Shared-cell cycle handling and JIT paths still require review when
  the runtime implementation is selected. This is not a security certification.
- Remaining limitations: capture-identity decision, three confirmed closure
  defects, full upvalue implementation and its complete validation remain open.
- Other roadmap owners: no generator/async/spawn code changed. Any subsequent
  slot/capture model change must coordinate `CallFrameData`, `GeneratorState`
  and async snapshots; spawn's current lowering does not run capture analysis.
- Merge recommendation: **blocked as runtime completion**; ready to review as
  an audit checkpoint. Do not close the upvalue deferral from these results.
