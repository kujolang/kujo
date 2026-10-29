# Collection replacement fallback rejection

Collection replacement candidate V2 is **rejected**, not shipped. Preserving
publication admission through collection literals fixes the targeted copying,
but the candidate allocates more bytes when a helper writes two distinct keys.

The [complete observations](collection-replacement-fallback-rejection-2026-10-29.json)
contain 64 target allocation pairs and 96 semantic checks, plus 96 two-key pairs
and 144 semantic checks. VM/interpreter/JIT semantics pass. Allocation measurements
cover VM/JIT, with independently verified source identities, coverage and arithmetic.

With an initial retained snapshot, array replacement with a captured counter falls
from 632,067 to 5,408 requested bytes per added call at width 2,048. The candidate
also uses 5,408 bytes at width 128. Other array/dictionary and argument/captured
counter combinations similarly remove the measured width penalty, with no target
total or slope regressions.

The two-key fallback matrix has **48 total and 24 slope regressions**: both array
and dictionary values, widths 2/128/2,048, no retained snapshots, and VM/JIT. Each
additional helper call requests 93 extra bytes; the largest total increase is
2,976 bytes over 32 calls. Allocation request counts decrease, but that does not
excuse higher byte traffic. Fresh-snapshot cases do not regress.

The first entry owns a `Box<Value>`; a second distinct key forces ordinary
publication. This leaves the extra payload allocation without an avoided owner
copy to pay for it. The next candidate retains array/dictionary storage handles
directly and uses a compact payload for other collections. Its fallback gate also
adds struct values. These revisions remain unverified.

The accepted baseline remains runtime `2c91c49`. Rejected candidate CLI SHA-256:
`b70db1cd04b22515f0ef83bba08433f2df874d7903991ccbb5cf5bb0fddb47b8`.
The expanded literal-boundary contract also passed in all three runtimes, but
broader library, integration, checksum and memory acceptance did not proceed
after fallback rejection. No elapsed-time speedup is established.
