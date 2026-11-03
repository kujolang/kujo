# Sieve complete-buffer audit

The unchanged port and a diagnostic clone agree on every checksum in 216
observations across VM, interpreter and JIT. The clone exports its final byte
flags after checksum calculation; no marking, traversal or checksum expression
is changed. Every flag matches an independent trial-division oracle. A second
verification using Ruby `Prime.each` checks the complete buffers, checksums,
output hashes and frozen input identities independently of the collector.

Coverage: limits 2, 3, 4, 8, 9, 10, 24, 25, 26, 48, 49, 50, 120, 121, 122,
1000, 1024 and 4096; initial accumulated checksums 0 and 4294967294; two
iterations per case. Limit 1000 is the registered test-size limit. The initial
checksum near the u32 boundary verifies accumulation wrapping. Square boundaries
exercise marking termination; the complete byte buffer includes composite and
prime entries, not only the last prime and count.

Frozen CLI SHA-256:
`b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.
It contains the accepted loop identity correction, not the pending memory fix.
The collector verifies upstream Rust/Goose source against commit
`3c9f84ddaeec834c0631747cb789e8cfd602f993`. Source review finds the same byte
initialization, square-root marking boundary, p-squared starting point, odd-number
counting traversal and wrapping checksum. Rust and Goose were not executed here.

Reproduce with `probes/audit-sieve-state.rb`, passing the pinned arena checkout,
frozen Kujo executable and a new output directory. The collector preserves both
original and diagnostic functions and verifies their results agree. The source,
outputs, identities and independent-verification receipt are archived in
`sieve-state-audit-2026-11-03.json.gz`.

This is partial structural evidence. It does not establish native execution
coverage, equivalent operation counts, startup costs, comparator performance,
invalid limits below two, production-size completion or production iteration
counts. Runs overlapped a compiler build and carry no speed claim. No benchmark
port or runtime source was changed; the all-50 ledger row remains open.
