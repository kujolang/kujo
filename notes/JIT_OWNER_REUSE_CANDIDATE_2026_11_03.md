# Exclusive code-owner reuse candidate

Base: `cd5f2b20` on `codex/native-performance-campaign-20261102`.
Candidate branch: `codex/jit-owner-reuse-20261103`.

This candidate is not integrated or accepted. It begins the separately gated
reuse implementation described in
`benchmarks/langarena/JIT_OWNER_REUSE_BOUNDARY_2026_11_03.md`.

The first patch places the private compiler, function/direct-argument entry
maps, native name metadata and inline call cache in one exclusively owned
`VmCodeOwner`. The inline cache also contains a native function pointer and must
not outlive that compiler. Reclamation moves from VM destruction to owner
destruction; the standalone public compiler contract stays unchanged.

Invocation state and function admission counters stay in the VM. No pool,
task-route change, concurrent sharing, new unsafe trait implementation, or speed
claim is included. A regression moves the owner to a fresh VM on another worker,
drops the original VM first, requires the same compiled entry, executes it and
checks returned values after final owner destruction. Existing lifetime, code
identity, callback and generator tests remain applicable.

Additional boundary regressions execute code against different VM globals,
retain embedded string literals through both VM destructions, and toggle the
previous/current cancellation tokens independently. They preserve the compiler's
cancellation-check policy while requiring each invocation's current token. The
targeted platform workflow also runs the owner boundary without JIT on Linux.
These checks are pending execution, not proof that task pooling is safe.

The next patch records per-compiler compilation attempts, successful definitions,
completed finalizations and emitted machine-code bytes across offset/loop,
function, direct-argument and script compilation. A loop cache hit must not grow
these counters; changed bytecode must. Rejected compilation increases attempts
without claiming successfully finalized code. Definition bytes are recorded
before context clearing/finalization; unknown buffer size saturates accounting
instead of appearing free. These counters are **not total retained memory**:
backend code-page rounding, GOT/PLT, compiler metadata and embedded owners need
separate bounds/resource evidence before pool acceptance. No pool is enabled.

The workflow offers a Linux-only owner/accounting gate to validate the next
candidate without repeating all platform builds for every preparatory patch.
Final acceptance still requires the applicable full platform refresh.

Visible metadata now has a separate checked quota weight covering table capacity,
loop slot vectors, profiling tables, function names, dictionary keys, shapes,
field/path names, append strings, literal keys/payloads and exposed backend
declaration names/signatures. Shared payloads may be charged more than once.
Unknown literal kinds reject accounting. A regression covers large string/byte
literals, deduplicated retention, spare table capacity and an unknown payload.
This is a logical retention quota, **not a measured total-memory bound**;
backend-private capacities, relocations, symbol/GOT/PLT storage, allocator/control
bytes and page rounding remain outside it. Reset/drop of transient compiler
context and the separate physical-resource gate remain required before accepting
pooling. The method is staged and not yet connected to task execution.

Verification is pending. Run the release library suite, no-JIT coverage and
callback/task/generator contracts before accepting this boundary. The subsequent
pool requires cancellation-policy matching, lineage propagation, admission/reset
policy, explicit code/metadata budgets, whole-owner eviction and panic/reentry
coverage. Those requirements remain open; this patch alone does not reduce
compilation counts.

Build/test gates should run on CI while the frozen local Brainfuck production
sequence continues. Do not substitute this candidate for its frozen executable
or change the canonical port during that sequence.
