# Reviewed release campaign measurements

Internal regression evidence only. Each platform has its own A/A control and paired comparison.
Runtime rows report median same-round percent deltas; median ratios can differ.
Static rows show raw median process times without a paired-delta claim.
All samples, warmups, p90/range/deviation, sources and metadata remain in the archives.

## Linux x64 hosted

Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; linux/x64; rustc 1.99.0 (b940084d7 2026-09-28).

| Workload | Baseline median ms | Candidate median ms | Paired delta % | A/A paired delta % |
| --- | ---: | ---: | ---: | ---: |
| calls | 40.227 | 40.341 | 1.40 | 1.09 |
| captures_generators | 49.701 | 50.280 | 1.04 | 0.15 |
| tasks | 112.425 | 112.640 | 0.11 | -0.02 |
| calls_sustained | 724.345 | 728.853 | 0.35 | 0.46 |
| captures_generators_sustained | 455.519 | 457.370 | 0.34 | -0.14 |
| tasks_sustained | 1090.903 | 1088.321 | -0.02 | -0.40 |
| language_collections_structs | 29.272 | 29.498 | 1.12 | -0.36 |
| closures_retained | 26.082 | 26.344 | 0.69 | 0.46 |
| generator_nested_alias | 275.736 | 278.921 | 0.93 | 0.30 |
| promise_observers | 1094.633 | 1097.438 | 0.08 | -0.02 |
| scheduler_channels | 1092.959 | 1094.171 | 0.14 | -0.01 |
| task_admission_rejection | 6.299 | 6.422 | 2.95 | -0.68 |
| task_cancellation | 5.637 | 5.603 | 0.38 | 0.94 |
| ai_native_hash | 4.445 | 4.388 | -1.88 | 0.70 |
| collection_serialization | 17.761 | 17.763 | 0.22 | 1.29 |
| sparse_dictionary | 434.203 | 408.843 | 0.07 | 0.01 |
| schema_regex | 10.600 | 10.951 | 3.03 | -0.53 |
| schema_integer_dictionary | 26.516 | 27.045 | 1.56 | -0.57 |
| ai_context_fitting | 6.307 | 6.340 | 1.12 | -0.68 |
| token_text_ascii | 21.517 | 6.531 | -69.83 | 0.14 |
| token_text_unicode | 18.371 | 16.996 | -7.37 | -0.06 |
| startup | 3.630 | 3.748 | — | — |
| project_check | 4.937 | 5.061 | — | — |
| lsp_diagnostics | 5.706 | 5.864 | — | — |

Binary bytes: 50764368 → 50764752.

Instrumented resource series (baseline from A/A, candidate from paired run; separate from timings):

| Token workload | Baseline CPU ms | Candidate CPU ms | Baseline peak RSS bytes | Candidate peak RSS bytes |
| --- | ---: | ---: | ---: | ---: |
| token_text_ascii | 18.002 | 2.951 | 15466496 | 15466496 |
| token_text_unicode | 14.646 | 13.331 | 15384576 | 15380480 |

## macOS x64 local

Intel(R) Core(TM) i7-9750H CPU @ 2.60GHz; darwin/x64; rustc 1.96.0 (ac68faa20 2026-05-25).

| Workload | Baseline median ms | Candidate median ms | Paired delta % | A/A paired delta % |
| --- | ---: | ---: | ---: | ---: |
| calls | 121.678 | 120.117 | -1.35 | 0.90 |
| captures_generators | 153.513 | 149.206 | -0.43 | 0.68 |
| tasks | 172.681 | 170.968 | -0.24 | -0.13 |
| calls_sustained | 1814.614 | 1831.964 | 1.55 | 0.82 |
| captures_generators_sustained | 2034.097 | 2004.572 | 0.15 | 0.67 |
| tasks_sustained | 1327.194 | 1330.941 | -0.25 | -1.50 |
| language_collections_structs | 78.641 | 78.092 | 2.57 | -0.77 |
| closures_retained | 85.313 | 85.990 | -1.80 | 3.90 |
| generator_nested_alias | 732.199 | 723.127 | 0.33 | -4.49 |
| promise_observers | 1352.020 | 1351.712 | -0.22 | 0.10 |
| scheduler_channels | 1509.862 | 1529.161 | 0.91 | -0.01 |
| task_admission_rejection | 24.088 | 26.431 | 9.59 | -11.56 |
| task_cancellation | 28.629 | 27.298 | -5.61 | -6.83 |
| ai_native_hash | 23.688 | 22.987 | -2.22 | 3.67 |
| collection_serialization | 56.603 | 57.251 | -0.37 | -1.52 |
| sparse_dictionary | 741.180 | 729.578 | -1.62 | 0.36 |
| schema_regex | 43.117 | 43.273 | -0.44 | -0.33 |
| schema_integer_dictionary | 83.875 | 86.848 | 3.54 | -5.09 |
| ai_context_fitting | 27.906 | 26.850 | -4.67 | -7.66 |
| token_text_ascii | 50.539 | 24.367 | -52.50 | 0.83 |
| token_text_unicode | 50.603 | 40.286 | -23.77 | -3.95 |
| startup | 19.767 | 20.375 | — | — |
| project_check | 24.010 | 24.589 | — | — |
| lsp_diagnostics | 24.643 | 25.484 | — | — |

Binary bytes: 44317016 → 44317016.

Instrumented resource series (baseline from A/A, candidate from paired run; separate from timings):

| Token workload | Baseline CPU ms | Candidate CPU ms | Baseline peak RSS bytes | Candidate peak RSS bytes |
| --- | ---: | ---: | ---: | ---: |
| token_text_ascii | 30.345 | 5.323 | 7028736 | 6950912 |
| token_text_unicode | 30.342 | 20.484 | 7000064 | 6987776 |

## macOS arm64 hosted

Apple M1 (Virtual); darwin/arm64; rustc 1.99.0 (b940084d7 2026-09-28).

| Workload | Baseline median ms | Candidate median ms | Paired delta % | A/A paired delta % |
| --- | ---: | ---: | ---: | ---: |
| calls | 39.852 | 42.300 | -0.24 | 3.60 |
| captures_generators | 46.604 | 45.729 | 0.22 | -2.71 |
| tasks | 1094.177 | 1100.609 | 6.20 | -0.40 |
| calls_sustained | 685.554 | 696.740 | 2.38 | 1.17 |
| captures_generators_sustained | 414.568 | 396.963 | -0.40 | -1.11 |
| tasks_sustained | 9532.478 | 9661.566 | 2.07 | -0.81 |
| language_collections_structs | 42.688 | 43.050 | -2.31 | 0.32 |
| closures_retained | 22.200 | 24.039 | 0.85 | -0.40 |
| generator_nested_alias | 301.601 | 270.115 | -9.49 | -1.29 |
| promise_observers | 8671.175 | 8362.810 | -3.47 | 0.00 |
| scheduler_channels | 9566.386 | 9597.489 | 0.55 | 0.83 |
| task_admission_rejection | 19.723 | 19.545 | -5.80 | -0.03 |
| task_cancellation | 12.250 | 12.749 | 1.34 | 2.13 |
| ai_native_hash | 8.059 | 7.853 | -3.75 | 13.08 |
| collection_serialization | 22.227 | 19.309 | -3.15 | 14.43 |
| sparse_dictionary | 803.582 | 798.163 | 0.82 | 2.12 |
| schema_regex | 19.593 | 19.345 | -0.07 | -0.36 |
| schema_integer_dictionary | 29.621 | 29.134 | -2.34 | 4.81 |
| ai_context_fitting | 10.846 | 10.067 | -2.14 | -3.53 |
| token_text_ascii | 33.735 | 10.199 | -69.58 | 1.91 |
| token_text_unicode | 23.240 | 22.614 | 2.30 | -1.14 |
| startup | 7.172 | 7.354 | — | — |
| project_check | 8.364 | 8.529 | — | — |
| lsp_diagnostics | 9.368 | 9.608 | — | — |

Binary bytes: 39918928 → 39918928.

Instrumented resource series (baseline from A/A, candidate from paired run; separate from timings):

| Token workload | Baseline CPU ms | Candidate CPU ms | Baseline peak RSS bytes | Candidate peak RSS bytes |
| --- | ---: | ---: | ---: | ---: |
| token_text_ascii | 26.101 | 2.750 | 9420800 | 9191424 |
| token_text_unicode | 15.889 | 14.968 | 9191424 | 9322496 |

## Windows x64 hosted

AMD EPYC 9V74 80-Core Processor                ; win32/x64; rustc 1.99.0 (b940084d7 2026-09-28).

| Workload | Baseline median ms | Candidate median ms | Paired delta % | A/A paired delta % |
| --- | ---: | ---: | ---: | ---: |
| calls | 50.776 | 50.235 | -0.81 | 0.31 |
| captures_generators | 57.216 | 56.968 | -0.17 | -0.23 |
| tasks | 166.039 | 166.373 | 0.02 | 1.22 |
| calls_sustained | 805.252 | 788.319 | -2.05 | 1.03 |
| captures_generators_sustained | 463.912 | 460.093 | -0.46 | 1.53 |
| tasks_sustained | 1551.921 | 1551.478 | -0.06 | 0.01 |
| language_collections_structs | 43.286 | 42.498 | -0.96 | -0.36 |
| closures_retained | 41.105 | 40.648 | -0.17 | -8.21 |
| generator_nested_alias | 258.380 | 258.662 | 0.42 | 1.69 |
| promise_observers | 1554.712 | 1555.616 | 0.10 | -0.26 |
| scheduler_channels | 1558.842 | 1555.997 | -0.01 | -0.03 |
| task_admission_rejection | 14.162 | 14.053 | -1.11 | -2.49 |
| task_cancellation | 13.658 | 13.533 | -0.47 | 1.01 |
| ai_native_hash | 11.790 | 11.669 | 0.22 | -1.04 |
| collection_serialization | 45.380 | 45.015 | -0.38 | -0.54 |
| sparse_dictionary | 2849.540 | 2894.915 | 1.27 | 1.23 |
| schema_regex | 21.459 | 21.344 | 0.39 | -0.54 |
| schema_integer_dictionary | 39.363 | 39.195 | -0.51 | 0.79 |
| ai_context_fitting | 15.970 | 15.861 | -0.48 | 0.30 |
| token_text_ascii | 27.446 | 13.743 | -50.19 | 0.39 |
| token_text_unicode | 24.839 | 24.898 | -0.36 | 0.09 |
| startup | 10.175 | 10.076 | — | — |
| project_check | 12.646 | 12.686 | — | — |
| lsp_diagnostics | 14.108 | 14.025 | — | — |

Binary bytes: 50080768 → 50081280.

Instrumented resource series (baseline from A/A, candidate from paired run; separate from timings):

| Token workload | Baseline CPU ms | Candidate CPU ms | Baseline peak RSS bytes | Candidate peak RSS bytes |
| --- | ---: | ---: | ---: | ---: |
| token_text_ascii | unavailable | unavailable | unavailable | unavailable |
| token_text_unicode | unavailable | unavailable | unavailable | unavailable |
