# Shared-helper current-runtime refresh — 2026-10-27

Runtime `65a58f2` still incurs the split-frame cost reproduced on `ce923d1`.
A same-module helper suspends captured locals separately from the module globals
before a foreign callback reenters the closure. All 144 paired allocation cases
have identical requests and bytes on both revisions; all 216 semantic runs pass.
[Machine evidence](split-local-refresh-2026-10-27.json) preserves identical-path
measurements, fixture sources, binary/probe hashes and expanded regression logs.

At depth four, writes request 35,443 bytes per additional outer call with no
extra locals, 160,285 with 128, 2,069,989 with 2,048, and 13,630,073 with 8,192.
These are requested allocation traffic, not elapsed speed or peak memory.

The six existing direct/foreign and shared-helper tests now also verify:

- Captured module-global writes remain consistent with captured-local writes.
- Helpers shadow global names without leaking local bindings into globals.
- Helper arrays retain snapshots across nested callbacks and errors.
- Exact helper invocation/validation counts survive recursion and caught errors.
- Independently captured exported functions retain their original global snapshot.

All six tests pass on both archived runtimes, covering VM, interpreter and JIT.
The standalone Rust test harness was compiled against each exact archived binary
with the recorded command; this avoids rebuilding an unchanged runtime. Formatting
and diff checks pass. The initial draft expected shared globals across distinct
captures; that expectation contradicted the documented snapshot identity and was
corrected before committing. No runtime semantic defect is claimed from that draft.

## Requirements before accepting the next runtime change

Preserve separate global/local restoration, inherited global-write tracking,
public captured owners, helper parameters, snapshots and exception unwinding.
Do not enlarge every activation frame merely to store recoverable owner indices.
Task and bridge routes require their existing ownership controls.

Require no total or per-added-call request/byte increases across the same-path
matrix. Require strict reductions for the nonempty-local depth-four groups and
width-independent extra recursion cost (subtract depth-zero activation) within
the existing retention regimes. If a fixed pool-boundary step remains, measure
multiple oversized widths through 16,384 and report it explicitly; the single
8,192-width sample here cannot establish oversized flatness. Preserve direct,
same-owner foreign, ordinary activation, collection, cold and append controls,
plus all three runtimes' test-size checksum suites.

This refresh and expanded regression coverage are prerequisites. The split-frame
optimization, wider performance checklist and production/clean-host comparisons
remain unfinished. No runtime source changes or Kostya contact occurred.

Follow-up: [split-owner transfer](SPLIT_OWNER_TRANSFER_2026_10_27.md) now records
the accepted fix and complete matching-candidate verification for this route.
