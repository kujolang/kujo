# Bounded native loop fallback retries

Pinned aae3919f reproduces 801 native entries with zero completions for three
300-iteration callback-condition scans. A recovery control performs one
600-iteration callback scan, rebinds the callable to builtin `len`, and performs
two more scans: 503 native entries, two completions. Both controls preserve exact
callback counts and results in VM, interpreter and JIT. Before evidence resides
in `.audit-evidence/loop-fallback-retry-2026-11-01`.

The candidate retains the existing chunk/start/end admission key and adds a
bounded retry state to that site's counter. Returning to VM inside a loop delays
the next native attempt by 1, 2, 4, ... up to 256 backedges. A successful native
exit clears the delay. The VM continues executing during the delay; all existing
dynamic admission and ownership checks still apply on each native attempt.
This avoids a permanent blacklist for a callable or representation that may
change. It adds no field or allocation to every call frame. A later short call
can remain in VM while the bounded delay expires; this is a tiering tradeoff,
not a language-semantic change.

Tests cover exact retry bounds and reset, distinct chunks and loop sites,
callback effects, and recovery after builtin rebinding. Existing repeated native
execution, nested snapshots, callback errors and recursion suites remain required.
New full runtime verification, unchanged 150-checksum matrix and paired cost
measurements are pending. No candidate acceptance or speed claim is made here.
