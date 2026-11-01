# Production CSV verification across runtimes

CSV::Parse passes unchanged production checksums in VM, interpreter and JIT on
accepted runtime `fd7b764`. Each run uses 100,000 rows, 14 warmup iterations and
70 measured iterations. No runtime code, algorithm or input changed.

| Case | Runtime | Verified checksum | Total elapsed | Peak RSS bytes |
| --- | --- | ---: | ---: | ---: |
| CSV::Parse | jit | 1071523081 | 166.498 s | 284102656 |
| CSV::Parse | vm | 1071523081 | 202.936 s | 343789568 |
| CSV::Parse | interpreter | 1071523081 | 163.926 s | 325730304 |

Single observations on the loaded local machine establish completion and observed
resources, not controlled runtime comparisons or speedups. Each child ran alone
within this verification sequence. Total elapsed includes setup and warmup.

Independent verification checks exact case/configuration inventories, source and
accepted executable hashes, exits, untruncated output, checksum and summary lines,
complete monotonic phase checkpoints and child peak RSS. JIT measurements show
native entries, without implying native CPU coverage. VM measurements also
pass schema/hash checks and report zero native JIT entries. Interpreter execution
uses `--interpreter`; VM measurement instrumentation is unsupported there and was
not requested. Runner commands and reports preserve that distinction.

The bundle `accepted-production-csv-parity-2026-10-31.json.gz` contains
41 digest-verified artifacts, including raw results, runners, verifiers,
measurements where supported and host metadata. Accepted executable SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.

With the earlier sorting and JSON passes, current production coverage is 14/150 runtime/case
combinations. The remaining 136, platform verification and clean-host comparison
remain open. No Kostya contact occurred.
