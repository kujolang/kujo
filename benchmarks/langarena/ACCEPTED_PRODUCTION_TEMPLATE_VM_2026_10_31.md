# Production template VM verification

Both unchanged production template workloads pass on accepted runtime `fd7b764`
in default VM mode: count 50,000, 10 warmups, 50 measured iterations.
No runtime source, workload algorithm or production configuration changed.

| Case | Verified checksum | Total elapsed | Peak RSS bytes |
| --- | ---: | ---: | ---: |
| Template::Regex | 3554506535 | 136.428 s | 109449216 |
| Template::Parse | 3554506535 | 2089.852 s | 104816640 |

These instrumented single observations on a loaded laptop establish completion
and observed resources, not competitive timing or a controlled speed comparison.
Worktree preparation, formatting, evidence processing and changing user activity
overlap; no local candidate build ran during this sequence. Total elapsed includes
setup, warmup, measurement and checksum computation. CI tests ran remotely.

The independent verifier checks exact case/configuration inventory, pinned input
and executable hashes, exit status, untruncated output, checksum/summary lines,
complete monotonic phase checkpoints, measurement hashes and child RSS.
The VM reports zero native JIT entries, as expected. The evidence bundle
`accepted-production-template-vm-2026-10-31.json.gz` preserves 18
SHA-256-verified artifacts. Accepted executable SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.

Including earlier verified sorting, JSON, CSV, Base64 and JIT template results,
production coverage is now 24/150 runtime/case combinations. The remaining 126,
clean-host comparisons and platform checks remain open. Interpreter templates
continue in the original sequence. The separate loop-call candidate is not
accepted by this baseline verification. No Kostya contact occurred.
