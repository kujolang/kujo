# Kujo 1.8 shared-analysis latency receipt

Candidate source: `bae94d204707749a20eb1de092d124a96039d287`

This is a bounded editor-latency guardrail, not a cross-version marketing
benchmark. The baseline invokes the standalone completion entry point, which
builds a fresh analyzed program for every request. The candidate invokes the
same completion logic against the immutable snapshot retained by an open LSP
document.

## Environment

- Platform: macOS 26.6.2 (25G83), x86_64
- CPU: Intel Core i7-9750H at 2.60 GHz
- Rust: 1.96.0 (`ac68faa20`, LLVM 22.1.2)
- Build mode: Cargo `test` / debug, unoptimized with debuginfo
- Workload: 1,209-line Kujo source with a function, loop, call and 1,200 bindings
- Repetitions: five process batches; 10 operations per full analysis and
  uncached completion batch, 20 per cached completion/diagnostic/hover batch
- Warmup: zero explicit warmup iterations; each batch constructs the retained
  analyzed program before measuring cached requests

The release-profile attempt was stopped because Kujo's full PDF/image dependency
graph reduced host free space below 2 GiB before reaching the benchmark. No
release-mode number is reported or inferred.

## Results

Values are the median of five batch means; the final column is the observed
minimum-to-maximum batch range.

| Operation | Baseline median | Candidate median | Change | Observed range |
| --- | ---: | ---: | ---: | ---: |
| Repeated completion | 27.168 ms | 7.297 ms | -73.1% | baseline 25.185–33.979 ms; candidate 6.706–8.172 ms |
| Cached diagnostics | n/a | 0.227 ms | request reuses snapshot | 0.212–0.258 ms |
| Cached hover | n/a | 4.998 ms | request reuses snapshot | 4.710–5.423 ms |
| Full-file analysis | n/a | 20.572 ms | new content revision | 18.811–25.613 ms |

The retained optimization is architectural: open-document analysis count drops
from one parse/check per diagnostics, completion or hover request to one per
content revision. Imported-module parsing is additionally reused across analyses;
unit coverage verifies a cache hit for unchanged content and a miss after a
same-length content change.

## Reproduction

```bash
cargo test --test lsp_latency_guardrails -- --nocapture
```

Run the command five times on an otherwise idle host. The exact captured output
is retained in [`raw.txt`](raw.txt).
