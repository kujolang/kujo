# Shared capture-name component experiment — 2026-10-28

An unconditional conversion of binding-kind map keys from `String` to `Arc<str>`
is rejected under the current cold-allocation nonregression gate. Sharing names
removes repeated string allocations, but increases owner-construction bytes in
30 of 55 tested width/name-length groups. No runtime integration was made.

The existing runtime still clones retained names in oversized private capture
views. This experiment uses the actual `BindingKind` from accepted `ca3403d`,
comparing map construction and invocation-map cloning separately. It does not
execute `Environment::capture_activation_with_pool` or establish full runtime
correctness, throughput, peak memory or retention behavior.

## Results

Requested allocation traffic, 8,192 names:

| Name length | Cold-owner byte increase | Invocation bytes, String | Invocation bytes, shared | Invocation requests, String → shared |
| ---: | ---: | ---: | ---: | ---: |
| 16 | 0 | 671,760 | 409,616 | 8,193 → 1 |
| 17 | 57,344 | 679,952 | 409,616 | 8,193 → 1 |
| 64 | 0 | 1,064,976 | 409,616 | 8,193 → 1 |
| 65 | 57,344 | 1,073,168 | 409,616 | 8,193 → 1 |
| 1,024 | 0 | 8,929,296 | 409,616 | 8,193 → 1 |

At 12,288 names of 16 bytes, cold construction increases by 65,536 bytes.
Invocation cloning saves 327,680 bytes and 12,288 requests. Those repeated-call
savings do not make the owner-construction regression disappear. Most measured
penalties amortize within one map clone, but that is a component allocation
calculation, not evidence that every real owner will make such a call or that
execution is faster.

An initial 24-row experiment used widths 0/128/8,192/16,384 and name lengths
16/64/1,024. Every tested cold total matched, which was insufficient evidence for
an unconditional conversion. The expanded 110-row experiment adds nearby sizes
and uneven name lengths: widths 0/1/3/128/129/2,048/8,191/8,192/8,193/12,288/16,384,
lengths 16/17/64/65/1,024, and both owner/invocation phases. Both experiments are
preserved, including their exact sources, raw outputs and artifact identities.

## Verification and limits

The component checks every name and binding kind against the String-map baseline.
Private maps retain admission membership after owner removal and exclude later
owner declarations. Its allocator counts requests and requested bytes; input
creation is outside the measurement. Exact CSV coverage, nonnegative counts and
source/binary identities are checked by the retained analyzer.

The first standalone link failed because Apple's linker could not read Rust LLVM
dependency bitcode. The successful build uses Rust `-C lto=thin`; this is a build
configuration correction, not a runtime or semantic failure. The analyzer was
also corrected to use `Hash#reject` because the local Ruby lacks `Hash#except`.

Any future implementation must still preserve host-editable public scope behavior,
removal-before/after-admission semantics, immutable binding errors, lexical
membership, detached snapshots, recursion, callbacks, tasks and owner expiration.
Sharing strings alone does not remove map traversal. No collection values or
public owners may be retained merely to save name allocations; idle pool bounds
remain unchanged.

Section 3 remains open. The next design must address cold construction as well as
repeated calls, or explicitly establish a justified tradeoff before integration.
This component experiment does not close a runtime checklist item.

[Machine-readable evidence](shared-capture-names-2026-10-28.json) includes the
probe and analyzer source, compiler arguments and both measurement matrices.
Audit directories: `.audit-evidence/shared-capture-names-2026-10-28` and
`.audit-evidence/shared-capture-names-boundaries-2026-10-28`.
