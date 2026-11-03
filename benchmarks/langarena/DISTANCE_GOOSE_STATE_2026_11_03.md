# Goose distance preparation and wrapper state

Executed Goose matches **all 64 recorded Rust states** from the six-configuration
distance audit, including all generated strings in order, RNG state, algorithm,
phase and accumulated checksum. Kujo previously matched these same states in
each of VM/interpreter/JIT, for 192 Kujo observations. No new runtime or port
implementation is introduced.

The collector copies the unchanged pinned `distance.goose`, `helper.goose` and
`bench.goose` at `3c9f84ddaeec834c0631747cb789e8cfd602f993`. Its separate main
constructs explicitly configured Jaro/NGram states, then invokes original
prepare/run/checksum functions. It records the same preparation, one warmup,
RNG reset and each subsequent iteration as the Rust/Kujo diagnostic. Both
official test-size input/checksum combinations are included; other configurations
are diagnostic, not official benchmark results. The CLI configuration loader is
not exercised.

Pair strings are rendered directly inside JSON quotes because the audited
generator emits only `a` through `j`; this is not a general JSON string encoder.
Every byte matches the independently reconstructed RNG reference. Goose uses
its original prepared byte-array representation, not a converted Kujo port.

The independent verifier checks exact full-state equality, source/compiler/
binary identities, command exits and raw-output hashes. It rejects a changed
RNG state that preserves all checksum values. The same verifier's previous
800-score mode still passes its original pair evidence and negative control.
The original Rust/Kujo full-state evidence is reverified separately. Executable
Goose output is no longer a missing gate for these bounded pair/state controls.

Remaining work includes production input sizes/iterations, wider numeric and
configuration boundaries, operation/copy/conversion counts and native execution
coverage for the Kujo paths. Per-call Kujo UTF-8 encoding remains unchanged and
unmeasured as an allocation/time contribution. These correctness checks do not
prove equal work, production completion or competitive speed.

Reproduce:

```text
ruby benchmarks/langarena/probes/audit-distance-goose-state.rb UPSTREAM GOOSE_COMPILER VERIFIED_STATE_EVIDENCE NEW_OUTPUT
ruby benchmarks/langarena/probes/verify-distance-goose.rb NEW_OUTPUT VERIFIED_STATE_EVIDENCE --state
```

`distance-goose-state-2026-11-03.json.gz` preserves pinned and generated source,
standalone C, raw states, commands, identities, collector, verifier and referenced
case/Rust output. Builds use the same existing Goose compiler, standalone `-O2`
and Apple clang `-O2 -w -lm -pthread` as the pair audit. Executables are identified
by hash, not embedded. No timing or new memory result is claimed.
