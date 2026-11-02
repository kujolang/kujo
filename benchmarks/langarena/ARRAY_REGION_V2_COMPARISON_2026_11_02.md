# Revised array-region comparison

All **126/126 samples** passed independent checksum, configuration, binary,
round-order, timer, truncation and resource-output verification. This is seven
rotating rounds of three unchanged test-size workloads across accepted Kujo
VM/JIT, candidate VM/JIT, Rust and Goose. It is not an all-50 speed score,
production result, independent-session replication or clean-host measurement.

## Result and decision

The sole hypothesis revision did **not establish repeatable target improvement**.
Do not accept or describe this candidate as a performance fix. The fallback
scaling sweep completes the diagnostic record before the candidate is removed;
it cannot substitute a synthetic win for missing target-workload gains. No third
tactical revision is authorized by this experiment's delivery limit. The broader
direct-native-access area remains open, and the next bounded experiment is call
overhead after this experiment closes.

Speedup below one means the candidate took longer. Each pair divides accepted
time by candidate time from the same round; the table reports the median of
those seven ratios.

| Workload | VM paired speedup | JIT paired speedup | JIT pairs faster |
| --- | ---: | ---: | ---: |
| QuickSort | 0.994x | 0.914x | 2/7 |
| MergeSort | 0.999x | 0.964x | 3/7 |
| NeuralNet | 0.935x | 1.009x | 4/7 |

The local host is variable: QuickSort VM paired ratios span 0.799–1.076, and
NeuralNet VM spans 0.831–1.142. These controls do not justify a correction factor
or prove exactly how much slowdown the candidate causes. They also do not turn
the measurements into evidence of a win. The first candidate's promising
three-round NeuralNet observation did not translate into a repeatable win in
this revised seven-round comparison.

## Measured loop medians, seconds

| Workload | Accepted VM | Candidate VM | Accepted JIT | Candidate JIT | Rust | Goose |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| QuickSort | 8.041896 | 8.327926 | 1.991013 | 2.179430 | 0.019 | 0.020 |
| MergeSort | 21.870731 | 21.772460 | 7.326287 | 7.166875 | 0.020 | 0.015 |
| NeuralNet | 7.870565 | 8.418423 | 4.988395 | 4.957909 | 0.003 | 0.003 |

The ratio of these separate medians is not the median paired ratio. In particular,
MergeSort's separate JIT medians look favorable while its paired median does not;
retain the actual pairs and their order when assessing the candidate. Rust and
Goose report milliseconds, so tiny denominators have limited resolution. Port
work/layout differences remain as documented in the investigation and audits;
these numbers do not isolate compiler quality.

Candidate/accepted JIT peak-RSS medians were 51,700/51,316 KiB for QuickSort,
95,104/83,796 KiB for MergeSort and 34,836/33,576 KiB for NeuralNet. These are
whole-process peaks, including compilation, not retained heap or executable-code
leak measurements. The larger MergeSort peak is an additional investigation
signal, not an established allocation-site diagnosis.

## Provenance and limits

Accepted CLI: `3d6b3ea8e621d6bf8f74ce2cd8f102bc2da0629221b6d3fdc091640ff4e790b3`.
Candidate CLI: `acf5569da1fa6d789240aedbce813558aa387318da38f46dd064a9933b03471f`.
The candidate was copied before measurement; its exact all-runtime correctness
gates are in [the revised receipt](ARRAY_REGION_V2_CORRECTNESS_2026_11_02.md).
"Accepted" means the current frozen checkpoint, not the original historical
Kujo runtime. No competitor was rebuilt for this experiment.

No builds ran during these samples. The local controller verified source and
binary identities around workloads and retained all samples in rotating order.
An earlier wrapper attempt failed before sampling because an output directory
was missing. Its failure is retained, the directory creation was corrected, and
the successful run used a fresh output root. No runtime failure was suppressed.

`array-region-v2-comparison-2026-11-02.json.gz` retains **398 hash-verified
artifacts**, including all raw sample outputs, generated timed sources, inputs,
resource receipts, manifests, controller, verifier and prior wrapper failure.
`probes/verify-array-region-pilot.rb` now validates the recorded odd round count
and explicit candidate path; it completed successfully on these 126 samples.
No projection of clean-host speeds or statistical-significance claim is made.
