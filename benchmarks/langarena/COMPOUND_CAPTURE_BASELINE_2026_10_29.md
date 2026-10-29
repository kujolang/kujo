# Compound indexed updates and scalar captures: baseline

On accepted runtime `2c91c49`, eight direct dictionary assignments using only a
parameter have flat owner-width costs. Adding only a captured scalar read, only
a captured scalar assignment, or indexed arithmetic independently restores
width-dependent copying. These are separate remaining batching limitations.

The [complete observations](compound-capture-baseline-2026-10-29.json) contain
128 allocation observations and 192 passing VM/interpreter/JIT semantic checks.
VM/JIT allocation slopes agree. Cases cross direct/compound assignments, four
counter modes, widths 128/2,048, initial-only/fresh snapshots and 16/32 calls.
Every helper writes eight entries. Full original/final dictionary contents,
saved changed entries and final counter values are asserted.

Requested bytes per added helper call with an initial retained snapshot:

| Assignment | Counter mode | Width 128 | Width 2,048 |
| --- | --- | ---: | ---: |
| Direct | Parameter only | 7,062 | 7,062 |
| Direct | Captured read only | 45,523 | 633,043 |
| Direct | Captured write only | 45,531 | 633,051 |
| Direct | Captured increment and return | 45,531 | 633,051 |
| Compound | Parameter only | 45,524 | 633,044 |
| Compound | Captured read only | 45,523 | 633,043 |
| Compound | Captured write only | 45,531 | 633,051 |
| Compound | Captured increment and return | 45,531 | 633,051 |

Read-only returns the unchanged captured counter. Write-only assigns the parameter
to the captured counter and returns the parameter, so it does not read that counter
in its update or return. Increment-and-return exercises both effects. The compound
form computes `state[j] + 1`; the direct form stores the call's parameter. This
separates effects combined in the earlier collection-factor isolation.

All seven non-control rows add 587,520 requested bytes when width grows from 128
to 2,048. Fresh-snapshot observations remain separate because preserving the old
values requires isolation. This is allocation characterization, not elapsed-time
measurement or fix acceptance. The unaccepted collection-replacement candidate
is not the baseline.

`probes/compound-capture-isolation.rb` reproduces the matrix. The report embeds
fixture sources and executable/runner hashes. Independent verification checks
exact coverage, actual fixture identities, slope arithmetic and VM/JIT agreement.
Future comparisons must reuse identical fixture paths. Callback-capable values,
errors, aliases, cold cases and broader loops still require fix-specific gates.
