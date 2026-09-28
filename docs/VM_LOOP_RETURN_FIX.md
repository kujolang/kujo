# VM loop / early-return correctness fix

Baseline source: `646fc9b94abec1071e93ea54648fa75f56c0c55b`.
Regression-only commit: `2fe6ed306e0ade589b0ac39013c24f1de2d4c5db` (four of the initial five contract tests failed).
Causal fix candidate: `db4608c1df59668ff9c89956ae4ef1ab18e86f14`.
No version bump, language change, assurance feature or participant API change.

## Baseline and minimization

The retained `docs/evidence/kujo-1.6-review/runtime-return-probe.kujo.txt` fails
under the default VM (exit 4, `Stack underflow in return`) and succeeds under the
interpreter (exit 0, `{"ok":true}`). The isolated fixture runner reports VM 0/1
(exit 3), interpreter 1/1 and dual 1/1 **with interpreter fallback**. A green dual
baseline alone would conceal the defect. Exact binary hash, commands, stdout,
stderr and per-case outcomes are retained in `docs/evidence/vm-loop-return-fix/`.
No production source was changed before recording the original VM/interpreter
failure. The unchanged baseline binary was retained for the dual comparison.

| Minimized case | Before fix: VM versus interpreter |
|---|---|
| A: no return in loop | Both print 1, 2 |
| B: unconditional return, then empty input | VM prints only 1; interpreter prints 1, 9 |
| C: first-iteration conditional match | Both print 1 |
| D: no matching item, normal completion | VM prints nothing; interpreter prints 9 |
| E: first mismatch, later match | Both print 2 |
| F: nested condition and empty input | VM prints only 2; interpreter prints 2, 9 |
| G: nested loop and empty input | VM prints only 4; interpreter prints 4, 9 |
| H: caller adds returned values | VM prints nothing; interpreter prints 11 |
| I: folded return expression | VM fails; interpreter prints 5, 9 |
| J: break/continue and normal completion | Both print 2 |

The silent omissions matter: the defect was not limited to a diagnostic or a
particular dictionary operation. Exhaustion could jump beyond the function's
instruction stream and lose the caller's output.

## Causal trace and stack invariant

All minimized sources parse without diagnostics. Compiler lowering with
optimization disabled executes successfully. `ForNext` consumes iterable and
index from the value stack. For an item, it pushes the item and falls through to
loop-variable storage; on exhaustion it jumps without pushing an item. Iteration
state lives in compiler-selected locals, not a residual return value on the stack.

`Return` requires a value on the stack, restores the caller and truncates its
frame's stack region. Compiler return lowering evaluates the value and unwinds
runtime scopes before emitting `Return`. Those operations are unchanged.

For the historical function, pre-optimization instruction 16 is `ForNext(42)`;
instruction 42 loads the true constant, 43 builds the result dictionary, and 44
returns it. Dead-code elimination removes two unreachable instructions following
the conditional return (and the trailing implicit return). Its remapping handled
ordinary jumps but **omitted ForNext**, leaving instruction 16 as `ForNext(42)`
even though the value setup moved to 40/41 and `Return` moved to 42. Exhaustion
therefore entered `Return` directly with no value: the VM correctly rejected it.
The fixed bytecode uses `ForNext(40)` and reaches the unchanged value setup.

Reachability also omitted the exhaustion successor. Unconditional return in the
loop could make the post-loop return seem unreachable and delete it entirely.
The old `JumpBack` scan incorrectly treated it as having fall-through, sometimes
masking that missing edge. Both successors of ForNext are now explicit;
unconditional Jump/JumpBack terminate the current reachability path.

The local audit found the same address-preservation class in constant folding
and peephole deletion: those passes changed instruction indices without relocating
branch operands at all. The folded-expression seed and direct bytecode regressions
prove why fixing only the ForNext enum arm would be incomplete.

## Fix and opcode audit

Only production file `src/optimizer.rs` changes. Every instruction-changing pass
now installs its output through a shared boundary relocation routine. It relocates
Jump, JumpIfFalse, JumpIfTrue, JumpBack, ForNext and BeginTry; handler ranges/catch
entries and source locations follow the same map. The end boundary is included
for exclusive ranges. Removed source locations do not overwrite surviving ones.

Constant folding and two-instruction peephole rewrites do not cross an alternate
entry or exception-range boundary. This preserves the stack contract of a jump
into the middle of a candidate sequence. Optimization remains enabled.

Break/Continue lower to Jump; there are no separate bytecode targets to omit.
Return/ReturnNone/Throw stop normal reachability. Yield and Await retain the
following instruction as their continuation and carry no instruction-address
operand. MatchCasePattern carries a pattern string, not a jump address. Existing
compiler guards for specialized match/exception/logical/method flows are unchanged.
No compiler lowering, iterator cleanup, capture/upvalue layout, generator/task
implementation, interpreter behavior or VM stack check was modified.

## Permanent coverage and correctness review

`tests/loop_return_contracts.rs` checks exact VM/interpreter output for all eleven
seeds (including the historical one), pre/post compiler bytecode, exhaustion
reachability, folding/peephole relocation, alternate entry points, all six address
operands plus exception/source metadata, and malformed Return stack rejection.
`tests/test_loop_early_return.kujo` aggregates the seeds for VM/interpreter/dual
fixture sweeps without replacing any existing fixture. The parser fuzz corpus
also retains a minimized loop/conditional/return seed; no runtime fuzzing system
or unrun fuzz campaign is claimed.

The change restores supported semantics. Live targets cannot be removed as dead
because every actual normal branch successor is traced; exception entries remain
conservatively reachable. Sequence rewrites reject interior entry points. Stack
underflow for genuinely empty Return still errors. Direct malformed-bytecode
validation is not broadened or weakened. No interpreter fallback or source-level
boolean workaround is used by these regressions.

## Validation and downstream reconciliation

Kujo canonical gates passed on the causal fix: formatting, check, full Rust tests
(2,856 passed; 17 existing ignored), VM/interpreter/dual fixture sweeps (150/150
each, 11 existing skips, zero dual fallback), and 232 targeted contracts. The
aggregate loop fixture increases each sweep from 149 to 150. Targeted coverage
includes closures/upvalues, generator continuations, async/task and loop parity,
README/docs, CLI/JSON and diagnostic contracts. No hosted CI is claimed.

The SDK onboarding is not rerun: its API/contracts are unchanged; the existing
automated real adopter integration is the supplemental downstream check. Human
usability remains unperformed and post-release validation.

The exact optimized candidate was built with `cargo build --release --locked`
from `db4608c1df59668ff9c89956ae4ef1ab18e86f14` using Rust/Cargo 1.96.0 on
Darwin x86_64. Binary SHA-256:
`74f42acdf3470ea73cd770f3ac2eae281c1bfe3609c3db5e2cfa8c197e523685`.
All eleven retained probes pass in both runtimes on that binary (22 executions,
identical stdout). The original historical source bytes are unchanged.

The downstream run selects that copied binary through `KUJO_BIN`; it does not
reuse the older ecosystem runtime pin. Exact repository revisions are in
[`repositories.json`](evidence/vm-loop-return-fix/repositories.json), toolchain and
lockfile digest in [`candidate.json`](evidence/vm-loop-return-fix/candidate.json),
and probe commands/stdout/stderr/exits in
[`release-probes.json`](evidence/vm-loop-return-fix/release-probes.json).

The full Dispatch release gate passed at
`f5bbca46098f542ecee5e193c78fa8008706454d`: all 45 focused suites, all 24 shards
(101 source tests), command smoke and 3/3 bounded workloads. This includes
persisted negotiation, SQLite/Git/Ability verification, beta migration, and the
Agents SDK/MCP/HTTP/Git/TypeScript/Python paths through generic correlation.

The separate Workcell effect-assurance gate passed its atomic ref/CAS, duplicate,
conflict, expiry, moved-target and privacy checks, plus four-process contention
(one admitted, three denied, one logical effect). The existing automated adopter
integration also passed using the installed package: real pre/post-CAS SIGKILL,
unknown participant knowledge with live not_started/committed state respectively,
fresh controllers retaining required policy, one logical effect, 12 input denials
and one-admitted/three-denied contention. No new source-blind onboarding or human
pilot is claimed.

[`results.json`](evidence/vm-loop-return-fix/results.json) records commands, exits,
source/toolchain/artifact pins and raw/compressed log hashes. The compressed
`dispatch-suites.json.gz` retains every individual suite log and its exact-byte
SHA-256. Build warnings from the existing tiny_http dependency remain warnings;
no gate failure was suppressed.

## Decision and commit map

**VM_BLOCKER_FIXED — READY_FOR_1_6_RELEASE_PREP.** No valid-program discrepancy
remains in the original/minimized cases or the canonical/runtime/downstream gates.
This is bounded local correctness evidence, not a claim to have proved all programs
or all release platforms. Existing stack safety checks remain in force.

| Repository / commit | Purpose |
|---|---|
| Kujo `2fe6ed306e0ade589b0ac39013c24f1de2d4c5db` | Reproduction, failing contracts, minimized fixtures and fuzz seed |
| Kujo `db4608c1df59668ff9c89956ae4ef1ab18e86f14` | Causal optimizer fix, complete address/metadata tests, fixture expansion and changelog |
| Kujo, documentation commit containing this report | Immutable evidence and readiness reconciliation |
| Dispatch / Workcell / Ability / MCP / Agents SDK | Unmodified; exact tested revisions retained in repositories.json |

Next task: **Kujo 1.6 release-source/version/changelog/artifact reconciliation**.
Reconcile the eventual release candidate and downstream pins, consolidate release
notes, perform the version bump and full candidate/platform/artifact checks, and
prepare reviewable tags/checksums. No version, release or package publication was
performed here. Wave C beta and Wave D alpha remain experimental and opt-in.
