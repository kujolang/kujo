# Nested publication verification gates

These runners compare allocation requests and requested bytes, not elapsed time.
Use probes linked against the baseline and candidate runtime libraries with the
same compiler configuration. Retain their source, library, binary and compiler
argument identities. Each runner requires a new output directory.

```text
ruby benchmarks/langarena/probes/indexed-publication-allocations.rb BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY MODULE_OUTPUT nested
ruby benchmarks/langarena/probes/nested-publication-controls.rb BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY CONTROL_OUTPUT
ruby benchmarks/langarena/probes/dictionary-read-allocations.rb BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY KEY_OUTPUT control
```

The `nested` module gate retains all 128 allocation pairs and 192 semantic runs.
It requires an actual reduction, no request/byte regressions, and flat added-call
allocation across 128/2,048-element rows for both struct and dictionary owners,
with one and eight writes per helper. Passing an unchanged baseline as the
candidate must fail the actual-reduction requirement.

The additional controls cover struct and dictionary rows of 2, 128 and 2,048
elements, repeated writes, dense writes, different rows, escaped snapshots and a
competing direct-array or direct-dictionary write. All 72 allocation pairs must
avoid request/byte regressions, and all 108 VM/interpreter/JIT runs must preserve asserted values and
snapshots. Each competing case returns its direct collection element, avoiding an
unrelated expression that would flush its pending write and hide interference.
Passing identical baseline probes is a runner/fixture check, not an optimization
result.

The dictionary `control` mode retains all 16 read/write pairs and 24 semantic
runs, with no request/byte regression. It does not demand another reduction in
dictionary-read costs when a different operation is being optimized. Existing
`read` and `write` target modes still require their respective improvements.

These are bounded controls. They do not establish general ownership correctness,
universal no-regression behavior, peak memory, production benchmark completion or
competitive speed. Run the runtime regressions, publication-boundary tests, other
allocation controls and all benchmark checksums before accepting a runtime change.
