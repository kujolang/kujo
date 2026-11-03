# Idle compiler reset: targeted Linux gate verified

Exact source `06e896cf21fe4069504d35eebc5ac4863458f91d`, run `41632651515`,
job `125905850886`, artifact `13230561364`: **72 targeted passes** independently
verified: six owner tests, sixty integration tests, two minimal-feature tests and
four accounting/reset tests. API provenance, source/host, exact test membership,
builds, summaries and artifact hashes were checked.

The extended loop test requires cleared transient table capacities, fresh loop
admission, the same compiled loop pointer and unchanged compilation/byte counters
after idle reset. Changed bytecode must still create a distinct compiled entry.
The rejection test covers changed cancellation policy, disabled compiler,
attempt/definition imbalance and legacy offset-only entries. The three existing
accounting tests and all sixty selected task/callback/generator tests also pass.

Six verifier controls accept original/CRLF logs and reject duplicate/fabricated
integration membership and fabricated accounting/no-JIT tests. Raw evidence and
verifier sources are preserved with checked SHA-256 hashes in
`jit-owner-reset-06e896cf-2026-11-03.json.gz`.

This verifies the reset prerequisite only. Later bounded storage, task-route,
callback/mapper propagation and recovery tests require their own gates. The
result is not full-library, all150, physical-resource or performance acceptance.
