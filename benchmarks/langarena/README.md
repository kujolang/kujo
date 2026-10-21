# Local LangArena port

Kujo implementations of the 50 benchmarks in `kostya/LangArena` at
`3c9f84ddaeec834c0631747cb789e8cfd602f993`. This is a local development port,
not an accepted LangArena submission. Upstream code is MIT licensed; see
[LICENSE.upstream](LICENSE.upstream).
Recorded results and validation limits are in [VERIFICATION.md](VERIFICATION.md).

`test.json` and `run.json` are unchanged copies of upstream `test.js` and
`run.js`. Expected checksums are used only by the runner, never by benchmark
implementations. Source fragments are assembled into one Kujo program so the
VM executes compiled local functions rather than imported interpreter functions.

From the repository root:

```sh
cargo build --release
target/release/kujo run benchmarks/langarena/build.kujo
target/release/kujo run benchmarks/langarena/port.kujo --scheduler-timeout-ms 3600000 benchmarks/langarena/test.json
target/release/kujo run benchmarks/langarena/port.kujo --interpreter --scheduler-timeout-ms 3600000 benchmarks/langarena/test.json
```

Append an exact benchmark name to select one case. An unknown name or missing
implementation fails. The runner constructs, resets RNG, prepares, warms up,
resets RNG again, runs the configured iterations, and checks the final unsigned
32-bit checksum. Warmup state is retained. Brainfuck uses its separate warmup
program without adding the warmup result. No iteration counts are reduced.

Reusable regression gates:

```sh
cargo test --test byte_iteration_contracts --test native_environment_contracts --test langarena_port_contracts
KUJO_LANGARENA_BINARY="$PWD/target/release/kujo" cargo test --test langarena_port_contracts -- --ignored --nocapture
```

The full checksum test is explicit because it is a long-running workload. The
ordinary tests check the exact 50-name registry and small algorithm, binary
roundtrip, AST, and value-semantics cases in both runtimes.

## Representation choices requiring review before submission

- Object trees and calculator ASTs use distinct recursive structs; the arena
  tree uses indexed nodes. Matrix storage uses nested rows and fresh outputs.
- Threaded matrix variants transpose before dispatch and submit 4, 8, or 16
  row-block jobs to Kujo's bounded `parallel_map` scheduler. Kujo's scheduler
  lifecycle differs from Crystal's benchmark-owned persistent worker pool.
- Neural-network synapses and linked maze/Game of Life cells use indices for
  shared graph identity. Cache nodes retain real string keys and a doubly linked
  LRU order. These choices preserve Kujo value semantics.
- `Json::ParseMapping` currently parses a DOM and explicitly constructs typed
  coordinate structs. Kujo does not have Crystal's direct typed JSON decoder;
  checksum equivalence alone does not settle benchmark comparability.
- `Hash::SHA256` ports upstream's custom toy hash, not standard SHA-256.
- Graph and maze priority queues preserve upstream's exact heap behavior,
  including its pop ordering, because changing it changes traversal behavior.
- A passing `test.json` run proves those exact checksum workloads only. It does
  not establish production `run.json` performance, memory limits, or acceptance.
- This runner checks correctness; upstream timing/start protocol, language
  registration, Linux/Docker packaging, and official driver integration remain
  submission work.

No maintainer contact or upstream submission is part of this local work.
