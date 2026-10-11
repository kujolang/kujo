# Context and output budget fixture

`cargo test --test token_payload_budget --test release_gate_output_contract`
runs these deterministic regression gates as part of ordinary `cargo test`.

The synthetic chat has 345 compact UTF-8 JSON bytes and an 80-token **heuristic**
estimate for `gpt-fixture`. At a budget of 40, the existing policy retains the
system instruction and latest user question, drops two older messages, and
returns 185 JSON bytes with an estimate of 30. This is a compatibility baseline,
not a newly achieved reduction or provider billing measurement. The fitted
result includes metadata, so its byte count is not just message bytes.

The release-gate fixture emits 100,000 bytes plus a second `done` command. Receipt
mode must print less than 1,024 bytes and at most 256 estimated text tokens with
the ordinary temporary test path,
preserve every command byte in separate logs, preserve exit status 7, stop on
failure, keep separate invocations isolated, and reject an unusable log path.
The ceiling applies to this fixed fixture; receipts include user-selected paths
and are not globally capped by truncating paths or diagnostics.

To approve intentional growth, change the input and expected fitted fixture,
review the retained system/user evidence, recompute compact UTF-8 lengths and
deterministic estimates, update the exact expectations in `token_payload_budget.rs`
and this document, and run both tests plus the affected API/parity tests. Record
the justification in the changelog or audit. Do not blindly regenerate results
or substitute provider-exact token claims. Timing measurements are manual and
have no CI threshold.
