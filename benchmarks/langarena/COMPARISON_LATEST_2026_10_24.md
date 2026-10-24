# Latest Kujo comparison — 2026-10-24

Latest local optimized release: `faf9f95`. Original optimization baseline: `f037705`. This is a measurement-only rerun; no runtime changes were made.

Seven rotating rounds across baseline VM, latest VM, latest opt-in JIT, Rust and Goose: **140/140 samples passed checksum checks**. Each sample ran sequentially; no builds or profiling overlapped. First three workloads use unchanged production inputs; QuickSort uses unchanged test inputs. JIT fallback is allowed.

## Median measured-loop seconds

| Workload | Baseline VM | Latest VM | Latest JIT | Goose | Rust |
|---|---:|---:|---:|---:|---:|
| Base64::Encode | 1.471 | 1.505 | 1.565 | 0.501 | 0.308 |
| Json::Generate | 7.979 | 3.710 | 3.804 | 0.382 | 0.452 |
| Etc::LogParser | 0.458 | 0.435 | 0.438 | 0.306 | 0.451 |
| Sort::Quick | 6.418 | 6.282 | 2.444 | 0.016 | 0.014 |

## Paired speedup over the baseline rerun

Greater than 1 means faster. These are medians of seven within-round ratios, not ratios of independently selected times. Ranges show observed variability, not confidence intervals.

| Workload | Latest VM median (range) | Latest JIT median (range) |
|---|---:|---:|
| Base64::Encode | 0.97x (0.95–1.08x) | 0.96x (0.87–0.99x) |
| Json::Generate | 2.17x (2.11–2.20x) | 2.11x (2.03–2.16x) |
| Etc::LogParser | 1.06x (0.97–1.11x) | 0.99x (0.85–1.14x) |
| Sort::Quick | 1.04x (0.99–1.08x) | 2.63x (2.29–2.72x) |

## Remaining competitor gaps

Ratios below are Kujo median time divided by competitor median time; lower is better. Competitor output is rounded to milliseconds, so large QuickSort ratios are approximate.

| Workload | VM / Goose | VM / Rust | JIT / Goose | JIT / Rust |
|---|---:|---:|---:|---:|
| Base64::Encode | 3.00x | 4.89x | 3.12x | 5.08x |
| Json::Generate | 9.71x | 8.21x | 9.96x | 8.42x |
| Etc::LogParser | 1.42x | 0.96x | 1.43x | 0.97x |
| Sort::Quick | 392.61x | 448.69x | 152.73x | 174.54x |

## Total process wall seconds

These include process startup, preparation, warmup, measured work and checksum handling. They are not isolated startup measurements.

| Workload | Baseline VM | Latest VM | Latest JIT | Goose | Rust |
|---|---:|---:|---:|---:|---:|
| Base64::Encode | 1.849 | 1.873 | 2.004 | 0.630 | 0.392 |
| Json::Generate | 10.497 | 5.329 | 5.624 | 0.478 | 0.598 |
| Etc::LogParser | 1.597 | 1.628 | 2.222 | 0.391 | 0.619 |
| Sort::Quick | 10.569 | 10.277 | 4.507 | 0.043 | 0.038 |

## Interpretation and limits

JSON generation and JIT QuickSort show clear gains in every paired round. Base64 shows no improvement; JIT was slower than the original VM in every pair. VM LogParser and QuickSort changes are small with ranges crossing parity. JIT is not uniformly faster than current VM.

LogParser's measured loop is close to Rust, but total process wall time remains materially longer. QuickSort is still far from Goose/Rust despite the JIT gain. The six earlier production timeouts are separate evidence on an older build; this four-case run neither clears nor extends them.

The shared Intel laptop reported CPU_Speed_Limit=100 before and after; host load remained nonzero. This is stronger same-session evidence than comparing separate dates, but not controlled-host certification, native time coverage or an all-50 score. No aggregate score is claimed across mixed workload sizes.

Raw samples, binary/source hashes, host observations and all summaries: [comparison-latest-2026-10-24.json](comparison-latest-2026-10-24.json). Reproducible harness: [compare_current.kujo](compare_current.kujo). Callback-promotion and argument-transfer candidates remain unapplied; implementation remains paused for review.
