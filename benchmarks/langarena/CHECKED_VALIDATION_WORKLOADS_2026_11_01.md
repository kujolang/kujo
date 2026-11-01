# Checked-assignment workload review, 2026-11-01

All 72 observations pass independent verification: four alternating pairs for
QuickSort, MergeSort and NeuralNet in VM, interpreter and JIT. Baseline runtime
is `55e2f683`; candidate is `568dc612`. Exact binary/source hashes, invocations,
unchanged test-size configuration, checksums and timing markers are verified.
The frozen baseline CLI runs both versions. No compiler was active at admission.

## Results and limitations

Ratios are baseline divided by candidate; above 1 favors the candidate. Medians
use four paired ratios. Kernel time excludes warmup; elapsed includes target
setup/warmup; child CPU includes runner and children. CPU time still reflects
frequency changes and other loaded-laptop effects. These are diagnostic results.

| Workload | Runtime | Kernel median | Elapsed median | Child CPU median | Kernel pair range |
|---|---|---:|---:|---:|---:|
| Sort::Quick | vm | 0.959 | 0.973 | 0.964 | 0.870–1.074 |
| Sort::Quick | interpreter | 0.948 | 0.977 | 0.970 | 0.824–1.063 |
| Sort::Quick | jit | 1.038 | 1.031 | 1.030 | 0.984–1.118 |
| Sort::Merge | vm | 0.954 | 0.974 | 0.966 | 0.811–1.218 |
| Sort::Merge | interpreter | 0.963 | 0.988 | 0.985 | 0.817–1.080 |
| Sort::Merge | jit | 1.003 | 1.033 | 1.026 | 0.973–1.103 |
| Etc::NeuralNet | vm | 0.962 | 0.953 | 0.963 | 0.858–1.054 |
| Etc::NeuralNet | interpreter | 1.038 | 1.039 | 1.040 | 1.020–1.133 |
| Etc::NeuralNet | jit | 1.060 | 1.070 | 1.081 | 0.983–1.289 |

All three VM kernel medians favor the baseline, corresponding to roughly 4–5%
longer candidate times. QuickSort interpreter has a 0.948 ratio (roughly 5.5%
longer candidate time). These losses are retained for clean-host follow-up.
QuickSort/NeuralNet VM each have three slower pairs and one faster pair;
MergeSort VM and QuickSort/MergeSort interpreter each split two/two. The ranges
are broad and the laptop load was uncontrolled. This does not prove the losses
are caused by load, nor establish a reliable runtime regression. Likewise the
favorable JIT medians do not establish a general language speedup. No Goose/Rust
ranking or production-size performance claim follows.

## Acceptance decision

Accept the narrow duplicate-validation removal for integration based on direct
reproduction and structural verification, preserved ownership/error/depth rules,
1,343 local release passes, all 150 unchanged checksum cases, seven successful
Linux CI jobs, and the completed workload review. The production change replaces
one redundant validation wrapper with its already-validated writer; the counter
is test-only. No new cache, retained owner, allocation, benchmark shortcut or
weakened check is introduced. The exact-depth repair fallback is unchanged.

This decision accepts removal of demonstrated redundant work; it does not claim
the observed elapsed-time losses are resolved. The VM and interpreter timing
losses require controlled-host follow-up before a speed claim. Repeating the same
uncontrolled laptop round would not settle that question. Broader section 5,
production coverage and competitive acceptance remain open.

The [294-artifact workload bundle](checked-validation-workloads-2026-11-01.json.gz)
contains raw reports/output, collector, verifier and completed state. Every
embedded artifact hash was checked after compression. The
[regression and CI report](CHECKED_VALIDATION_REGRESSIONS_2026_11_01.md) records
the semantic gates and their evidence.
