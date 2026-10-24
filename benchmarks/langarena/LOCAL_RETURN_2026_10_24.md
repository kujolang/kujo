# Discarded native function locals — 2026-10-24

Runtime `93e7caa`, compared with `16b96d5`. Reproduction and measured counts
are retained in `native-local-return-2026-10-24.json`.

Successful native function returns copied dirty SSA locals into a temporary
map that the VM then discarded. The earlier allocation histogram sampled
hash-map growth in `jit_store_variable`. VM-managed whole-function compilation
now omits that successful-return persistence. Public standalone compilation,
loop/script compilation, named/global assignment and error synchronization
keep their existing behavior. Direct-argument compilation is unchanged.

The return value is still extracted through the existing ownership boundary;
normal frame teardown releases local roots. No value checks or copy-on-write
rules are removed. Builds without JIT retain the disabled-JIT error contract.

## Structural measurement

Default-feature debug library, macOS x86_64. The same warmed QuickSort probe
and unchanged input generator were used before and after. This measures
cumulative allocation requests, not elapsed time or peak resident memory.

| 8,192-element JIT case | Before requests | After requests | Before requested bytes | After requested bytes |
| --- | ---: | ---: | ---: | ---: |
| Unique input | 124,380 | 58,534 | 28,608,056 | 9,278,520 |
| Shared input | 124,376 | 58,530 | 29,720,376 | 10,390,840 |

The unique case uses approximately 53% fewer allocation requests and 68% fewer
requested bytes. Its 660- and 1,304-byte buckets disappear. Unique inputs retain
their backing and make zero array-sized allocations; shared inputs detach once
and preserve the saved snapshot. Sorted outputs are checked element by element.
Sizes 128, 1,024 and 8,192 pass in both VM and JIT configurations.

The probe's optional `--expect-local-return-budget` asserts fewer than 65,000
requests and 11,000,000 requested bytes for both 8,192-element JIT cases. It
passes the candidate and fails the previous runtime's allocation-count gate.
`--expect-reuse` continues to check the earlier, broader root-reuse budget.

## Verification

- 1,090 library tests pass; 8 intentionally ignored.
- A new native unit test checks scalar/null returns, an empty unallocated local
  map after successful VM-managed calls, preserved standalone local maps,
  error-state persistence and root/depth teardown.
- 38 targeted integration tests pass, including VM/interpreter/JIT returns,
  snapshots, recursion, globals, argument ownership and loop-error state.
- `cargo check --no-default-features`, `cargo fmt --check` and `git diff --check`
  pass. The no-default check caught a missing shim method before acceptance;
  the forwarding method is included in this commit.

The first integration link attempt exhausted local disk space. Cargo cleanup
removed this checkout's disposable debug build artifacts; serial compilation
with incremental caches disabled then passed. Source and evidence were retained.

## Frozen release verification

`local-return-release-2026-10-24.json` verifies **150/150 test-size checksums**
on frozen `93e7caa`, with registry/configuration/output checks and source/binary
identities. Seven rotating QuickSort rounds compare it with frozen `16b96d5`:

| Runtime | Before median seconds | After median seconds | Median paired speedup | Favorable pairs |
| --- | ---: | ---: | ---: | ---: |
| VM | 8.1191 | 8.1557 | 0.9927x | 3/7 |
| JIT | 2.5272 | 2.2930 | 1.1631x | 6/7 |

All 28 timing samples pass their checksums. CPU speed limits changed from 100%
to 71%; the JIT result is favorable local evidence, not a clean-server guarantee
or a Goose/Rust comparison. The unchanged VM path has no demonstrated speedup.

Two `/usr/bin/time -l` samples per runtime/variant also pass checksums. Mean
maximum RSS is 51.54 → 49.07 MiB for VM and 49.03 → 49.28 MiB for JIT.
With only two samples and host variation, these are descriptive measurements;
they do not establish a JIT peak-memory reduction. Cumulative requested bytes
and resident memory measure different things.

An earlier timing/memory pass overlapped agent-started compilation work. It was
preserved under `superseded_measurement_pass` and repeated after those jobs
finished, using the same frozen binaries. The table above uses the repeat.

This verification predates `9f401eb` and subsequent argument-buffer edits.
Production completion, clean-server comparison and supported-platform gates
remain open.
