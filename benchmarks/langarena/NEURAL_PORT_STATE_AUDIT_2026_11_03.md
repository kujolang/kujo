# NeuralNet full training-state audit

Pinned LangArena 3c9f84ddaeec834c0631747cb789e8cfd602f993 Rust neural_net.rs
construction, connection, training, weight-update and forward methods were
compiled and executed unchanged. Only the benchmark wrapper/import was removed;
a standalone harness supplies the unchanged pinned RNG helper prefix and prints
normalized complete states. Goose source was pinned/read, not executed.

The frozen accepted Kujo runtime (SHA-256
17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325) matches all
399 Rust states in each of VM/interpreter/JIT: 1,197 complete observations and
134,064 floating-point comparisons, with observed maximum absolute difference
zero. Collector policy permits 1e-12 relative/absolute tolerance for portability;
independent review confirms exact numeric equality in this run.

Seeds 42, 1 and 139967 each cover preparation, 128 individual training examples
in official XOR order, then four forward evaluations. Every state includes all
13 neurons' thresholds/previous thresholds/outputs/errors/incoming/outgoing links
and all 30 synapses' weights/previous weights/endpoints. Rust layer-local indices
are normalized to Kujo's global indices only for serialization. Each Kujo update
asserts its retained prior network snapshot is unchanged. Twelve negative controls
alter a weight, link, output or previous weight and fail full-state equality.

Work/storage remains different: Rust uses separate typed layer vectors, Goose
neuron references and bounded inline link arrays, Kujo global struct arrays and
value-semantics function results. This audit does not count transfers/allocations,
prove native execution of each JIT region, execute Goose, or complete 4,000-step
benchmark iterations/full production. No runtime or canonical port change.

Initial harness attempts failed before comparison: a redundant Rust helper import
and serialization of Kujo structs directly to JSON. The successful fixture uses
explicit field projection, preserving all semantic fields. These were harness
issues, not runtime regressions. Raw source/oracle/fixture/output/receipt evidence
is archived in neural-state-2026-11-03.json.gz; local originals and standalone
oracle executable are at /tmp/kujo-neural-state-review-v4. Reproduce with
probes/audit-neural-state.rb FROZEN_KUJO PINNED_UPSTREAM NEW_OUTPUT, then
probes/verify-neural-state.rb OUTPUT.
