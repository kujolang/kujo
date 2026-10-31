# Struct field-read allocation isolation

On accepted runtime `a1f1858`, inline reads of a local struct field allocate
**248 additional VM bytes per read** when the struct type name grows from 8 to
256 characters. Extracting the field before the loop removes this loop-dependent
penalty. Dictionary controls using documented bracket indexing have no such VM
penalty. No runtime implementation or benchmark source changed in this study.

The verified matrix covers 128 allocation observations and 192 semantic runs:
struct/dictionary receivers, scalar/array fields, inline/extracted reads,
8/256-character type names, 64/1,024 loop iterations and one/eight calls.
All VM/interpreter/JIT semantic checks pass, including retained snapshots.
The independent verifier checks exact membership, hashes, output and counters.

JIT whole-process compiler allocations vary, so an exact JIT slope is not
claimed. Forty-eight additional observations reproduce an inline name-length
penalty of approximately 247.48–248.66 bytes per added read. Extracted controls
range from approximately -6.32 to +0.59 bytes per added read. The original VM
penalty is exactly 248 across both payload types and call counts.

All 64 fixture shapes were also run with the existing runtime measurement
collector. Each records exactly the requested number of native function entries
(one or eight), with successful output. A retained compiler trace shows
`LoadLocal(0)` followed by `FieldGet("value")`, successful compilation of `read`
and retry of its current call with native code. Native-entry counts establish
execution, not CPU-time coverage.

## Attribution and next gate

`Value::Struct` owns its type name as a `String`. The VM's ordinary `LoadLocal`
clones the local value; native operand duplication likewise calls
`NativeOwnedValues::duplicate`, which clones the rooted value. These transient
receiver copies explain the type-name-dependent allocation observed during
repeated field reads. The field name and returned numeric payload stay constant.

The next implementation gate is to avoid the demonstrated transient receiver
copy for eligible local field reads. Preserve independent ownership of returned
collections, caller-owned native roots, captured/global fallbacks, legacy method
receivers, error order and snapshots. Keep the public struct representation and
its value semantics intact. Existing field-access memory-copy profile frames
alone do not prove which ABI transfers survive compiler optimization.

Extraction is an isolation control, not an optimization applied to LangArena's
port. Interpreter allocation traffic, elapsed-speed improvements, retained memory
and competitive performance remain unmeasured here. The previous named-load fix
remains accepted and unchanged.

## Rejected control and evidence

An initial dictionary dot-access control succeeded in VM/JIT and failed with
runtime exit code 4 in the interpreter. The language specification documents
bracket indexing for dictionaries and declared field access for structs; this
experiment does not treat dictionary dot access as a portable contract. The
original script, fixtures, partial results and three-runtime observation are
retained. Dictionary controls were corrected to bracket indexing and the full
matrix rerun. No language behavior was changed to make the control pass.

The [evidence bundle](struct-field-read-baseline-2026-10-31.json.gz) contains raw
observations, fixtures, native-entry receipts, repeats, verifier sources and the
rejected control. Artifact SHA-256 digests are verified after writing. The accepted
CLI and allocator probe identities are bound to the previous fix's build receipt.
No Kostya contact, release or upstream submission occurred.
