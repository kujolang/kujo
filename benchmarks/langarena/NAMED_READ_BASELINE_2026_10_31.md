# Named-read allocation baseline

The symbolized VM profiles identified opcode cloning as a remaining cost. On
accepted runtime `5a0632d`, compiler-emitted global reads inside functions use
`LoadVar`; the corresponding local reads use `LoadLocal(1)`.

The verified matrix contains 32 allocation observations and 48 semantic runs:
global/local bindings, identifier lengths 8/256, loop counts 64/1,024, call
counts 1/8, VM/JIT allocation observations and VM/interpreter/JIT semantics.
After isolating both loop count and name length, the VM allocates **248 extra
bytes per global read** for a 256-character name versus an 8-character name.
The local control has zero identifier-length slope. This supports removing the
owned opcode-name copy on the measured VM dispatch path. It does not establish
that all name-related allocation can disappear: a per-call component remains
separate from the per-read component.

JIT observations include variable compiler allocation traffic; this report makes
no exact JIT per-read slope or elapsed-time claim. Loaded-laptop timing is not a
competitive benchmark. An initial 1,024-character identifier fixture was rejected
by the existing 256-character lexer limit; it was replaced and the valid matrix
rerun. The rejected attempt is retained separately from accepted observations.

Twelve additional semantic baseline cases pass on the accepted CLI: short/long
names, snapshots, closures, callbacks, lazy builtins and missing global/function
names across all three runtimes. Missing names exit with code 4. These checks
establish existing behavior; they do not accept a candidate implementation.

Raw observations, fixtures, bytecode output, verifier and rejected attempt are
in [the evidence bundle](named-read-baseline-2026-10-31.json.gz). Every bundled
artifact has a SHA-256 digest; the bundle is decoded and checked after writing.
The allocation verifier independently checks matrix membership, stdout, counters,
artifact identities and the length-by-iteration slope. The original CLI SHA-256
is `6fa8f819d3bf023bd586996546e2d51735515a4ee846b22c0be2e52b22b8b92c`.

The borrowed named-load candidate is still under verification. No runtime fix,
all-50 candidate pass, release or Goose/Rust speedup is claimed by this report.
The completed sections 3/4 review cutoff remains distinct from this follow-up.
