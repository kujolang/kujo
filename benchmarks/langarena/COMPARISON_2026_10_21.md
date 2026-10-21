# Local comparison: Kujo, Goose and Rust

All **36 checksum-validated samples passed**: four workloads, three languages,
three sequential runs each. These are exploratory measurements on a busy Intel
i7-9750H Mac with 16 GiB RAM and macOS 26.6.2, not official LangArena scores.

Median measured-loop seconds; lower is better. Each time covers the entire
configured iteration batch, not one operation. Preparation, warmup, RNG reset,
checksum and compilation are excluded for all three runners.

| Workload | Input / measured iterations | Kujo VM | Goose | Rust | Kujo / Goose | Kujo / Rust |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Base64 encode — production | 500,000 bytes / 5,000 | 4.978 s | 0.941 s | 0.848 s | 5.3× | 5.9× |
| JSON generation — production | 20,000 coordinates / 100 | 16.170 s | 0.816 s | 0.962 s | 19.8× | 16.8× |
| Log parsing — production | 52,000 lines / 10 | 1.158 s | 0.666 s | 0.939 s | 1.7× | 1.2× |
| QuickSort — **test size** | 100,000 elements / 2 | 17.296 s | 0.027 s | 0.026 s | ~641× | ~665× |

Ratios divide the medians; values above one mean Kujo took longer. They describe
this sample only. QuickSort uses the unchanged upstream `test.json`; the other
three use unchanged `run.json`. No sizes, algorithms, warmup counts, measured
iteration counts or expected checksums were edited.

## What these results establish

Kujo completed these exact workloads correctly. Log parsing was its closest
result. JSON generation was substantially slower, and the Kujo QuickSort port
was hundreds of times slower than the ahead-of-time compiled ports. QuickSort
therefore warrants focused VM/port profiling; this comparison does not establish
which implementation cost causes the gap or prove a semantic defect.

The first three cases exercise substantial native library work. They cannot be
used as a general language-speed ranking. The QuickSort row adds a language-heavy
case, but it is deliberately labeled test size and does not close the outstanding
full production QuickSort gate. Do not aggregate these four rows into an overall
LangArena score.

## Variability and limits

Observed minimum–maximum seconds across the three samples:

| Workload | Kujo VM | Goose | Rust |
| --- | ---: | ---: | ---: |
| Base64 encode | 2.736–5.376 | 0.863–1.305 | 0.613–1.114 |
| JSON generation | 12.260–28.703 | 0.762–1.147 | 0.745–1.143 |
| Log parsing | 0.753–1.263 | 0.648–0.744 | 0.862–2.520 |
| QuickSort, test size | 14.136–23.359 | 0.024–0.029 | 0.021–0.039 |

The host reported CPU speed limits of 51% before and 48% after the run, and load
averages of 38.76 and 42.92 respectively. Other work remained active. No benchmark
samples or our compiler builds overlapped each other, and language order rotated
each round, but neither measure removes external contention or thermal effects.
There are only three samples per cell; these ranges are not confidence intervals.
Rust and Goose print timings rounded to milliseconds, which also limits ratio
precision, especially for QuickSort. Controlled reruns are required before
publishing performance claims. Peak memory was not measured.

## Builds and reproduction

- LangArena ports/configurations: `3c9f84ddaeec834c0631747cb789e8cfd602f993`.
- Kujo: development runtime `34b82c909ebbf7a50c19e9835b562ac700a58959`, normal
  default-feature release build with thin LTO, Rust 1.96.0, default bytecode VM.
  This is not a claim about previously distributed stable release artifacts.
- Goose: `ea1f368c109c083a2161580ff1d74c70200b6cc8`, the version pinned by that
  LangArena checkout. Upstream `prod` configuration: `goose -O2 --standalone`,
  then Apple clang 17.0.0 `cc -O2 -w ... -lm -pthread`.
- Rust: rustc 1.96.0, `cargo build --locked --release`, upstream release profile.
  This is the standard release configuration, not its separate `max-perf` profile.

Build the competitor binaries first. Assemble the normal Kujo port, create an
output directory, then run from the Kujo repository root:

```sh
target/release/kujo run benchmarks/langarena/build.kujo
mkdir -p /tmp/kujo-comparison
target/release/kujo run benchmarks/langarena/compare.kujo --scheduler-no-timeout \
  /absolute/path/to/kujo \
  /absolute/path/to/LangArena/rust/target/release/benchmarks \
  /absolute/path/to/LangArena/goose/target/benchmark \
  /tmp/kujo-comparison
```

The [Kujo-native runner](compare.kujo) instruments a temporary copy of the port
with two timing reads around its measured loop; it leaves the normal assembled
port and benchmark kernels unchanged. It validates each checksum, checkpoints
every sample, and fails on an invalid result. Each child has a ten-minute limit.
An initial pilot exposed a boolean-handling error in the new output parser; the
checksum itself passed. The parser was corrected and the entire measurement set
restarted. That pilot is excluded from these samples.

[Raw samples, medians, hashes and build provenance](comparison-2026-10-21.json)
preserve every accepted result. The [production follow-up](FOLLOWUP_VERIFICATION.md)
still describes the remaining suite-wide gates. No Kostya contact or upstream
submission occurred.
