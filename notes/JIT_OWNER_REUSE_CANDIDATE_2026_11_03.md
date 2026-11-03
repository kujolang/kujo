# Exclusive code-owner reuse candidate

The initial ownership-bundle commit now has a verified seven-job Linux gate;
see `benchmarks/langarena/JIT_OWNER_LINUX_GATE_2026_11_03.md` for its exact
1,360-pass scope. Later candidate revisions still require their own gates.

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

## First Linux result and feature-guard correction

Run `41627350511`, job `125887926893`, artifact `13226839913`, exact
`cb931728f8cf7d59856f84830b4213a87eec2146`: all six optimized owner-lifetime
tests passed, including cross-worker transfer, fresh globals/literal ownership,
current cancellation and executable-entry revocation. Exact names, source and
summary were independently checked. This is not whole-candidate acceptance.

The same job failed the no-default-features library build with 18 diagnostics;
no no-JIT tests ran, and the integration step was skipped. Five network-only
tests and their helpers referenced `network_policy` or HTTP functions compiled
out by that feature configuration. The four affected source files are byte-for-
byte unchanged from accepted base `cd5f2b20`; the owner extraction did not create
those references. The raw logs and identities, including the failed outcome,
are retained in `benchmarks/langarena/jit-owner-linux-boundary-cb931728-2026-11-03.json.gz`.

The follow-up applies `runtime-network` guards only to those network tests,
helpers and imports. It adds an explicit disabled-network diagnostic test and
keeps the no-default-features gate unchanged. Networking-enabled tests remain
enabled. The correction and new test still need execution; it does not turn the
earlier failed job into a pass.

The workflow now collects default-feature accounting and integration results
after a successful owner gate even when the independent no-JIT gate fails.
Cancellation still stops those steps, and a failed no-JIT step still fails the
job. This prevents an unrelated feature-configuration failure from hiding later
default-feature test outcomes; it does not relax acceptance.
