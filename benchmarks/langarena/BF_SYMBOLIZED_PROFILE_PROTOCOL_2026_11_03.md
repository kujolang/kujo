# Symbolized production profiling protocol

Use the registered vm-code-lifetime workflow with profile_only=true. This routes
exclusively to bf-profile-gate on macOS Intel; checksum, elapsed and lifetime gates
remain separate. The release build retains symbols and line tables (strip=none,
debug=1), preserving the ordinary release optimization/LTO settings. Build records,
source commit, toolchain, symbols, executable bytes and SHA-256 are uploaded.

The collector extracts canonical common helpers and bf_commands/bf_array, with
unchanged production/warmup programs from run.json. An independent bounded Ruby
byte interpreter computes the warmup checksum (1781448654). All 1000 warmup calls
must match before the ready marker. Only then does /usr/bin/sample attach for ten
seconds at five-millisecond intervals. VM, interpreter and JIT run sequentially.
Each process begins the exact full production program once; the collector stops
its owned process after sampling if still running. A completed production program
asserts its expected checksum, but an interrupted profile is never a checksum pass.
The warmup deadline is 300 seconds; errors and stripped-symbol profiles fail closed.

Retained profiles must contain a named Kujo VM/interpreter/JIT frame. This does not
symbolize dynamically generated Cranelift regions or prove their coverage; review
raw stacks and separate entry evidence before attribution. Sampling under a
symbol-preserving build is diagnostic, not comparable elapsed-speed evidence.

Local checks: Ruby syntax, YAML parsing, all eight dispatch combinations have
exclusive routing, independent warmup oracle matches the prior 120-call checksum,
and reduced one-call fixture smoke succeeds in VM/interpreter/JIT. That smoke
replaces production with the warmup program solely to check generated syntax and
markers; CI uses the unchanged generated production fixture and full warmup.
Historical all150 artifact verification and cargo fmt --check also pass.
No runtime code changed. CI collection and independent artifact review remain open.
