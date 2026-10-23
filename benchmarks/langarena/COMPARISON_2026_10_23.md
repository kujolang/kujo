# Local comparison rerun: October 23, 2026

All **36/36 checksum-validated samples passed**. The same binaries, runner, port,
configurations and workloads from [October 21](COMPARISON_2026_10_21.md) were
reused; SHA-256 identities were verified. No runtime or benchmark changes were made.
This isolates changes in observed host conditions from code changes, but does not
control external load or establish causality. This tests the previous development
runtime, not the current 1.9.0 release.

Median measured-loop seconds (lower is better). Positive reduction means Kujo
spent less time than in the previous run.

| Workload | Previous Kujo | New Kujo | Goose | Rust | Kujo time reduction |
| --- | ---: | ---: | ---: | ---: | ---: |
| Base64::Encode | 4.978 | 3.166 | 0.917 | 0.593 | +36.4% |
| Json::Generate | 16.170 | 14.035 | 0.732 | 0.820 | +13.2% |
| Etc::LogParser | 1.158 | 0.872 | 0.592 | 0.788 | +24.7% |
| Sort::Quick | 17.296 | 12.155 | 0.025 | 0.023 | +29.7% |

Base64, JSON generation and log parsing use unchanged production sizes. QuickSort
uses **test size**, not production. Each cell has three sequential samples;
language order rotates each round. Setup, warmup and checksum time are excluded.
Rust/Goose report milliseconds, limiting ratio precision for short samples.

Observed minimum–maximum seconds:

| Workload | Kujo | Goose | Rust |
| --- | ---: | ---: | ---: |
| Base64::Encode | 3.042–3.493 | 0.902–0.958 | 0.552–0.658 |
| Json::Generate | 13.633–15.158 | 0.681–0.759 | 0.758–1.023 |
| Etc::LogParser | 0.816–0.887 | 0.577–0.973 | 0.787–0.863 |
| Sort::Quick | 12.117–14.918 | 0.024–0.026 | 0.022–0.024 |

## Host conditions and interpretation

Before:

```text
Note: No thermal warning level has been recorded
Note: No performance warning level has been recorded
2026-10-23 08:19:34 -0400 CPU Power notify
	CPU_Scheduler_Limit 	= 100
	CPU_Available_CPUs 	= 12
	CPU_Speed_Limit 	= 48
 8:19  up 2 days, 12:39, 2 users, load averages: 10.52 22.14 17.32
```

After:

```text
Note: No thermal warning level has been recorded
Note: No performance warning level has been recorded
2026-10-23 08:22:34 -0400 CPU Power notify
	CPU_Scheduler_Limit 	= 100
	CPU_Available_CPUs 	= 12
	CPU_Speed_Limit 	= 44
 8:22  up 2 days, 12:42, 2 users, load averages: 12.20 16.99 16.02
```

The previous run reported load averages of 38.76–42.92 and CPU speed limits of
51%–48%. Lower load alone does not imply an unthrottled CPU. These samples remain
exploratory, not official scores or proof that laptop load explains the runtime
performance gap. Peak memory was not measured. This four-case rerun does not close
the remaining full-production checksum gates or Linux verification.

[Raw samples, summaries and verified hashes](comparison-2026-10-23.json).
Use the existing [comparison runner](compare.kujo) and the reproduction command in
the previous report, supplying a fresh output directory. No Kostya contact occurred.
