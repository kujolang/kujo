# Brainfuck array: skip the matched bracket after a taken jump

The canonical array interpreter now resumes one command after the matched bracket
for both zero-cell `[` and nonzero-cell `]` jumps. The old port dispatched the
counterpart bracket again before entering/exiting the body. Rust and Goose
advance past it. This changes exactly two assignments in `20_text_hash.kujo`;
the generated port is verified against the ordered build fragments and differs
from the reconstructed previous port only at those two sites.

## Verification

- The corrected-jump regression gate rejects the original canonical source.
- Sixteen focused programs pass in VM/interpreter/JIT with complete tape,
  position, checksum and command counts: 96 baseline/corrected state observations.
- Seven taken-jump fixtures improve command counts; nine controls retain them.
  The old variant executes 711 Brainfuck commands versus 666 in the corrected
  variant per fixture pass. This is not a VM-instruction count or elapsed-speed
  result. No benchmark input or checksum is weakened.
- Independent verification checks all state/count rows, runtime exits, hashes,
  source identity and three count-corruption controls that retain tape/position/
  checksum values. Canonical-source mode requires both corrected jump sites.
- All150 test-size checksum cases pass: fifty each in VM, JIT and interpreter.
  Independent verification checks every configuration, exact success summary,
  warmup/run phase sequence, monotonic phase times and frozen identity.
- Production status: the sequential JIT/VM/interpreter Brainfuck::Array sequence
  has started with original `run.json` inputs and one-hour per-case deadlines.
  JIT is running at this capture; these are not yet production passes.

Verification uses the accepted memory runtime's frozen binary
`17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325`.
The generated corrected port hash is
`a3def3c4a52dac9f49dde8d24297c847cbede04837f680bf850ce2e8082dcbfb`.
No runtime implementation changes accompany this port fix. The preceding
isolated audit remains historical in `BF_ARRAY_JUMP_AUDIT_2026_11_03.md`.

## Remaining scope

The structural ledger row stays partial: executed comparator traces, malformed
or comma-input policy, empty/large-growth boundaries and production acceptance
remain distinct. The focused oracle is independent Ruby; comparator jump behavior
is pinned-source evidence, not executed Rust/Goose trace comparison. Local loaded
host durations do not establish a competitive performance gain.

Reproduce focused controls with
`probes/audit-bf-array-jumps.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT --require-correct-jumps`
then `probes/verify-bf-array-jumps.rb NEW_OUTPUT`. The archive
`bf-array-jump-fix-2026-11-03.json.gz` preserves the old-source negative gate,
bounded source diff, focused raw states, all150 raw outcomes/phase verification,
source manifests, collectors and verifiers. Live production evidence remains
separate under `.audit-evidence/native-performance-campaign-2026-11-03/bf-jump-fix/production/`.
Its coordinator records actual process handles; poll them before any restart.
