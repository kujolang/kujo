# Brainfuck command bytes: diagnostic only, not accepted

Rust and Goose store Brainfuck Array commands as bytes. Kujo currently stores
one-character strings. A diagnostic copy changes the filter to UTF-8 bytes,
stores filtered commands in a byte buffer, and compares integer opcodes. It
retains all commands, tape operations, jump behavior and preparation within each
call. The canonical port and runtime are unchanged.

Twenty controls pass in VM/interpreter/JIT for both representations: 120 complete
states with identical full tape, final position, checksum and command counts.
Controls cover empty input, Unicode comments, byte wrapping, nested/untaken loops,
clamped left movement, repeated output and growth past 30,000 tape cells. The
comma case preserves Kujo's existing no-op behavior; Rust/Goose reject execution
of comma, so this is not a claim of complete comparator equivalence. Comparator
sources are byte-checked at `3c9f84ddaeec834c0631747cb789e8cfd602f993`.

## Short process comparison

Each process executes the official warmup program twenty times. Seven alternating
paired rounds follow one excluded process warmup per runtime/variant. All 48
observations pass their independent checksum oracle; 42 are measured samples.
Ratios are original time / byte-variant time; values above one favor the variant.

| Runtime | Median paired process speedup | Paired range |
| --- | ---: | ---: |
| VM | 1.0076x | 0.9758–1.1599x |
| Interpreter | 1.0420x | 1.0043–1.1046x |
| JIT | 1.0140x | 0.9723–1.0624x |

These short samples do not reach function admission for the Brainfuck kernel.
Separate instrumented runs show five compilation entries, 1,002 native boundaries,
1,002 native function entries and no type-guard failures for each representation;
those aggregate counters alone do not identify which function executed natively.

## Warmed JIT comparison

Separate diagnostic traces with 120 calls confirm that `bf_array` and
`bf_byte_array` respectively reach the 100-call threshold, compile successfully
and retry their current call natively. Their command builders compile from a
native callback. Debug/instrumented runs are excluded from timing scores.

The subsequent uninstrumented experiment performs 120 admission calls followed
by twenty timed calls in each process. Seven alternating pairs preserve both the
full process time and the measured kernel interval from monotonic `time_ns()`.
All fourteen samples pass expected checksums. Median kernel speedup is **1.0267x**
(range **0.7675–1.1365x**); median process speedup is **1.0054x** (range
**0.9212–1.0403x**). Raw outputs, sample membership, frozen identities, invocation
flags and both sets of ratios are independently verified.

## Disposition

This loaded-laptop experiment establishes semantic equivalence on the controls,
but no convincing repeatable VM/JIT performance win. The representation variant
is not integrated. It does not explain or resolve the production timeout, and
these ratios cannot be projected onto that different program. No global language
speedup, competitor result or full-production pass is claimed.

The executable is frozen accepted SHA
`17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325`, not the
worker-reuse candidate. No local compilation overlapped samples; ambient user
load was uncontrolled. `bf-byte-command-experiment-2026-11-03.json.gz` preserves
state, timing, admission traces, source identities and diagnostic scripts. The
next useful performance work needs attribution of the remaining execution costs,
rather than accepting this small and inconsistent change as a solution.
