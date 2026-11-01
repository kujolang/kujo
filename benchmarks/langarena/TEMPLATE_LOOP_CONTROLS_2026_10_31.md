# Loop-condition call admission controls

Six small source programs isolate the previously reproduced template parser
admission failure on accepted runtime `fd7b764`. All 18 executions return the
expected result across VM, interpreter and JIT. No runtime change is made.

| Control | JIT stack-depth failure | Compiled-loop entry observed |
| --- | --- | --- |
| Simple loop, `len(text)` condition | Yes | No |
| Simple loop, precomputed bound | No | Yes |
| Nested scan, `len(text)` conditions | Yes | No |
| Nested scan, precomputed bound | No | Yes |
| Simple loop, `len(text)` in body | No | Yes |
| Effectful function in condition | Yes | No |

`DISABLE_FUNCTION_JIT=1` deliberately isolates loop compilation; the controls do
not depend on a nested function blocking whole-function compilation. `DEBUG_JIT`
and `DEBUG_JIT_LOOP` provide admission diagnostics. A compiled-loop entry does
not establish how much of the loop executes natively, because callback guards
can return control to the VM.

The effectful condition checks exactly 301 calls for 300 iterations, and all
runtimes preserve that count. Hoisting condition calls is not a general fix.
Precomputed bounds are diagnostic controls only; the LangArena port is unchanged.
The call-in-body control shows that this is not blanket rejection of every loop
containing a call. These cases narrow the control-flow investigation; they do not
prove a repair or quantify production speedup.

Source fixtures are in `probes/loop_call_admission/`. Independent verification
checks the exact six-by-three inventory, executable/source/output hashes, argv,
environment, exits, expected outputs and admission signatures. The bundle
`template-loop-controls-2026-10-31.json.gz` preserves 46 digest-verified artifacts,
including runners and verifier. All checks pass against the expected baseline
behavior; the three admission failures remain unresolved.

These short diagnostics overlapped the ongoing production parser run and are
not timing comparisons. The original production session remains active, and
accepted runtime source is unchanged. Future work must preserve callback effects,
operand order, locals, early exits, errors and cancellation while handling loop
control flow correctly. Existing loop-callback contract tests remain required.
