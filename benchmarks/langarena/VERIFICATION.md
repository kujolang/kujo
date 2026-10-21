# Initial local port verification

This records the initial port gate. Current runtime verification and production
attempts are recorded in [PRODUCTION_VERIFICATION.md](PRODUCTION_VERIFICATION.md).

Upstream pin: `3c9f84ddaeec834c0631747cb789e8cfd602f993`.
The checked-in `test.json` and `run.json` match upstream byte for byte.

**Result: all 50 pinned test checksums passed in both runtimes (100/100).**
The final explicit full-suite regression used the optimized binary on macOS
x86-64 and passed in 375.63 seconds under concurrent compilation; this elapsed
time is validation evidence, not a performance comparison.

```sh
KUJO_LANGARENA_BINARY="$PWD/target/release/kujo" cargo test --test langarena_port_contracts -- --ignored --nocapture
```

The tested release binary includes execution fixes through `fcb5eee`, with
port source through the LZW correction in `59b144b`. Its SHA-256 is
`72d40503c9c5374a2706a99f54936735747cc37ae8f0e7f6d3e2c69bdfa6eaa7`.
The subsequent diagnostic-only type-checker fix in `544fae4` was verified with
the rebuilt debug binary, the complete assembled-source type check, 132 targeted
tests, and all 38 type-checker unit tests. The optimized binary was not rebuilt
again for this diagnostic-only change.

Config SHA-256 values:

- `test.json`: `424371dd25f3fa3f779789c823a05dbe90d2eddbc2a3ca3c5e4319252c572797`
- `run.json`: `8fe7baed41e8eb0a233057c4406446029f7e5d792cf3a3ac6fc7e714964a155f`

## Runtime fixes found by the port

1. Byte iteration was unsupported: the VM failed, while the interpreter emitted
   a diagnostic and continued. Ordinary and generator loops now iterate unsigned
   byte values over a retained snapshot. Mutation detaches the source buffer.
   `byte_iteration_contracts` exercises both runtimes.
2. VM native fallback refreshed and cloned every global binding on each call,
   including unrelated bytecode functions. A sample of Calculator interpretation
   showed 9 of 10 active VM samples in native fallback, with 8 in environment
   refresh. Audited argument-only builtins now use the existing native dispatcher
   without refreshing globals. Callback and host APIs retain the refresh, and
   capability/arity checks remain in the dispatcher. The unit regression checks
   that an unrelated global buffer acquires no extra snapshot owner; dual-runtime
   tests check callback dependencies, aliases, errors, and restricted access.

An isolated pre-fix debug probe loaded all port definitions and performed 500
`wrapping_add` calls: real 18.26s, user 2.87s, system 0.23s. Concurrent compilation
makes wall time unsuitable for a benchmark claim. Post-fix measurements and the
full checksum results are recorded below.

## Port-specific corrections

- Integer bounds helpers preserve integer array indices; Kujo `min`/`max` return
  floating-point values.
- `starts_with` returns a Boolean; JSON generation checks it directly.
- String slicing uses `substring`; collection slicing uses `slice`.
- Byte tapes and sieve flags use byte storage, with explicit wraparound.
- LZW decoding now retains byte sequences instead of UTF-8-encoding dictionary
  characters. The original implementation expanded a six-byte input containing
  128 and 255 into ten bytes in both runtimes. Binary, empty, and repeated-high-byte
  round trips now pass, and the unchanged LZW checksums remain 208 and 100244.

## Scope

This verifies the pinned checksum workloads, not official LangArena integration
or production `run.json` performance. Representation and submission questions
are listed in `README.md`. No Kostya contact or upstream submission occurred.

The same native-call probe after the fix took real 0.15s, user 0.12s, system
0.02s. This supports removal of the measured cloning cost; it is not a language
performance ranking. The ordinary dual-runtime port regressions took 3.88s after
the fix (previous run 78.91s under different concurrent build load).

The port also exposed numeric-index parity gaps. Finite floating-point sequence
reads now truncate consistently in VM and interpreter, including byte reads;
non-finite reads fail. Interpreter simple array writes and dictionary writes now
reject invalid index/key types consistently with VM and nested writes, without
changing aliases or the original container on rejection.

The complete port also exposed two advisory type-check warnings for valid
remainder-based string indices in CSV and log generation. The checker omitted
`%` from numeric arithmetic inference. Its rule now infers integer, floating,
and mixed numeric remainders. Unknown helper-argument types defer to runtime
index validation; known invalid types remain static errors. Regressions execute
these indices in both runtimes and check the entire assembled port for static
diagnostics.

Sorting profiling also found suspended caller bytecode copied on every VM call.
Call frames now retain the caller's owned chunk, restoring it on return or
exception unwind. The unit regression checks the original instruction-storage
addresses survive transfer; dual-runtime regressions cover recursive values,
exceptions, subsequent calls, escaped recursive closures, and generator resume.

## Regression gates

- Byte iteration, byte mutation, generator continuation, and parity: 155 passed.
- Numeric index compatibility and parity: 132 passed.
- Final call-frame, generator, native-environment, numeric-index, port-kernel,
  and parity gate: 158 passed; the full checksum gate is opt-in.
- Ordinary port and README contracts: 4 passed; the full checksum gate is opt-in.
- CLI fixture gates after the call-frame fix: VM 150/150; dual 150/150,
  with 150 VM-primary results and no interpreter fallback. Both report 11
  explicitly skipped fixtures.
- Both storage-ownership unit regressions passed separately.
- Final diagnostic/index/port/parity gate: 132 passed, with the expensive
  checksum test run separately. Type-checker unit suite: 38 passed.
- Optimized full checksum regression: 1 passed, asserting 50 successful
  checksums and zero failures for each runtime. Output values were also
  independently compared against the pinned configuration.
- `cargo fmt --check`, `git diff --check`, and strict TODO inventory generation
  passed. Inventory regeneration only changed source-line references.

The final gate is targeted rather than a fresh full `cargo test`, because the
artifact builds exhausted local disk space and required removal of rebuildable
build caches. It covers the changed runtime paths and repository fixture policy.
The checksum runs separately exercise every pinned benchmark in both runtimes.

## Completed checksum results

Complete debug runs and the final optimized full-suite regression both produced
`Summary: 50, 50, 0` for each runtime. The LZW correction was included in the
optimized regression. Each result below was independently matched to the
unchanged pinned config.

| Benchmark | Expected | VM | Interpreter |
|---|---:|---:|---:|
| Binarytrees::Obj | 124 | 124 | 124 |
| Binarytrees::Arena | 124 | 124 | 124 |
| Brainfuck::Array | 3562897308 | 3562897308 | 3562897308 |
| Brainfuck::Recursion | 3562897308 | 3562897308 | 3562897308 |
| Matmul::Single | 720656960 | 720656960 | 720656960 |
| Matmul::T4 | 2420334760 | 2420334760 | 2420334760 |
| Matmul::T8 | 4014223764 | 4014223764 | 4014223764 |
| Matmul::T16 | 418055336 | 418055336 | 418055336 |
| Base64::Encode | 3894061047 | 3894061047 | 3894061047 |
| Base64::Decode | 732348015 | 732348015 | 732348015 |
| Json::Generate | 4 | 4 | 4 |
| Json::ParseDom | 2918383076 | 2918383076 | 2918383076 |
| Json::ParseMapping | 1969907348 | 1969907348 | 1969907348 |
| CSV::Parse | 3563398641 | 3563398641 | 3563398641 |
| Etc::Sieve | 4660 | 4660 | 4660 |
| Etc::TextRaytracer | 1438660 | 1438660 | 1438660 |
| Etc::NeuralNet | 1586674334 | 1586674334 | 1586674334 |
| Etc::CacheSimulation | 179773224 | 179773224 | 179773224 |
| Etc::GameOfLife | 4006235749 | 4006235749 | 4006235749 |
| Etc::Words | 580539813 | 580539813 | 580539813 |
| Etc::LogParser | 117 | 117 | 117 |
| Template::Regex | 1554169905 | 1554169905 | 1554169905 |
| Template::Parse | 917360148 | 917360148 | 917360148 |
| Sort::Quick | 2548773 | 2548773 | 2548773 |
| Sort::Merge | 2548773 | 2548773 | 2548773 |
| Sort::Self | 2548773 | 2548773 | 2548773 |
| Graph::BFS | 20 | 20 | 20 |
| Graph::DFS | 28 | 28 | 28 |
| Graph::AStar | 36 | 36 | 36 |
| Hash::SHA256 | 1036619072 | 1036619072 | 1036619072 |
| Hash::CRC32 | 3031698216 | 3031698216 | 3031698216 |
| Calculator::Ast | 774028720 | 774028720 | 774028720 |
| Calculator::Interpreter | 3208680 | 3208680 | 3208680 |
| Maze::Generator | 2309652669 | 2309652669 | 2309652669 |
| Maze::BFS | 6771 | 6771 | 6771 |
| Maze::AStar | 6771 | 6771 | 6771 |
| CLBG::Fannkuchredux | 4428 | 4428 | 4428 |
| CLBG::Mandelbrot | 2236342953 | 2236342953 | 2236342953 |
| CLBG::Nbody | 338796608 | 338796608 | 338796608 |
| CLBG::Spectralnorm | 1052168784 | 1052168784 | 1052168784 |
| Compress::BWTEncode | 200 | 200 | 200 |
| Compress::BWTDecode | 100204 | 100204 | 100204 |
| Compress::HuffEncode | 56 | 56 | 56 |
| Compress::HuffDecode | 100212 | 100212 | 100212 |
| Compress::ArithEncode | 56 | 56 | 56 |
| Compress::ArithDecode | 100220 | 100220 | 100220 |
| Compress::LZWEncode | 208 | 208 | 208 |
| Compress::LZWDecode | 100244 | 100244 | 100244 |
| Distance::Jaro | 7020 | 7020 | 7020 |
| Distance::NGram | 42 | 42 | 42 |
