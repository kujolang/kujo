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

Verification is pending. Run the release library suite, no-JIT coverage and
callback/task/generator contracts before accepting this boundary. The subsequent
pool requires cancellation-policy matching, lineage propagation, admission/reset
policy, explicit code/metadata budgets, whole-owner eviction and panic/reentry
coverage. Those requirements remain open; this patch alone does not reduce
compilation counts.

Build/test gates should run on CI while the frozen local Brainfuck production
sequence continues. Do not substitute this candidate for its frozen executable
or change the canonical port during that sequence.
