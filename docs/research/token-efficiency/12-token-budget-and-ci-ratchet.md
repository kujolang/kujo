# Token budgets and CI ratchet

## Proposed dimensions

Track base contract, selected skills, references, tool schemas, task/spec, repository discovery, retrieved source, handoffs, tool results, retries, and output. Each dimension needs measured provider usage where available and deterministic estimates otherwise.

Derive budgets from at least 20 runs per scenario after instrumentation. Use median and high-percentile baselines rather than arbitrary numbers. Establish:

- warning threshold for review;
- hard regression threshold for unexpected growth;
- explicit approved-growth record containing reliability evidence and an updated baseline.

## CI design

Check normalized, sorted, redacted payload snapshots; component byte/character/heuristic counts; provider usage fixtures; and end-to-end scenario receipts. Freeze serialization versions and exclude timestamps, random IDs, machine paths, and nondeterministic logs. Baseline updates require a deliberate command, diff review, and task-success/security test results.

Tokenizer differences must be reported as separate columns. A whitespace-only change can still affect provider billing, so do not rely solely on byte counts.

## Dispatch and cache gates

Budget CI must also assert that the intended role, skill, and tool actually ran, using structured lifecycle receipts rather than configuration text. Prefix reuse or provider prompt caching may reduce billed/latency cost while leaving logical context unchanged; CI must report those savings separately and must not treat a cache hit as justification for oversized payloads.
