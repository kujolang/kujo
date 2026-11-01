# Depth-cache workload comparison, 2026-11-01

All 72 observations pass independent verification: four alternating baseline/candidate pairs for QuickSort, MergeSort and NeuralNet in VM, interpreter and JIT. Baseline source is `348a6270`; candidate is `9fb0ed705c59a8da6c6eab36e9f50c3a0722f728`. Exact binaries, workload source, unchanged test-size config, ordered case membership, exit status, unique checksums and timing markers are verified. The accepted baseline CLI is the fixed runner for both versions. No local compiler was running at measurement startup.

## Results and limitations

Each ratio is baseline divided by candidate, so above 1 favors the candidate. Medians are calculated from four paired ratios. Kernel time excludes existing warmup using profile markers; elapsed includes target setup/warmup; child CPU includes the runner and its children. CPU measurements do not eliminate thermal throttling, CPU-frequency variation or all effects of competing laptop activity.

| Workload | Runtime | Kernel median | Elapsed median | Child CPU median | Kernel pair range |
|---|---|---:|---:|---:|---:|
| Sort::Quick | vm | 1.065 | 1.058 | 1.053 | 1.018–1.131 |
| Sort::Quick | interpreter | 0.979 | 0.982 | 0.992 | 0.873–1.080 |
| Sort::Quick | jit | 0.992 | 0.996 | 0.997 | 0.983–1.179 |
| Sort::Merge | vm | 1.119 | 1.106 | 1.087 | 0.854–1.180 |
| Sort::Merge | interpreter | 0.983 | 1.001 | 1.005 | 0.938–1.119 |
| Sort::Merge | jit | 0.979 | 0.981 | 0.980 | 0.921–1.086 |
| Etc::NeuralNet | vm | 1.067 | 1.066 | 1.067 | 1.012–1.127 |
| Etc::NeuralNet | interpreter | 1.008 | 1.032 | 1.032 | 0.876–1.251 |
| Etc::NeuralNet | jit | 1.021 | 1.004 | 1.002 | 0.946–1.193 |

VM kernel medians favor the candidate by approximately 6.5%, 11.9% and 6.7% as speed ratios. Interpreter medians range from 0.979 to 1.008 and JIT from 0.979 to 1.021. These local observations do not establish a whole-language gain or a reliable 2% regression: individual pairs change direction and span wider ranges. MergeSort JIT has two slower and two faster pairs, with roughly 2% lower kernel/CPU medians; it remains a clean-host follow-up rather than an ignored tradeoff. QuickSort JIT is approximately flat. No competitive ranking or production-size speed claim follows.

## Acceptance review

Accept this narrow internal-address hashing change for integration, based on the combined evidence: isolated lookup and maintenance gains across all four owner kinds/counts; smaller empty-cache object; unchanged maintenance/weak-owner policy; new ownership regressions; all 150 unchanged test-size checksum cases; all seven Linux CI jobs; and the completed workload comparison. The measured local VM improvement is supportive but is not the sole reason for acceptance. No benchmark inputs, work, checksums or value semantics were weakened. Loaded-host variability remains explicit; controlled-host follow-up is still required by the broader plan.

The [regression/CI review](DEPTH_CACHE_CANDIDATE_REGRESSIONS_2026_11_01.md) records semantic and retention evidence. The [component comparison](DEPTH_CACHE_HASH_COMPARISON_2026_11_01.md) records isolated improvements. The [294-artifact workload bundle](depth-cache-workload-comparison-2026-11-01.json.gz) preserves all raw reports/logs, collector, verifier and completed state. This acceptance closes only this candidate, not section 5, production coverage or clean-host comparisons.
