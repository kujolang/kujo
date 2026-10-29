# Nested initial dictionary insertion controls

Run `nested-insertion-cold-controls.rb BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT`.
It refuses an existing output directory. Preserve both executables and the report.

The matrix covers dictionary, struct and array owners; 0/2/128/2,048 initial
entries; 1/4/5/64 new entries per helper call; 1/2/16/32 calls; and no initial
snapshot versus a fresh snapshot on every call. It asserts full final contents
and old snapshot contents. VM/JIT probes measure allocation requests and requested
bytes; VM/interpreter/JIT CLI runs assert semantics. Expected coverage is 768
paired allocation observations and 1,152 CLI semantic observations.

Reject any increase in total requests/bytes or the paired 1→2 and 16→32 call
slopes. Independently verify the Cartesian case sets, fixture and executable
hashes, source files and recomputed slopes. Baseline self-comparison must pass
before accepting a candidate. Traffic is not retained memory or elapsed speed.

`tests/nested_dictionary_initial_insertion.rs` additionally exercises immediate
reads, callback publication, errors, numeric/string aliases, independent old
owners and leaf replacement in all three runtime modes and all three owner shapes.

Baseline `4cd3bf1` passes this gate and the new contract. See
`../nested-insertion-baseline-2026-10-28.json`. Width 128→2,048 adds 587,520 requested
bytes per added one-key call across all three owner shapes, VM and JIT.

The generated `k + offset` writes exercise inline overflow/fallback, not every
spill-proof path. Before candidate acceptance also run literal/parameter-key spill
controls, the existing nested fallback/error/deep-owner matrices, full relevant
regressions and all 150 test-size checksums. Review memory and artifact identities.
The candidate is not accepted by a baseline-only pass.
