# Repeated string-constant allocation baseline

Accepted runtime `fd7b7646e8adf99d1a798302ef9a18978e4c330d` has a reproduced
VM allocation cost when a loop repeatedly loads a string literal as a dictionary
key. No runtime change is included in this measurement.

The matched control binds the same key outside the loop. Both forms perform the
same dictionary lookups and assert the sum and unchanged dictionary. Key widths
are 8 and 256 bytes; loop counts are 200 and 2,000. Warmup and compilation occur
outside allocation phase 1. Phase 1 includes the measured call and fixed marker
costs, so matched differences are more meaningful than absolute totals.

| Runtime | Extra requests per literal read | Extra requested bytes per literal read |
| --- | ---: | ---: |
| VM | 2 | key width + 40 |
| JIT | 0 | 0 |

All three repetitions give identical counts in every group. The 2,000-read,
256-byte VM case requests 602,397 bytes with literals versus 10,397 with bound
keys: 592,000 avoidable requested bytes in this particular comparison. These are
allocator requests, not retained heap, RSS, or an elapsed-time speedup.

All 24 semantic runs pass across VM, interpreter and JIT. All 48 allocation
observations pass. VM records no native entries; each JIT case records two
(warmup and measured call). Positive named-opcode and zero-allocation scalar
controls pass in the instrumented allocator. Interpreter allocation behavior was
not measured.

`Vm::constant_to_value` constructs `Arc::new(s.clone())` for `Constant::String`.
This supports the attribution to repeated literal materialization. NeuralNet's
port uses repeated dictionary-key literals in its loops. This screen does not
establish what fraction of NeuralNet time they consume, and does not identify a
QuickSort hot-loop bottleneck.

## Reproduction and provenance

Collector: `probes/string-constant-controls.rb`. The allocation harness is
`probes/local_isolation_allocations.rs`, linked to the frozen accepted library;
the bundle preserves build command, source digests, exit status, fixtures,
measurements, controls and raw outputs. Accepted CLI SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.
Accepted library SHA-256:
`ffce3ed37b9cd1935ac3898ce042c354024c5b64e21148a255636dabde8bbd2a`.
Do not substitute the experimental binary in `target/release`.

Verify the committed evidence with:

```sh
ruby benchmarks/langarena/probes/verify-string-constant-controls.rb benchmarks/langarena/string-constant-loads-2026-10-31.json.gz
```

Next: evaluate reusable immutable constant storage while preserving value
semantics, bytecode/public API compatibility, cold allocation/retention bounds,
and recursive/imported execution. This is section 5 work, not a reopening of the
completed sections 3/4 cutoff. No runtime acceptance or competitive speed claim
follows from this baseline.
