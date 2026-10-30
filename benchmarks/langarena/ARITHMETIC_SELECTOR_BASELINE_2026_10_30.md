# Arithmetic selector baseline — 2026-10-30

Runtime baseline: `94ad8f5`, the accepted initial string/byte batch fix.
This characterizes the next confirmed cost while the mixed two-write candidate
is still undergoing verification. It does not report results for that candidate.

[Raw evidence](arithmetic-selector-baseline-2026-10-30.json) embeds the frozen
runtime/probe identities, fixture sources, raw allocation observations, semantic
outputs, independent verifier and separate error/callback contract.

## Results

All 192 allocation observations and 288 semantic checks passed independent
coverage, invocation, source identity and raw-counter verification. The matrix
covers eight forms, widths 8/2,048, an initial snapshot versus a fresh snapshot
per call, and 1/16/32 helper calls. Allocation measurements use VM and JIT;
semantic checks use VM, interpreter and JIT. VM/JIT allocation slopes agree in
every group.

With one initial snapshot, requested bytes per added helper call are:

| Form | Width 8 | Width 2,048 | Width penalty |
| --- | ---: | ---: | ---: |
| Literal indices (control) | 6,514 | 6,514 | 0 |
| Offset plus constant | 7,406 | 284,846 | 277,440 |
| Offset minus constant | 7,406 | 284,846 | 277,440 |
| Nested addition/subtraction | 7,406 | 284,846 | 277,440 |
| Negative offset plus constant | 7,406 | 284,846 | 277,440 |
| Counted loop, offset plus counter | 9,133 | 286,573 | 277,440 |
| Counted loop, offset minus counter | 9,133 | 286,573 | 277,440 |
| Counted loop, negated offset plus counter | 9,133 | 286,573 | 277,440 |

Slopes use the difference between 16 and 32 calls, divided by 16. Fresh-snapshot
observations remain in the evidence and must be compared separately: preserving
those snapshots can require copying. Allocation requests are 72 per added call
for the four direct arithmetic forms and the literal control, and 76 for the
three loop forms, independent of width in this matrix.

The current array batch admission evaluator handles literals, integer parameters,
a counted-loop variable and unary negation, but rejects binary arithmetic index
expressions. The literal control demonstrates that the bounded mixed batch itself
already avoids width growth. Extending admission must preserve checked integer
arithmetic and reject uncertain bounds or observable callbacks; this baseline
alone does not prove a proposed extension safe.

## Semantic baseline and next gate

A separate contract passes in all three runtime modes. It checks ascending and
descending selectors, negative indices, offset loops, saved snapshots, overflow,
bounds errors with earlier writes preserved, and callback visibility. JIT-mode
success is not proof that every operation executes natively.

Before accepting a fix, replay these exact fixtures against matching candidate
artifacts, retain fresh-snapshot controls, add admission/rejection unit tests,
and run the existing runtime and all-50 test-size checksum gates. Overflow,
callback and bounds failures must retain their ordinary evaluation/publication
order. No arithmetic-selector runtime fix is included in this baseline commit.

Local elapsed times are not used for acceptance. These are allocation/scaling
results, not speedups against Goose/Rust or clean-host performance claims.
