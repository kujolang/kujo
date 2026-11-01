# Template parser loop admission gap

Accepted runtime `fd7b764` reproduces a JIT compilation failure on the existing
test-size `Template::Parse` workload and a small nested string scanner. Both
return correct results through fallback. This is an optimization gap, not a
checksum failure, and does not establish the cause of the whole production cost.

`probes/nested_string_scan.kujo` returns `nested scan: OK 200` in VM, interpreter
and JIT. With `DEBUG_JIT=1`, JIT reports `Inconsistent JIT stack depth` while
compiling its loops. The local nested function deliberately prevents whole-function
compilation, exposing the loop compilation path used by the template callback.

The unchanged test-size template case returns checksum `917360148`. Its JIT log
records failed loop compilation at offsets 38 and 83, with excess operand-stack
depths of four and two respectively. Other functions compile successfully;
aggregate native-entry counters alone would not prove these hot loops run natively.

Source review finds that `guard_loop_callback` terminates native translation at
calls and preserves operands for VM resumption. Loop translation subsequently
visits other blocks. The diagnostic shows inconsistent stack depths on those
later edges. This identifies a control-flow investigation target; the exact repair
and its safety are not yet established. Do not remove callback guards or weaken
stack validation to make compilation appear successful.

The follow-up must preserve callback effects, operand order, local updates,
early exits, errors and cancellation, and verify actual loop execution rather
than only a passing checksum. Runtime source remains unchanged while the original
full-production template sequence runs.

The diagnostic CLI runs took under one second in total and overlapped production
parser warmup. A separate three-second CPU sample also occurred during warmup;
the production timing report must disclose both. Neither diagnostic is a clean
speed comparison. The sample cannot reliably resolve Kujo functions in the stripped
executable and is not used for causal attribution here.

`template-loop-admission-2026-10-31.json.gz` preserves ten digest-verified artifacts:
the reproducer, three runtime outputs and diagnostics, verification receipt, and
the original template admission output/diagnostics. Source/input hashes and the
accepted executable identity are included. No runtime fix or production pass is
claimed from this evidence.
