# Production JSON verification across runtimes

The three JSON workloads now pass unchanged production checksums in VM,
interpreter and JIT on accepted runtime `fd7b764`. This report adds VM and
interpreter evidence to [the JIT verification](ACCEPTED_PRODUCTION_JSON_2026_10_31.md).
No runtime code or workload changed. Generate uses 20,000 coordinates, ParseDom
10,000 and ParseMapping 25,000; each uses 20 warmups and 100 measured iterations.

| Case | Runtime | Verified checksum | Total elapsed | Peak RSS bytes |
| --- | --- | ---: | ---: | ---: |
| Json::Generate | vm | 120 | 7.327 s | 55783424 |
| Json::ParseDom | vm | 3989922952 | 18.897 s | 118837248 |
| Json::ParseMapping | vm | 3826684672 | 68.122 s | 198004736 |
| Json::Generate | interpreter | 120 | 6.827 s | 87089152 |
| Json::ParseDom | interpreter | 3989922952 | 17.164 s | 122822656 |
| Json::ParseMapping | interpreter | 3826684672 | 57.191 s | 195899392 |

Single observations on the loaded local machine establish completion and observed
resources, not controlled runtime comparisons or speedups. Each child ran alone
within this verification sequence. Total elapsed includes setup and warmup.

Independent verification checks exact case/configuration inventories, source and
accepted executable hashes, exits, untruncated output, checksum and summary lines,
complete monotonic phase checkpoints and child peak RSS. VM measurements also
pass schema/hash checks and report zero native JIT entries. Interpreter execution
uses `--interpreter`; VM measurement instrumentation is unsupported there and was
not requested. Runner commands and reports preserve that distinction.

The bundle `accepted-production-json-parity-2026-10-31.json.gz` contains
45 digest-verified artifacts, including raw results, runners, verifiers,
measurements where supported and host metadata. Accepted executable SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.

With the two sorting JIT passes, current production coverage is 11/150 runtime/case
combinations. The remaining 139, platform verification and clean-host comparison
remain open. No Kostya contact occurred.
