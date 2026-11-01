# Reusable JIT loops: final acceptance, 2026-11-01

Accepted runtime source: `348a62700179e59beccaefddc7ed4da30992d67b`.
Integration preserves the exact tested source, tests, scripts, Cargo files and workflows.
This completes this section 5 loop fix; the bounded sections 3/4 cutoff was already complete.

The fix preserves loop-exit operands, reuses chunk-scoped compiled loops, keeps statistics and invalidation coherent, and bounds repeated callback fallback retries. A successful native exit clears retry backoff. Short subsequent calls can remain in the VM until that delay expires; correctness and dynamic admission checks remain active.

## Verification

- Mac: 1,312 library and 37 targeted integration passes; eight existing ignores. Linux: 1,313 library and 37 integration passes; eight ignores. The additional Linux test is explicitly platform-gated.
- All 150 test-size checksums pass across VM, interpreter and JIT: [matrix evidence](LOOP_FALLBACK_CHECKSUMS_2026_11_01.md).
- All seven Linux workflow jobs pass at exact candidate revision, including full release gate, Clippy, MSRV and no-JIT parity: [gate evidence](LOOP_FALLBACK_RETRY_LINUX_2026_11_01.md). Optional benchmark smoke and unavailable audit/deny tools were not verified.
- Callback native-entry counts fall from 801 to 11 and recovery-case counts from 503 to 11, preserving effects and successful recovery: [exact controls](LOOP_FALLBACK_AFTER_2026_11_01.md).
- Large call-heavy loops reduce VM call counts from 600,602/600,402 to 701; precomputed controls retain 402: [structural evidence](LOOP_REUSE_CALL_COUNTS_2026_11_01.md).

## Paired local timing

108 observations cover six unchanged fixtures, three runtimes, two revisions and three alternating pairs. Baseline is `fd7b764`; candidate is `348a6270`. Compilation was finished before timing. Elapsed measurements include startup and runtime JIT compilation. Ratios below are medians of paired baseline/candidate elapsed times; above 1 means faster candidate.

| Runtime | Shape | Width | Ratio |
|---|---|---:|---:|
| vm | condition | 300 | 1.033× |
| vm | condition | 3000 | 1.001× |
| vm | body | 300 | 1.016× |
| vm | body | 3000 | 1.014× |
| vm | precomputed | 300 | 1.030× |
| vm | precomputed | 3000 | 1.022× |
| interpreter | condition | 300 | 1.003× |
| interpreter | condition | 3000 | 1.002× |
| interpreter | body | 300 | 1.035× |
| interpreter | body | 3000 | 1.019× |
| interpreter | precomputed | 300 | 1.005× |
| interpreter | precomputed | 3000 | 0.991× |
| jit | condition | 300 | 1.498× |
| jit | condition | 3000 | 1.743× |
| jit | body | 300 | 1.559× |
| jit | body | 3000 | 1.755× |
| jit | precomputed | 300 | 3.500× |
| jit | precomputed | 3000 | 17.428× |

These isolated controls show JIT improvements, with VM/interpreter near parity. The busy laptop prevents clean-host speed acceptance. These are not whole-suite, production-size, Goose or Rust comparisons. The 17.43× control must not be generalized to Kujo overall.

[Raw paired observations and Mac verification bundle](loop-reuse-final-review-2026-11-01.json.gz) contains 229 independently hash-verified artifacts. Historical provisional statuses in raw receipts are superseded by this completed acceptance review.

## Remaining scope

No further round is required for the bounded sections 3/4 cutoff or this loop fix. Broader capture/fallback and collection representation costs, additional section 5 optimization, full production coverage and clean-host competitive acceptance remain open. Earlier 26/150 production evidence belongs to `fd7b764`; it does not certify the new revision. Further fixes require attributed workloads and their own regression gates; no reliable total number of future rounds is established.
