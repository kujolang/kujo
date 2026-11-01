# Production template verification across runtimes

Template::Regex and Template::Parse pass unchanged production checksums in VM,
interpreter and JIT on accepted runtime `fd7b764`. Each uses count 50,000,
10 warmups and 50 measured iterations. No runtime code or workload changed.

| Case | Runtime | Verified checksum | Total elapsed | Peak RSS bytes |
| --- | --- | ---: | ---: | ---: |
| Template::Regex | jit | 3554506535 | 148.641 s | 118747136 |
| Template::Parse | jit | 3554506535 | 2127.594 s | 116584448 |
| Template::Regex | vm | 3554506535 | 136.428 s | 109449216 |
| Template::Parse | vm | 3554506535 | 2089.852 s | 104816640 |
| Template::Regex | interpreter | 3554506535 | 187.235 s | 106147840 |
| Template::Parse | interpreter | 3554506535 | 1528.182 s | 95395840 |

Single observations on the loaded local machine establish completion and observed
resources, not controlled runtime comparisons or speedups. A short CPU sample
occurred during JIT parser warmup. Admission diagnostics, worktree preparation,
formatting and evidence processing overlapped the sequence. Repository inventory
fixture verification also overlapped the interpreter sequence. No local candidate
build ran during these measurements. Each child ran alone within this verification
sequence. Total elapsed includes startup, setup, warmup and checksum computation.

Independent verification checks exact case/configuration inventories, source and
accepted executable hashes, exits, untruncated output, checksum and summary lines,
complete monotonic phase checkpoints and child peak RSS. JIT measurements show
native entries, without implying native CPU coverage. VM measurements also
pass schema/hash checks and report zero native JIT entries. Interpreter execution
uses `--interpreter`; VM measurement instrumentation is unsupported there and was
not requested. Runner commands and reports preserve that distinction.

The bundle `accepted-production-template-parity-2026-10-31.json.gz` contains
66 digest-verified artifacts, including raw results, runners, verifiers,
measurements where supported and host metadata. Accepted executable SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.

With the earlier sorting, JSON, CSV and Base64 passes, current production coverage
is 26/150 runtime/case combinations. The remaining 124, platform verification and clean-host comparison
remain open. No Kostya contact occurred.
