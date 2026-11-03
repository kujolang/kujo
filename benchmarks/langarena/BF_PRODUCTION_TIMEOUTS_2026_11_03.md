# Brainfuck Array: corrected-port production outcomes

The frozen corrected-jump production sequence has terminated. All three modes
timed out at their unchanged one-hour deadline; none completed a checksum.

| Mode | Elapsed seconds | Warmup completed by seconds | Outcome |
| --- | ---: | ---: | --- |
| JIT | 3600.016147744 | 42.216477256 | Timeout; no completed production iteration |
| VM | 3600.011419995 | 79.780553836 | Timeout; no completed production iteration |
| Interpreter | 3600.012703090 | 63.906583818 | Timeout; no completed production iteration |

Warmup timestamps are from process start and include setup. These sequential
observations on a loaded laptop are not paired speed comparisons. They do not
justify a performance ranking, an exact completion estimate or dismissal of the
failure as solely host contention.

The verifier rehashes all 28 frozen source/input/executable identities, requires
the exact production configuration (1,000 warmups and one production iteration),
matches raw stdout/stderr against each report, checks driver/child outcomes and
deadlines, and confirms there is no completed iteration/checksum/summary. The
JIT/VM measurement reports remain explicitly unavailable after termination;
there is no final counter report from which to infer native coverage or CPU
attribution. Both coordinator and final child were confirmed absent after the
terminal state was recorded at `2026-11-03T14:09:26Z`.

The executable is accepted frozen SHA
`17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325`, not the
worker-reuse candidate. The tested main-worktree port and all manifest inputs
were unchanged throughout collection and rehashing. The separate calculator
source correction was made afterward in the candidate worktree.

`bf-production-timeouts-2026-11-03.json.gz` preserves raw reports, output files,
collector state, manifest, verification receipt and verifier, with checked
content hashes. Binary bytes remain in the frozen local evidence rather than
being duplicated into Git. These results close collection of this sequence;
they leave production acceptance failed/incomplete. Test-size checksums and
bounded state/operation-count controls do not substitute for production success.
