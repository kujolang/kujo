# Bug report: Kujo VM aborts with a native stack overflow on nested data

## TL;DR
`kujo run` (bytecode VM) **hard-aborts the process** with
`fatal runtime error: stack overflow, aborting` when it performs a recursive
operation over a **moderately nested `Value`** (e.g. a dict nested ~300 levels
in a debug build, ~1500 in release). This is a *native* Rust stack overflow on
the tokio worker thread — **distinct from, and bypassing, the existing graceful
256-deep call guard**. It should degrade to a catchable `KUJOVM001` runtime
error, not `SIGABRT`. In practice it also bites real programs at much shallower
data depth when nested values are passed/cloned through deep call chains.

## Environment
- Repo: this repo. Binaries `target/debug/kujo` and `target/release/kujo`.
- Entry point is `#[tokio::main]` (`src/main.rs:1104`) → execution runs on a
  **tokio worker thread (Rust default ~2 MB stack)**; the abort message names
  `tokio-runtime-worker`.
- `kujo run` defaults to the **bytecode VM** (`src/vm.rs`), not the tree-walking
  interpreter (`src/main.rs:150` — "Use tree-walking interpreter instead of
  bytecode VM (default: VM)").

## Two different limits (important distinction)
1. **Graceful guard (works as intended):** `DEFAULT_MAX_VM_CALL_DEPTH = 256` and
   `DEFAULT_MAX_EXPRESSION_DEPTH = 256` in `src/runtime_limits.rs`; enforced in
   `src/interpreter/mod.rs` (~lines 136/153/439, "Maximum call stack depth of {}
   exceeded"). Deep *simple* recursion hits this and returns a clean `KUJOVM001`.
2. **Native abort (the bug):** recursive operations over nested `Value`
   structures overflow the real thread stack and `abort()` with no `KUJOVM001`,
   no stack trace, and no catchability via `try/except`.

## Reproductions (all pure Kujo, no external deps)

**A — the graceful guard (control, behaves correctly):**
```
func rec(n) { if n <= 0 { return 0 } return rec(n-1) + 1 }
print(to_string(rec(300)))
# → [KUJOVM001] Maximum VM call stack depth of 256 exceeded while calling rec   (GOOD: catchable)
```

**B — the native abort (the bug):**
```
func build(n) { mut d := {"v":1} mut i := 0 while i < n { d = {"c": d} i = i+1 } return d }
x := build(600)
print("ok")
# debug build → "fatal runtime error: stack overflow, aborting"  ("ok" never prints)
```
Note `build` is **iterative** (a `while` loop, logical call depth 1) — so this
is *not* call-depth recursion. The overflow happens while constructing /
returning / dropping the nested value, before any `to_json`.

**C — depth threshold (shows it's native-stack-size bound, not a logical limit):**

| nested dict depth | debug `target/debug/kujo` | release `target/release/kujo` |
|---|---|---|
| 300 | ok | ok |
| 600 | **abort** | ok |
| 1500 | **abort** | ok (abort a bit higher) |

Release tolerates ~5× deeper → classic "native frame size × recursion depth
exceeds thread stack." A `fat(250)` recursion with 400 locals per frame does
**not** overflow — so it isn't call depth or frame width on their own; it's
**unbounded native recursion over the `Value` graph**.

## Root cause
`Value` derives `Clone` and relies on the compiler's default `Drop`
(`src/interpreter/value.rs:53` and the VM's value type in `src/bytecode.rs`,
several `#[derive(Debug, Clone, PartialEq)]`). For a recursively nested variant
(Dict/Array holding `Value`s), these are **recursive on structure depth**:
- **`Drop`**: dropping a deeply nested `Value` recurses (drop child → drop its
  child → …). Most likely trigger in repro B (the old `d` and the final result
  are dropped recursively).
- **Derived `Clone`**: cloning on argument-pass / dict-field assignment recurses
  the same way.
- Likely also `PartialEq` and any serialization (`to_json` / `to_string`) over
  nested values.

On a ~2 MB tokio worker stack this overflows at modest depth and there is **no
depth guard on these data operations** (unlike the 256 call-depth guard), so it
`abort()`s instead of erroring.

### Why it hit a real program at shallow apparent depth
A real Kujo CLI (a browser-QA tool, `lens`) passed large nested dicts (a
provider result: `result → viewports[] → {console[], network[], dom{…}}`)
through chains of ~5–6 function calls, each call cloning the structure. The deep
call chain + repeated recursive clones/drops of the nested value consumed the
thread stack together, so it aborted at ~5–6 logical frames with only ~5-deep
data — much shallower than repro B in isolation. Workarounds that fixed it
(routing deep work to top-level functions, shrinking giant functions, avoiding
passing big nested values deep) all reduced cumulative native stack — consistent
with this root cause.

## Recommended fixes (ordered)

1. **Immediate, high-impact, low-risk — give execution a big stack.** The VM
   currently runs on a default tokio worker. Either:
   - Build the runtime explicitly with a large worker stack:
     `tokio::runtime::Builder::new_multi_thread().thread_stack_size(64 * 1024 * 1024)…`
     instead of `#[tokio::main]` defaults (`src/main.rs:1104`); **or**
   - Run the actual program execution on a dedicated
     `std::thread::Builder::new().stack_size(256 * 1024 * 1024).spawn(…)` and join it.
   This raises the ceiling ~30–100× and converts most real-world aborts into
   either success or the graceful 256 guard.

2. **Make `Value` `Drop` iterative.** Add a manual `impl Drop for Value` that
   dismantles nested Dict/Array iteratively with an explicit work stack (the
   standard pattern for recursive data structures) so dropping deep values can
   never overflow.

3. **Add depth guards to recursive `Value` operations** (clone-on-pass,
   `to_json` / `to_string`, equality) that return `KUJOVM001` ("maximum data
   nesting depth N exceeded") instead of recursing unbounded — mirroring the
   existing 256 call-depth guard. Reuse `runtime_limits.rs`.

4. **Optional, reduces clone pressure:** share nested containers via `Rc`/`Arc`
   (copy-on-write) instead of deep-cloning `Value` on every argument pass / field
   assignment. Also helps performance for programs that thread large structures
   through many calls.

## Acceptance criteria
- Repro B (`build(600)` … `build(5000)`) prints `ok` or returns a catchable
  `KUJOVM001`, but **never** `fatal runtime error: stack overflow, aborting`.
- `try { x := build(100000) } except e { print("caught") }` prints `caught`
  (graceful, not abort).
- Repro A still returns the existing graceful 256 message (don't regress it).
- Deeply nested `to_json` / equality of a nested value errors gracefully rather
  than aborting.

## Repro commands
```bash
# B (abort, debug):
printf 'func build(n){mut d:={"v":1} mut i:=0 while i<n {d={"c":d} i=i+1} return d}\nx:=build(600)\nprint("ok")\n' > /tmp/b.kujo
./target/debug/kujo run /tmp/b.kujo        # → aborts
./target/release/kujo run /tmp/b.kujo      # → ok (raise to build(5000) to abort)
# A (graceful guard, control):
printf 'func rec(n){if n<=0{return 0} return rec(n-1)+1}\nprint(to_string(rec(300)))\n' > /tmp/a.kujo
./target/debug/kujo run /tmp/a.kujo        # → KUJOVM001 (catchable)
```

---
*Filed from debugging `lens` (browser-QA CLI), 2026-06-02. The overflow
surfaced repeatedly when threading large nested provider/bridge dicts through
deep call chains; mitigated app-side by keeping call stacks shallow, but the
underlying VM behavior (abort instead of graceful error) should be fixed here.*
