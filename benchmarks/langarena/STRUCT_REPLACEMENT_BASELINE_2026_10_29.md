# Struct replacement construction-boundary baseline

On accepted runtime `d853be4`, replacing a root dictionary entry with a struct
argument avoids width-dependent copying. Constructing the same struct inside the
helper still incurs that copying. Nested replacements retain both costs on this
accepted baseline; the uncommitted nested candidate is evaluated separately.

The [complete baseline](struct-replacement-baseline-2026-10-29.json) records
64 allocation observations and 96 semantic checks across VM, interpreter and JIT.
It covers root/nested dictionary state, literal/parameter payloads, widths 2/2,048,
initial-only/fresh snapshots, and 16/32 calls. Full original/final contents and
fresh snapshots are asserted. VM/JIT added-call slopes agree.

Requested bytes per added helper call with an initial retained snapshot:

| State | Struct construction | Width 2 | Width 2,048 |
| --- | --- | ---: | ---: |
| Root | Inside helper | 6,147 | 640,407 |
| Root | Caller argument | 5,919 | 5,919 |
| Nested | Inside helper | 8,013 | 642,273 |
| Nested | Caller argument | 8,484 | 642,744 |

Every non-flat row adds 634,260 requested bytes and 2,046 allocation requests per
added call when increasing width from 2 to 2,048. Literal and argument forms have
different setup costs; these are independent width comparisons, not a claim that
moving construction alone makes otherwise identical work faster.

Source inspection explains the admission difference: `Expr::StructInstance`
flushes the private publication journal before constructing fields, while ordinary
parameter reads can retain admission. Its field expressions are individually
evaluated and can call user code, expose an owner or fail. A future fix must retain
those operand boundaries, snapshot/error semantics and fallback allocation gates.
Fresh snapshots may require copying and must not be removed to improve results.

`probes/struct-replacement-isolation.rb` reproduces this characterization. The
report embeds fixture sources, executable/runner identities, independent exact
coverage verification and recalculated slopes. Future comparisons must reuse the
recorded fixture paths because path storage affects allocation totals.

No runtime change is accepted by this baseline. It does not establish elapsed
speed or complete sections 3 and 4.
