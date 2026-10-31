# Constant destination proof limit: measured residual cost

Accepted runtime `1ed1866`, measured after the section 3/4 representation review.
The eight-destination limit has a reachable allocation cost; it is no longer
only an unmeasured fallback concern. No runtime behavior was changed in this
measurement.

Imported helpers repeatedly replace 8, 9 or 16 fixed array positions with mixed
array/dictionary values. Owner widths are 32 and 2,048; iteration counts are 64
and 1,024; call counts are 1 and 8. Every fixture checks the original snapshot,
every final written position and every untouched element. Forty-eight allocation
observations and 72 semantic executions pass. VM and JIT allocation slopes agree;
semantic executions include the interpreter too.

Extra requested bytes per added call at width 2,048 versus 32:

| Destinations | 64 iterations | 1,024 iterations |
| --- | ---: | ---: |
| 8 | 0 | 0 |
| 9 | 39,360 | 165,312 |
| 16 | 81,344 | 165,312 |

The current proof discards its distinct-destination bound when the ninth unique
position is encountered, retaining the ordinary write-count reservation. The
proposed next fix retains a conservative upper bound from the number of constant
write sites when the inline exact union fills. This avoids increasing the inline
limit or allocating a set. Varying selectors must still retain their conservative
bound; raw endpoint, overflow, branch-union and value-semantics checks remain
mandatory. The proposed fix is not yet implemented or accepted.

[Raw evidence](ARRAY_DESTINATION_LIMIT_2026_10_31.json) includes exact fixture
sources, artifact identities, outputs, scripts and the independent verifier.
Slopes subtract the one-call observation from the eight-call observation and
divide by seven. These are allocation results, not elapsed-time speedups or a
claim that a specific production benchmark reaches this pattern.
